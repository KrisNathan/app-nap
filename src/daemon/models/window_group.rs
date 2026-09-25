use std::collections::{HashMap, HashSet};

use libc::pid_t;

use crate::{
    cgroup::UnitPath,
    config::models::cpu_load_polling::CpuLoadPollingConfig,
    daemon::models::{
        Load, LoadTracker, cpu_sample::CpuSample, policy::Policy, tier::Tier,
        wake_signals::WakeSignals,
    },
};

pub struct Window {
    pub minimized: bool,
    pub active: bool,
}

pub struct WindowGroup {
    comm: String,
    pid: pid_t,
    /// key: window id
    windows: HashMap<String, Window>,
    unit_paths: HashSet<UnitPath>,
    load_tracker: LoadTracker,
    tier: Tier,
    policy_vote: Policy,
}

impl WindowGroup {
    pub fn new(comm: String, pid: pid_t) -> Self {
        Self {
            comm,
            pid,
            windows: HashMap::new(),
            unit_paths: HashSet::new(),
            load_tracker: LoadTracker::new(),
            tier: Tier::Performance,
            policy_vote: Policy::Performance,
        }
    }

    pub fn has_windows(&self) -> bool {
        !self.windows.is_empty()
    }

    pub fn on_cpu_usage_tick(
        &mut self,
        wake_signals: &WakeSignals,
        new_cpu_sample: CpuSample,
        config: &CpuLoadPollingConfig,
    ) {
        let Some(_) = self.load_tracker.on_cpu_usage_tick(new_cpu_sample, config) else {
            return;
        };

        self.recompute_vote(wake_signals);
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
}

impl WindowGroup {
    pub fn refresh_units(&mut self, unit_paths: HashSet<UnitPath>) {
        if unit_paths == self.unit_paths {
            return;
        }

        self.unit_paths = unit_paths;
        self.load_tracker.clear_baseline();
    }

    /// Updates policy_vote
    pub fn recompute_vote(&mut self, wake_signals: &WakeSignals) {
        let is_any_active = self.windows.values().any(|window| window.active);
        let is_any_unminimized = self.windows.values().any(|window| !window.minimized);
        let is_inhibiting = wake_signals.is_inhibiting(&self.unit_paths);
        let is_playing_media = wake_signals.is_playing_media(&self.unit_paths);

        self.tier = Self::eval_tier(
            is_any_active,
            is_any_unminimized,
            is_inhibiting,
            is_playing_media,
        );

        self.policy_vote = Self::eval_policy(self.tier, self.load_tracker.load());

        // reset last cpu sample if performance
        if self.policy_vote == Policy::Performance {
            self.load_tracker.reset();
        }
    }

    pub fn window_minimized_changed(
        &mut self,
        window_id: &str,
        is_minimized: bool,
        wake_signals: &WakeSignals,
    ) {
        if let Some(window) = self.windows.get_mut(window_id)
            && window.minimized != is_minimized
        {
            window.minimized = is_minimized;
            self.recompute_vote(wake_signals);
        }
    }

    pub fn window_active_changed(
        &mut self,
        window_id: &str,
        is_active: bool,
        wake_signals: &WakeSignals,
    ) {
        if let Some(window) = self.windows.get_mut(window_id)
            && window.active != is_active
        {
            window.active = is_active;
            self.recompute_vote(wake_signals);
        }
    }

    pub fn window_added(
        &mut self,
        window_id: String,
        is_minimized: bool,
        is_active: bool,
        wake_signals: &WakeSignals,
    ) {
        self.windows.insert(
            window_id,
            Window {
                minimized: is_minimized,
                active: is_active,
            },
        );
        self.recompute_vote(wake_signals);
    }

    pub fn window_removed(&mut self, window_id: &str, wake_signals: &WakeSignals) {
        self.windows.remove(window_id);
        if self.has_windows() {
            self.recompute_vote(wake_signals);
        }
    }
}

impl WindowGroup {
    /// Window group is polled if not in "performance" tier
    pub fn is_polled(&self) -> bool {
        self.tier != Tier::Performance
    }

    pub fn unit_paths(&self) -> &HashSet<UnitPath> {
        &self.unit_paths
    }

    pub fn policy_vote(&self) -> Policy {
        self.policy_vote
    }

    pub fn window_pid(&self) -> pid_t {
        self.pid
    }

    pub fn comm(&self) -> &str {
        &self.comm
    }

    pub fn usage(&self) -> f64 {
        self.load_tracker.usage()
    }

    pub fn throttle(&self) -> f64 {
        self.load_tracker.throttle()
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }
}
