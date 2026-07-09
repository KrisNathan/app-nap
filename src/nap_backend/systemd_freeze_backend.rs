use super::{NapBackend, systemd_client::SystemdClient};
use crate::systemd::unit::systemd_units_for_pid_tree;
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
        for unit in systemd_units_for_pid_tree(pid)? {
            self.client.freeze(&unit)?;
        }
        Ok(())
    }

    fn send_cont(&self, pid: pid_t) -> io::Result<()> {
        for unit in systemd_units_for_pid_tree(pid)? {
            self.client.thaw(&unit)?;
        }
        Ok(())
    }
}
