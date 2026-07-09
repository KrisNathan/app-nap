use libc::pid_t;

use crate::{
    nap_backend::{NapBackend, systemd_client::SystemdClient},
    systemd::unit::systemd_unit_for_pid,
};
use std::{io, sync::Arc};

const CPU_WEIGHT_DEFAULT: u64 = 100;
const CPU_WEIGHT_UNFOCUSED: u64 = 1;

pub struct SystemdCpuWeightBackend {
    client: Arc<SystemdClient>,
}
impl SystemdCpuWeightBackend {
    pub fn new(client: Arc<SystemdClient>) -> Self {
        Self { client }
    }
}

impl NapBackend for SystemdCpuWeightBackend {
    fn send_stop(&self, pid: pid_t) -> io::Result<()> {
        let scope = systemd_unit_for_pid(pid)?;
        self.client
            .set_property(&scope, "CPUWeight", CPU_WEIGHT_UNFOCUSED)
    }

    fn send_cont(&self, pid: pid_t) -> io::Result<()> {
        let scope = systemd_unit_for_pid(pid)?;
        self.client
            .set_property(&scope, "CPUWeight", CPU_WEIGHT_DEFAULT)
    }
}
