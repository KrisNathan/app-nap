use std::sync::Arc;

use crate::{action::Action, systemd::dbus_client::SystemdDbusClient};

const CPU_QUOTA_UNSET: u64 = u64::MAX;

pub struct SystemdCpuQuotaAction {
    client: Arc<SystemdDbusClient>,
    quota: u64,
}

impl SystemdCpuQuotaAction {
    pub fn new(client: Arc<SystemdDbusClient>, quota: u64) -> Self {
        Self { client, quota }
    }
}

pub fn cpu_quota_from_percent(percent: u32) -> u64 {
    u64::from(percent) * 10_000
}

impl Action for SystemdCpuQuotaAction {
    fn apply(&self, cgroups: &[String]) {
        for cgroup in cgroups {
            self.client
                .set_property(cgroup, "CPUQuotaPerSecUSec", self.quota)
                .ok();
        }
    }

    fn revert(&self, cgroups: &[String]) {
        for cgroup in cgroups {
            self.client
                .set_property(cgroup, "CPUQuotaPerSecUSec", CPU_QUOTA_UNSET)
                .ok();
        }
    }
}
