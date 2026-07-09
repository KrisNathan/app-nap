use libc::pid_t;

use crate::{
    nap_backend::{NapBackend, systemd_client::SystemdClient},
    systemd::unit::systemd_units_for_pid_tree,
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

    fn set_cpu_weight(&self, pid: pid_t, weight: u64) -> io::Result<()> {
        for unit in systemd_units_for_pid_tree(pid)? {
            println!("{unit} CPUWeight = {weight}");
            self.client.set_property(&unit, "CPUWeight", weight)?;
        }
        Ok(())
    }
}

impl NapBackend for SystemdCpuWeightBackend {
    fn send_stop(&self, pid: pid_t) -> io::Result<()> {
        self.set_cpu_weight(pid, CPU_WEIGHT_UNFOCUSED)
    }

    fn send_cont(&self, pid: pid_t) -> io::Result<()> {
        self.set_cpu_weight(pid, CPU_WEIGHT_DEFAULT)
    }
}
