pub mod channel_event;
mod event_loop;
pub mod models;
mod policy_router;

pub use event_loop::EventLoop;
use log::warn;

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    io,
};

use libc::pid_t;

use crate::{
    cgroup::{
        UnitName, UnitPath, cpu_stat::get_cpu_stat, procfs::process_comm, resolve::related_units,
    },
    config::models::{Config, cpu_load_polling::CpuLoadPollingConfig},
    daemon::{
        models::{
            app_snapshot::AppSnapshot, cpu_sample::CpuSample, managed_unit::ManagedUnit,
            wake_signals::WakeSignals, window_group::WindowGroup,
        },
        policy_router::PolicyRouter,
    },
};

pub struct Daemon {
    window_groups: HashMap<pid_t, WindowGroup>,

    // this is needed because reverting a policy change requires us to know which unit to revert from
    units: HashMap<UnitPath, ManagedUnit>,

    wake_signals: WakeSignals,

    cpu_load_polling_config: CpuLoadPollingConfig,

    policy_router: PolicyRouter,
}

impl Daemon {
    pub async fn new(config: &Config, conn: &zbus::Connection) -> zbus::Result<Self> {
        Ok(Self {
            window_groups: HashMap::new(),
            units: HashMap::new(),
            wake_signals: WakeSignals::default(),
            cpu_load_polling_config: config.cpu_load_polling.clone(),
            policy_router: PolicyRouter::from_config(&config.policies, conn).await?,
        })
    }

    async fn leave_unit(&mut self, unit_path: &UnitPath, pid: pid_t) {
        let Some(unit) = self.units.get_mut(unit_path) else {
            return;
        };

        unit.voters.remove(&pid);

        if !unit.voters.is_empty() {
            return;
        }

        // unit is empty, remove from units

        // attempt to revert applied policy (if it fails we dont care)
        if let Some(applied) = unit.applied_policy
            && let Err(error) = self
                .policy_router
                .revert(applied, unit_path, &unit.name)
                .await
        {
            warn!(
                "failed to revert {applied:?} for unit {}: {error}",
                unit.name
            );
        }

        self.units.remove(unit_path);
    }

    /// Returns affected unit paths.
    async fn link_group_to_units(&mut self, pid: pid_t) -> HashSet<UnitPath> {
        let Some(group) = self.window_groups.get_mut(&pid) else {
            return HashSet::new();
        };

        let old_unit_paths = group.get_unit_paths().clone();

        let new_units = match related_units(pid) {
            Ok(units) => units,
            Err(e) => {
                warn!("failed to resolve units for pid {}: {e}", pid);
                return old_unit_paths;
            }
        };

        group.refresh_units(new_units);

        let new_unit_paths = group.get_unit_paths().clone();

        if old_unit_paths == new_unit_paths {
            return old_unit_paths;
        }

        let added = new_unit_paths
            .difference(&old_unit_paths)
            .map(|unit_path| (unit_path.clone(), UnitName::from(unit_path)))
            .collect::<Vec<_>>();

        for unit_path in old_unit_paths.difference(&new_unit_paths) {
            self.leave_unit(unit_path, pid).await;
        }

        for (unit_path, unit_name) in added {
            self.units
                .entry(unit_path)
                .or_insert(ManagedUnit::new(unit_name, BTreeSet::new()))
                .voters
                .insert(pid);
        }

        old_unit_paths.union(&new_unit_paths).cloned().collect()
    }

    pub async fn window_added(
        &mut self,
        window_id: String,
        pid: pid_t,
        minimized: bool,
        active: bool,
    ) {
        let comm = match process_comm(pid) {
            Ok(comm) => comm,
            Err(e) => {
                warn!("failed to get comm for pid {pid}: {e}");
                String::new()
            }
        };

        self.window_groups
            .entry(pid)
            .or_insert(WindowGroup::new(comm, pid));

        let affected = self.link_group_to_units(pid).await;

        if let Some(group) = self.window_groups.get_mut(&pid) {
            group.window_added(window_id, minimized, active, &self.wake_signals);
        }

        self.reconcile_policy(&affected).await;
    }

    pub async fn window_removed(&mut self, window_id: String, pid: pid_t) {
        let Some(group) = self.window_groups.get_mut(&pid) else {
            return;
        };
        group.window_removed(&window_id, &self.wake_signals);

        let unit_paths = group.get_unit_paths().clone();
        if group.has_windows() {
            self.reconcile_policy(&unit_paths).await;
            return;
        }

        // no windows left, remove cgroups
        self.window_groups.remove(&pid);
        for unit_path in &unit_paths {
            self.leave_unit(unit_path, pid).await;
        }
        self.reconcile_policy(&unit_paths).await;
    }

    pub async fn window_minimized_changed(
        &mut self,
        window_id: String,
        pid: pid_t,
        minimized: bool,
    ) {
        let Some(group) = self.window_groups.get_mut(&pid) else {
            return;
        };
        group.window_minimized_changed(&window_id, minimized, &self.wake_signals);
        let affected = group.get_unit_paths().clone();
        self.reconcile_policy(&affected).await;
    }

    pub async fn window_active_changed(&mut self, window_id: String, pid: pid_t, active: bool) {
        let Some(group) = self.window_groups.get_mut(&pid) else {
            return;
        };
        group.window_active_changed(&window_id, active, &self.wake_signals);
        let affected = group.get_unit_paths().clone();
        self.reconcile_policy(&affected).await;
    }

    async fn reconcile_policy(&mut self, keys: &HashSet<UnitPath>) {
        for key in keys {
            let Some(unit) = self.units.get_mut(key) else {
                continue;
            };

            let vote = unit
                .voters
                .iter()
                .filter_map(|pid| self.window_groups.get(pid))
                .map(|group| group.get_policy_vote())
                .max();

            let Some(new_policy) = vote else {
                continue;
            };

            let old_policy = unit.applied_policy;
            if old_policy == Some(new_policy) {
                continue;
            }

            if let Some(old_policy) = old_policy
                && let Err(error) = self.policy_router.revert(old_policy, key, &unit.name).await
            {
                warn!(
                    "failed to revert {old_policy:?} for unit {}: {error}",
                    unit.name
                );
                continue;
            }

            match self.policy_router.apply(new_policy, key, &unit.name).await {
                Ok(()) => unit.applied_policy = Some(new_policy),
                Err(error) => {
                    warn!(
                        "failed to apply {new_policy:?} to unit {}: {error}",
                        unit.name
                    );
                    unit.applied_policy = None;
                }
            }
        }
    }

    pub async fn inhibited_apps_changed(&mut self, apps: HashSet<String>) {
        self.wake_signals.powerdevil_apps = apps;
        self.recompute_wake_votes().await;
    }

    pub async fn media_units_changed(&mut self, unit_paths: HashSet<UnitPath>) {
        self.wake_signals.mpris_units = unit_paths;
        self.recompute_wake_votes().await;
    }

    /// A wake signal moved; recompute every window group's vote and reconcile
    /// the units whose policy actually changed.
    async fn recompute_wake_votes(&mut self) {
        let mut affected: HashSet<UnitPath> = HashSet::new();

        for group in self.window_groups.values_mut() {
            let before = group.get_policy_vote();
            group.recompute_vote(&self.wake_signals);
            if group.get_policy_vote() != before {
                affected.extend(group.get_unit_paths().iter().cloned());
            }
        }

        self.reconcile_policy(&affected).await;
    }

    pub fn has_polled_groups(&self) -> bool {
        self.window_groups.values().any(|group| group.is_polled())
    }

    pub async fn cpu_load_tick(&mut self) {
        let mut affected_unit_paths: HashSet<UnitPath> = HashSet::new();

        for group in self.window_groups.values_mut().filter(|g| g.is_polled()) {
            affected_unit_paths.extend(group.get_unit_paths().iter().cloned());

            let Ok(sample) = sample_cpu(group.get_unit_paths()) else {
                continue;
            };

            group
                .on_cpu_usage_tick(&self.wake_signals, sample, &self.cpu_load_polling_config)
                .await;
        }

        self.reconcile_policy(&affected_unit_paths).await;
    }
}

fn sample_cpu(unit_paths: &HashSet<UnitPath>) -> io::Result<CpuSample> {
    let mut usage_usec: u64 = 0;
    let mut throttle_usecs: HashMap<UnitPath, u64> = HashMap::new();
    // cpu.stat is subtree-inclusive, so a unit nested in another unit in
    // this set is counted twice.
    for unit_path in unit_paths {
        let stat = get_cpu_stat(unit_path)?;
        usage_usec += stat.usage_usec;
        throttle_usecs.insert(unit_path.clone(), stat.throttled_usec);
    }

    Ok(CpuSample::now(usage_usec, throttle_usecs))
}

impl Daemon {
    pub fn list_apps(&self) -> Vec<AppSnapshot> {
        self.window_groups.values().map(AppSnapshot::from).collect()
    }
}
