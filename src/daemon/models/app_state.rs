use std::collections::{HashMap, HashSet};

use crate::{
    cgroup::Cgroup,
    config::models::{cpu_load_polling::CpuLoadPollingConfig, cpu_sample::CpuSample},
    daemon::models::{policy::Policy, tier::Tier, wake_signals::WakeSignals},
};

#[derive(Clone, Copy, PartialEq)]
pub enum Load {
    Busy,
    Idle,
}

pub struct Window {
    pub minimized: bool,
    pub active: bool,
}

pub struct AppState {
    comm: String,
    /// key: window id
    windows: HashMap<String, Window>,
    cgroups: HashSet<Cgroup>,

    load: Load,
    queued_load: Load,
    ticks: u64,

    usage: f64,
    throttle: f64,

    last_cpu_sample: Option<CpuSample>,

    tier: Tier,
    voted_policy: Policy,
}

impl AppState {
    pub fn new(comm: String) -> Self {
        Self {
            comm,
            windows: HashMap::new(),
            cgroups: HashSet::new(),
            load: Load::Busy,
            queued_load: Load::Busy,
            ticks: 0,
            usage: 0.0,
            throttle: 0.0,
            last_cpu_sample: None,
            tier: Tier::Performance,
            voted_policy: Policy::Performance,
        }
    }

    pub fn has_windows(&self) -> bool {
        !self.windows.is_empty()
    }

    pub async fn on_cpu_usage_tick(
        &mut self,
        wake_signals: &WakeSignals<'_>,
        new_cpu_sample: CpuSample,
        config: &CpuLoadPollingConfig,
    ) {
        let Some(last_cpu_sample) = &self.last_cpu_sample else {
            self.last_cpu_sample = Some(new_cpu_sample);
            return;
        };

        let Some((usage, throttle)) = new_cpu_sample.delta_since(last_cpu_sample) else {
            return;
        };

        self.last_cpu_sample = Some(new_cpu_sample);

        self.usage = usage;
        self.throttle = throttle;

        let Some(candidate) = Self::eval_next_load(usage, throttle, config) else {
            return;
        };

        if candidate != self.queued_load {
            self.queued_load = candidate;
            self.ticks = 1;
        } else {
            self.ticks += 1;
        }

        let confirm_tick = match self.queued_load {
            Load::Idle => config.idle.confirm_ticks,
            Load::Busy => config.busy.confirm_ticks,
        };

        if self.ticks >= confirm_tick {
            self.ticks = 0;
            if self.load != self.queued_load {
                self.load = self.queued_load;
                self.reconcile_policy(wake_signals).await;
            }
        }
    }

    fn eval_next_load(usage: f64, throttle: f64, config: &CpuLoadPollingConfig) -> Option<Load> {
        if usage < config.idle.usage_thres && throttle < config.idle.throttle_thres {
            Some(Load::Idle)
        } else if usage > config.busy.usage_thres || throttle > config.busy.throttle_thres {
            Some(Load::Busy)
        } else {
            None
        }
    }

    fn eval_tier(
        is_any_active: bool,
        is_any_unminimized: bool,
        is_inhibiting: bool,
        is_playing_media: bool,
    ) -> Tier {
        if is_any_active || is_inhibiting {
            Tier::Performance
        } else if is_any_unminimized || is_playing_media {
            // yes this is intentional
            // I only intended Background media to be throttled slightly
            Tier::Background
        } else {
            Tier::Nap
        }
    }

    fn eval_policy(tier: Tier, load: Load) -> Policy {
        match (tier, load) {
            (Tier::Performance, _) => Policy::Performance,
            (Tier::Background, Load::Idle) => Policy::BackgroundIdle,
            (Tier::Background, Load::Busy) => Policy::BackgroundBusy,
            (Tier::Nap, Load::Idle) => Policy::NapIdle,
            (Tier::Nap, Load::Busy) => Policy::NapBusy,
        }
    }

    pub async fn reconcile_policy(&mut self, wake_signals: &WakeSignals<'_>) {
        let is_any_active = self.windows.values().any(|window| window.active);
        let is_any_unminimized = self.windows.values().any(|window| !window.minimized);
        let is_inhibiting = wake_signals.is_inhibiting(&self.cgroups);
        let is_playing_media = wake_signals.is_playing_media(&self.cgroups);

        self.tier = Self::eval_tier(
            is_any_active,
            is_any_unminimized,
            is_inhibiting,
            is_playing_media,
        );

        self.voted_policy = Self::eval_policy(self.tier, self.load);
    }

    pub async fn window_minimized_changed(
        &mut self,
        window_id: &str,
        is_minimized: bool,
        wake_signals: &WakeSignals<'_>,
    ) {
        if let Some(window) = self.windows.get_mut(window_id)
            && window.minimized != is_minimized
        {
            window.minimized = is_minimized;
            self.reconcile_policy(wake_signals).await;
        }
    }

    pub async fn window_active_changed(
        &mut self,
        window_id: &str,
        is_active: bool,
        wake_signals: &WakeSignals<'_>,
    ) {
        if let Some(window) = self.windows.get_mut(window_id)
            && window.active != is_active
        {
            window.active = is_active;
            self.reconcile_policy(wake_signals).await;
        }
    }

    pub async fn add_window(
        &mut self,
        window_id: &str,
        is_minimized: bool,
        is_active: bool,
        wake_signals: &WakeSignals<'_>,
    ) {
        self.windows.insert(
            window_id.to_string(),
            Window {
                minimized: is_minimized,
                active: is_active,
            },
        );
        self.reconcile_policy(wake_signals).await;
    }

    pub async fn remove_window(&mut self, window_id: &str, wake_signals: &WakeSignals<'_>) {
        self.windows.remove(window_id);
        if self.has_windows() {
            self.reconcile_policy(wake_signals).await;
        }
    }
}
