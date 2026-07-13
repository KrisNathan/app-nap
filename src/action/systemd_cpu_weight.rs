use std::sync::Arc;

use crate::{action::Action, systemd::dbus_client::SystemdDbusClient};

const CPU_WEIGHT_DEFAULT: u64 = 100;

pub struct SystemdCpuWeightAction {
    client: Arc<SystemdDbusClient>,
    weight: u64,
}

impl SystemdCpuWeightAction {
    pub fn new(client: Arc<SystemdDbusClient>, weight: u64) -> Self {
        Self { client, weight }
    }
}

impl Action for SystemdCpuWeightAction {
    fn apply(&self, cgroups: &[String]) {
        for cgroup in cgroups {
            self.client
                .set_property(cgroup, "CPUWeight", self.weight)
                .ok();
        }
    }

    fn revert(&self, cgroups: &[String]) {
        for cgroup in cgroups {
            self.client
                .set_property(cgroup, "CPUWeight", CPU_WEIGHT_DEFAULT)
                .ok();
        }
    }
}
