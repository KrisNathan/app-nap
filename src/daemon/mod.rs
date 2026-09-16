pub mod channel_event;
mod event_loop;
pub mod models;
mod policy_router;
mod usage_tracker;

pub use event_loop::EventLoop;
use tokio::sync::oneshot;

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    io,
};

use libc::pid_t;

use crate::{
    cgroup::{Cgroup, proc_util::process_comm},
    config::models::{Config, cpu_load_polling::CpuLoadPollingConfig, policies::PoliciesConfig},
    daemon::{
        models::{
            app_snapshot::AppSnapshot,
            app_state::{AppState, CgroupRefreshResult},
            cpu_sample::CpuSample,
            unit_state::UnitState,
            wake_signals::WakeSignals,
        },
        policy_router::PolicyRouter,
    },
};

pub struct Daemon {
    apps: HashMap<pid_t, AppState>,

    // this is needed because reverting a policy change requires us to know which unit to revert from
    units: HashMap<Cgroup, UnitState>,

    wake_signals: WakeSignals,

    policies_config: PoliciesConfig,
    cpu_load_polling_config: CpuLoadPollingConfig,

    policy_router: PolicyRouter,
}

impl Daemon {
    pub async fn new(config: &Config, conn: &zbus::Connection) -> Self {
        Self {
            apps: HashMap::new(),
            units: HashMap::new(),
            wake_signals: WakeSignals::default(),
            policies_config: config.policies.clone(),
            cpu_load_polling_config: config.cpu_load_polling.clone(),
            policy_router: PolicyRouter::from_config(&config.policies, conn).await,
        }
    }

    async fn leave_unit(&mut self, cgroup: &Cgroup, pid: pid_t) {
        let Some(unit) = self.units.get_mut(cgroup) else {
            return;
        };

        unit.members.remove(&pid);

        if !unit.members.is_empty() {
            return;
        }

        // unit is empty, remove from units

        if let Some(applied) = unit.applied_policy {
            self.policy_router.revert(applied, cgroup).await;
        }

        self.units.remove(cgroup);
    }

    /// Returns affected cgroups.
    async fn sync_membership(&mut self, pid: pid_t) -> HashSet<Cgroup> {
        let Some(app) = self.apps.get_mut(&pid) else {
            return HashSet::new();
        };

        let old = app.get_cgroups().clone();

        match app.refresh_cgroups() {
            CgroupRefreshResult::NoChange => return old,
            CgroupRefreshResult::Changed => {}
        }

        let new = app.get_cgroups().clone();

        for cgroup in old.difference(&new) {
            self.leave_unit(cgroup, pid).await;
        }

        for cgroup in new.difference(&old) {
            self.units
                .entry(cgroup.clone())
                .or_insert(UnitState::new(BTreeSet::new()))
                .members
                .insert(pid);
        }

        old.union(&new).cloned().collect()
    }

    pub async fn window_added(&mut self, window_id: String, pid: pid_t) {
        let comm = process_comm(pid).unwrap(); // pray
        self.apps.entry(pid).or_insert(AppState::new(comm, pid));

        let affected = self.sync_membership(pid).await;

        if let Some(app) = self.apps.get_mut(&pid) {
            app.window_added(window_id, false, true, &self.wake_signals);
        }

        self.reconcile_policy(&affected).await;
    }

    pub async fn window_removed(&mut self, window_id: String, pid: pid_t) {
        let Some(app) = self.apps.get_mut(&pid) else {
            return;
        };
        app.window_removed(&window_id, &self.wake_signals);

        let cgroups = app.get_cgroups().clone();
        if app.has_windows() {
            self.reconcile_policy(&cgroups).await;
            return;
        }

        // no windows left, remove cgroups
        self.apps.remove(&pid);
        for cgroup in &cgroups {
            self.leave_unit(cgroup, pid).await;
        }
        self.reconcile_policy(&cgroups).await;
    }

    pub async fn window_minimized_changed(
        &mut self,
        window_id: String,
        pid: pid_t,
        minimized: bool,
    ) {
        let Some(app) = self.apps.get_mut(&pid) else {
            return;
        };
        app.window_minimized_changed(&window_id, minimized, &self.wake_signals);
        let affected = app.get_cgroups().clone();
        self.reconcile_policy(&affected).await;
    }

    pub async fn window_active_changed(&mut self, window_id: String, pid: pid_t, active: bool) {
        let Some(app_state) = self.apps.get_mut(&pid) else {
            return;
        };
        app_state.window_active_changed(&window_id, active, &self.wake_signals);
        let affected = app_state.get_cgroups().clone();
        self.reconcile_policy(&affected).await;
    }

    async fn reconcile_policy(&mut self, keys: &HashSet<Cgroup>) {
        for key in keys {
            let Some(unit) = self.units.get_mut(key) else {
                continue;
            };

            let vote = unit
                .members
                .iter()
                .filter_map(|pid| self.apps.get(pid))
                .map(|app| app.get_voted_policy())
                .max();

            let Some(new_policy) = vote else {
                continue;
            };

            if let Some(old_policy) = unit.applied_policy {
                if old_policy == new_policy {
                    continue;
                }
                self.policy_router.revert(old_policy, key).await;
            }

            self.policy_router.apply(new_policy, key).await;
            unit.applied_policy = Some(new_policy);
        }
    }

    pub async fn inhibited_apps_changed(&mut self, apps: HashSet<String>) {
        self.wake_signals.powerdevil_apps = apps;
        self.recompute_wake_votes().await;
    }

    pub async fn media_units_changed(&mut self, cgroups: HashSet<Cgroup>) {
        self.wake_signals.mpris_units = cgroups;
        self.recompute_wake_votes().await;
    }

    /// A wake signal moved; recompute every app's vote and reconcile the
    /// units whose policy actually changed.
    async fn recompute_wake_votes(&mut self) {
        let mut affected: HashSet<Cgroup> = HashSet::new();

        for app in self.apps.values_mut() {
            let before = app.get_voted_policy();
            app.recompute_vote(&self.wake_signals);
            if app.get_voted_policy() != before {
                affected.extend(app.get_cgroups().iter().cloned());
            }
        }

        self.reconcile_policy(&affected).await;
    }

    pub fn has_polled_apps(&self) -> bool {
        self.apps.values().any(|app| app.is_polled())
    }

    pub async fn cpu_load_tick(&mut self) {
        let mut affected_cgroups: HashSet<Cgroup> = HashSet::new();

        for app in self.apps.values_mut().filter(|a| a.is_polled()) {
            let cgroups = app.get_cgroups();
            affected_cgroups.extend(cgroups.iter().cloned());

            let Ok(sample) = sample_cpu(cgroups) else {
                continue;
            };

            app.on_cpu_usage_tick(&self.wake_signals, sample, &self.cpu_load_polling_config)
                .await;
        }

        self.reconcile_policy(&affected_cgroups).await;
    }
}

fn sample_cpu(cgroups: &HashSet<Cgroup>) -> io::Result<CpuSample> {
    let mut usage_usec: u64 = 0;
    let mut throttle_usecs: HashMap<Cgroup, u64> = HashMap::new();
    for cgroup in cgroups {
        let stat = cgroup.get_cpu_stat()?;
        usage_usec += stat.usage_usec;
        throttle_usecs.insert(cgroup.clone(), stat.throttled_usec);
    }

    Ok(CpuSample::now(usage_usec, throttle_usecs))
}

impl Daemon {
    pub fn list_apps(&self) -> Vec<AppSnapshot> {
        self.apps
            .values()
            .map(|app| AppSnapshot::from(app))
            .collect()
    }
}
