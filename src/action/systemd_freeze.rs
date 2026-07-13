use std::sync::Arc;

use crate::{action::Action, systemd::dbus_client::SystemdDbusClient};

pub struct SystemdFreezeAction {
    client: Arc<SystemdDbusClient>,
}

impl SystemdFreezeAction {
    pub fn new(client: Arc<SystemdDbusClient>) -> Self {
        Self { client }
    }
}

impl Action for SystemdFreezeAction {
    fn apply(&self, cgroups: &[String]) {
        for cgroup in cgroups {
            self.client.freeze(cgroup).ok();
        }
    }

    fn revert(&self, cgroups: &[String]) {
        for cgroup in cgroups {
            self.client.thaw(cgroup).ok();
        }
    }
}
