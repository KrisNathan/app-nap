use super::{NapBackend, systemd_client::SystemdClient};
use crate::systemd::unit::systemd_unit_for_pid;
use libc::pid_t;
use std::io;

pub struct SystemdFreezeBackend {
    client: SystemdClient,
}

impl SystemdFreezeBackend {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            client: SystemdClient::new()?,
        })
    }
}

impl NapBackend for SystemdFreezeBackend {
    fn send_stop(&self, pid: pid_t) -> io::Result<()> {
        let unit = systemd_unit_for_pid(pid)?;
        self.client.freeze(&unit)
    }

    fn send_cont(&self, pid: pid_t) -> io::Result<()> {
        let unit = systemd_unit_for_pid(pid)?;
        self.client.thaw(&unit)
    }
}
