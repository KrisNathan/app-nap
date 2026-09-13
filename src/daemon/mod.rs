mod channel_event;
mod event_loop;
pub mod models;
mod policy_router;
mod usage_tracker;

use std::collections::{BTreeSet, HashMap};

use libc::pid_t;

use crate::{
    cgroup::{Cgroup, proc_util::process_comm},
    config::models::{Config, cpu_load_polling::CpuLoadPollingConfig, policies::PoliciesConfig},
    daemon::models::{app_state::AppState, unit_state::UnitState, wake_signals::WakeSignals},
};

pub struct Daemon {
    apps: HashMap<pid_t, AppState>,

    // ngl I realized we don't need this
    // we can just loop through all apps, accumulate policy vote in temporary local HashMap
    units: HashMap<Cgroup, UnitState>,

    wake_signals: WakeSignals,

    policies_config: PoliciesConfig,
    cpu_load_polling_config: CpuLoadPollingConfig,
}

impl Daemon {
    pub fn new(config: &Config) -> Self {
        Self {
            apps: HashMap::new(),
            units: HashMap::new(),
            wake_signals: WakeSignals::default(),
            policies_config: config.policies.clone(),
            cpu_load_polling_config: config.cpu_load_polling.clone(),
        }
    }

    pub async fn window_added(&mut self, window_id: String, pid: pid_t) {
        let comm = process_comm(pid).unwrap(); // pray
        let app_state = self.apps.entry(pid).or_insert(AppState::new(comm, pid));
        app_state
            .window_added(window_id, false, true, &self.wake_signals)
            .await;

        for cgroup in app_state.get_cgroups() {
            let unit = self
                .units
                .entry(cgroup.clone())
                .or_insert(UnitState::new(BTreeSet::new()));
            unit.members.insert(pid);
        }

        self.reconcile_policy().await;
    }

    pub async fn window_removed(&mut self, window_id: String, pid: pid_t) {
        let Some(app_state) = self.apps.get_mut(&pid) else {
            return;
        };
        app_state
            .window_removed(&window_id, &self.wake_signals)
            .await;

        if !app_state.has_windows() {
            // no windows left
            for cgroup in app_state.get_cgroups() {
                let Some(unit) = self.units.get_mut(cgroup) else {
                    continue;
                };
                unit.members.remove(&pid);
                if unit.members.is_empty() {
                    self.units.remove(cgroup);
                }
            }
        }

        self.reconcile_policy().await;
    }

    pub async fn window_minimized_changed(
        &mut self,
        window_id: String,
        pid: pid_t,
        minimized: bool,
    ) {
        let Some(app_state) = self.apps.get_mut(&pid) else {
            return;
        };
        app_state
            .window_minimized_changed(&window_id, minimized, &self.wake_signals)
            .await;

        self.reconcile_policy().await;
    }

    pub async fn window_active_changed(&mut self, window_id: String, pid: pid_t, active: bool) {
        let Some(app_state) = self.apps.get_mut(&pid) else {
            return;
        };
        app_state
            .window_active_changed(&window_id, active, &self.wake_signals)
            .await;

        self.reconcile_policy().await;
    }

    async fn reconcile_policy(&mut self) {}
}
