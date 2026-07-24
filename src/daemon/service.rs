use std::collections::HashMap;

use libc::pid_t;
use log::warn;
use tokio::sync::mpsc::Receiver;

use crate::{
    daemon::{
        app_state::{AppState, Tier, WindowState},
        channel_event::ChannelEvent,
        tier_policy_set::TierPolicySet,
    },
    inhibit::InhibitService,
    media::MediaService,
    systemd::cgroup,
};

pub struct Daemon<I, M>
where
    I: InhibitService,
    M: MediaService,
{
    app_states: HashMap<pid_t, AppState>, // key: app pid provided by kwin
    tier_policies: TierPolicySet,
    inhibit_service: I,
    media_service: M,
    rx: Receiver<ChannelEvent>,
}

impl<I, M> Daemon<I, M>
where
    I: InhibitService,
    M: MediaService,
{
    pub fn new(
        tier_policies: TierPolicySet,
        inhibit_service: I,
        media_service: M,
        rx: Receiver<ChannelEvent>,
    ) -> Self {
        Self {
            app_states: HashMap::new(),
            tier_policies,
            inhibit_service,
            media_service,
            rx,
        }
    }

    pub async fn init(&mut self) {
        loop {
            if let Some(event) = self.rx.recv().await {
                match event {
                    ChannelEvent::AddWindow { window_id, pid } => {
                        self.add_window(&window_id, pid).await
                    }
                    ChannelEvent::RemoveWindow { window_id, pid } => {
                        self.remove_window(&window_id, pid).await
                    }
                    ChannelEvent::MinimizeChanged {
                        window_id,
                        pid,
                        minimized,
                    } => {
                        self.window_minimize_changed(&window_id, pid, minimized)
                            .await
                    }
                    ChannelEvent::ActiveChanged {
                        window_id,
                        pid,
                        active,
                    } => self.window_active_changed(&window_id, pid, active).await,
                    ChannelEvent::UsageWatchTick => {
                        self.usage_watch_tick().await;
                    }
                }
            }
        }
    }

    async fn reconcile_state(&mut self, pid: pid_t) {
        let Some(app_state) = self.app_states.get(&pid) else {
            return;
        };

        let inhibited = match self.inhibit_service.is_inhibiting(&app_state.cgroups).await {
            Ok(v) => v,
            Err(err) => {
                warn!("failed to check inhibitors for pid={pid}: {err}");
                false
            }
        };
        let media_playing = match self.media_service.is_playing(&app_state.cgroups).await {
            Ok(v) => v,
            Err(err) => {
                warn!("failed to check media playback for pid={pid}: {err}");
                false
            }
        };
        let has_active_window = app_state.windows.values().any(|window| window.active);
        let has_unminimized_window = app_state.windows.values().any(|window| !window.minimized);
        let current_tier = app_state.tier;

        let keep_awake = media_playing || inhibited;

        let next_tier = if has_active_window {
            Tier::Performance
        } else if has_unminimized_window {
            Tier::Background
        } else if !keep_awake {
            Tier::Nap
        } else {
            Tier::Background
        };

        if current_tier == next_tier {
            return;
        }

        let Some(app_state) = self.app_states.get_mut(&pid) else {
            return;
        };

        // Leave `tier` as the last successfully applied value on failure so the
        // next reconcile retries the same transition instead of no-op'ing on Unknown.
        if let Err(err) = self.tier_policies.revert(current_tier, app_state).await {
            warn!("failed to revert tier={current_tier:?} for pid={pid}: {err}");
            return;
        }
        if let Err(err) = self.tier_policies.apply(next_tier, app_state).await {
            warn!("failed to apply tier={next_tier:?} for pid={pid}: {err}");
        }
    }

    async fn drop_app_state(&mut self, pid: pid_t) {
        let Some(app_state) = self.app_states.get_mut(&pid) else {
            return;
        };

        let current_tier = app_state.tier;
        if let Err(err) = self.tier_policies.revert(current_tier, app_state).await {
            warn!("failed to revert tier={current_tier:?} while removing pid={pid}: {err}");
        }
        self.app_states.remove(&pid);
    }

    pub async fn add_window(&mut self, window_id: &str, pid: pid_t) {
        let cgroups = match cgroup::get_related_cgroups(pid) {
            Ok(cgroups) => cgroups,
            Err(err) => {
                warn!("failed to resolve cgroups for pid={pid}: {err}");
                return;
            }
        };
        let app_state = self
            .app_states
            .entry(pid)
            .or_insert_with(|| AppState::new(cgroups));
        app_state
            .windows
            .entry(window_id.to_string())
            .or_insert(WindowState {
                minimized: false,
                active: true,
            });

        self.reconcile_state(pid).await;
    }

    pub async fn remove_window(&mut self, window_id: &str, pid: pid_t) {
        let Some(app_state) = self.app_states.get_mut(&pid) else {
            warn!("remove window for unknown pid: window_id={window_id} pid={pid}");
            return;
        };

        app_state.windows.remove(window_id);

        if app_state.windows.is_empty() {
            self.drop_app_state(pid).await;
            return;
        }

        self.reconcile_state(pid).await
    }

    pub async fn window_minimize_changed(&mut self, window_id: &str, pid: pid_t, minimized: bool) {
        let Some(app_state) = self.app_states.get_mut(&pid) else {
            warn!(
                "minimize changed for unknown pid: window_id={window_id} pid={pid} minimized={minimized}"
            );
            return;
        };

        let Some(window) = app_state.windows.get_mut(window_id) else {
            warn!(
                "minimize changed for unknown window: window_id={window_id} pid={pid} minimized={minimized}"
            );
            return;
        };

        window.minimized = minimized;
        self.reconcile_state(pid).await
    }

    pub async fn window_active_changed(&mut self, window_id: &str, pid: pid_t, active: bool) {
        let Some(app_state) = self.app_states.get_mut(&pid) else {
            warn!(
                "active changed for unknown pid: window_id={window_id} pid={pid} active={active}"
            );
            return;
        };
        let Some(window) = app_state.windows.get_mut(window_id) else {
            warn!(
                "active changed for unknown window: window_id={window_id} pid={pid} active={active}"
            );
            return;
        };

        window.active = active;
        self.reconcile_state(pid).await
    }

    async fn usage_watch_tick(&mut self) {
        // TODO
    }
}
