use std::{collections::HashMap, time::Duration};

use libc::pid_t;
use log::{debug, warn};
use tokio::{sync::mpsc::Receiver, time::MissedTickBehavior};

use crate::{
    config::model::CpuWatchConfig,
    daemon::{
        app_state::{AppState, Tier, WindowState},
        channel_event::ChannelEvent,
        tier_policy_set::TierPolicySet,
    },
    inhibit::InhibitService,
    media::MediaService,
    systemd::{cgroup, cpu_stat},
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
    cpu_watch: CpuWatchConfig,
}

fn resolve_next_tier(
    has_active_window: bool,
    has_unminimized_window: bool,
    inhibited: bool,
    media_playing: bool,
) -> Tier {
    if has_active_window || inhibited {
        Tier::Performance
    } else if has_unminimized_window || media_playing {
        Tier::Background
    } else {
        Tier::Nap
    }
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
        cpu_watch: CpuWatchConfig,
    ) -> Self {
        Self {
            app_states: HashMap::new(),
            tier_policies,
            inhibit_service,
            media_service,
            cpu_watch,
        }
    }

    pub async fn init(&mut self, mut rx: Receiver<ChannelEvent>) {
        let mut ticker =
            tokio::time::interval(Duration::from_millis(self.cpu_watch.interval_ms.max(1)));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                event = rx.recv() => {
                    let Some(event) = event else { return };
                    self.handle_event(event).await;
                }
                // The timer only ticks while the watch list is non-empty;
                // otherwise the daemon is fully event-driven.
                _ = ticker.tick(), if self.has_watched_apps() => {
                    self.usage_watch_tick().await;
                }
            }
        }
    }

    async fn handle_event(&mut self, event: ChannelEvent) {
        match event {
            ChannelEvent::AddWindow { window_id, pid } => self.add_window(&window_id, pid).await,
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
        }
    }

    /// The watch list is derived from desired tier: background/nap apps are
    /// sampled, performance apps are not.
    fn has_watched_apps(&self) -> bool {
        self.app_states
            .values()
            .any(|app| matches!(app.tier, Tier::Background | Tier::Nap))
    }

    async fn reconcile_state(&mut self, pid: pid_t) {
        let Some(app_state) = self.app_states.get_mut(&pid) else {
            return;
        };

        // App units can re-split (e.g. Chromium); refresh paths on reconcile.
        // A changed set invalidates the sample baseline.
        match cgroup::get_related_cgroups(pid) {
            Ok(cgroups) if cgroups != app_state.cgroups => {
                debug!("refreshed cgroups for pid={pid}: {cgroups:?}");
                app_state.cgroups = cgroups;
                app_state.load_tracker.clear_baseline();
            }
            Ok(_) => {}
            Err(err) => warn!("failed to refresh cgroups for pid={pid}: {err}"),
        }

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

        // Focus or a hard keep-awake (inhibitor) means performance. MPRIS only
        // blocks nap; it never forces performance or clears the load state.
        let next_tier = resolve_next_tier(
            has_active_window,
            has_unminimized_window,
            inhibited,
            media_playing,
        );

        if app_state.tier != next_tier {
            debug!("pid={pid} tier {:?} -> {next_tier:?}", app_state.tier);
        }
        if next_tier == Tier::Performance {
            app_state.load_tracker.reset();
        }
        app_state.tier = next_tier;

        self.apply_effective_policy(pid).await;
    }

    /// Revert/apply only when the effective (tier, load) policy changes.
    async fn apply_effective_policy(&mut self, pid: pid_t) {
        let Some(app_state) = self.app_states.get(&pid) else {
            return;
        };
        let Some(next) = self.tier_policies.resolve(app_state.tier, app_state.load()) else {
            return;
        };
        if app_state.applied == Some(next) {
            return;
        }
        let current = app_state.applied;
        let cgroups = app_state.cgroups.clone();

        if let Some(current) = current
            && let Err(err) = self.tier_policies.revert(current, &cgroups).await
        {
            warn!("failed to revert policy={current:?} for pid={pid}: {err}");
            return;
        }
        if let Some(app_state) = self.app_states.get_mut(&pid) {
            app_state.applied = None;
        }

        match self.tier_policies.apply(next, &cgroups).await {
            Ok(()) => {
                debug!("pid={pid} applied policy={next:?}");
                if let Some(app_state) = self.app_states.get_mut(&pid) {
                    app_state.applied = Some(next);
                }
            }
            Err(err) => warn!("failed to apply policy={next:?} for pid={pid}: {err}"),
        }
    }

    async fn drop_app_state(&mut self, pid: pid_t) {
        let Some(app_state) = self.app_states.get(&pid) else {
            return;
        };

        if let Some(current) = app_state.applied {
            let cgroups = app_state.cgroups.clone();
            if let Err(err) = self.tier_policies.revert(current, &cgroups).await {
                warn!("failed to revert policy={current:?} while removing pid={pid}: {err}");
            }
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
        let watched: Vec<pid_t> = self
            .app_states
            .iter()
            .filter(|(_, app)| matches!(app.tier, Tier::Background | Tier::Nap))
            .map(|(pid, _)| *pid)
            .collect();

        for pid in watched {
            let flipped = {
                let Some(app_state) = self.app_states.get_mut(&pid) else {
                    continue;
                };
                let sample = match cpu_stat::sample_cgroups(&app_state.cgroups) {
                    Ok(sample) => sample,
                    Err(err) => {
                        warn!("failed to sample cpu.stat for pid={pid}: {err}");
                        continue;
                    }
                };
                let flipped = app_state.load_tracker.observe(sample, &self.cpu_watch);
                debug!(
                    "pid={pid} tier={:?} load={:?} usage={:.3} throttle={:.3} queued={:?} ttl={}",
                    app_state.tier,
                    app_state.load_tracker.load,
                    app_state.load_tracker.usage,
                    app_state.load_tracker.throttle,
                    app_state.load_tracker.queued_load,
                    app_state.load_tracker.ttl,
                );
                flipped
            };
            if let Some(load) = flipped {
                debug!("pid={pid} load -> {load:?}");
                self.apply_effective_policy(pid).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_window_or_inhibitor_yields_performance() {
        assert_eq!(
            resolve_next_tier(true, false, false, false),
            Tier::Performance
        );
        assert_eq!(
            resolve_next_tier(false, false, true, false),
            Tier::Performance
        );
    }

    #[test]
    fn unminimized_window_or_media_yields_background() {
        assert_eq!(
            resolve_next_tier(false, true, false, false),
            Tier::Background
        );
        assert_eq!(
            resolve_next_tier(false, false, false, true),
            Tier::Background
        );
    }

    #[test]
    fn minimized_inactive_with_no_media_yields_nap() {
        assert_eq!(
            resolve_next_tier(false, false, false, false),
            Tier::Nap
        );
    }

    #[test]
    fn active_overrides_media_and_inactive_windows() {
        assert_eq!(
            resolve_next_tier(true, false, false, true),
            Tier::Performance
        );
        assert_eq!(
            resolve_next_tier(true, true, false, false),
            Tier::Performance
        );
    }

    #[test]
    fn inhibitor_overrides_media_and_unminimized() {
        assert_eq!(
            resolve_next_tier(false, true, true, true),
            Tier::Performance
        );
    }

}
