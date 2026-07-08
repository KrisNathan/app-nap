use super::{NapBackend, systemd_client::SystemdClient};
use crate::systemd::unit::systemd_unit_for_pid;
use libc::pid_t;
use std::io;

// systemd exposes CPU quota over D-Bus as `CPUQuotaPerSecUSec` (u64 microseconds
// per second), not the `CPUQuota=5%` string syntax that `systemctl set-property`
// accepts. `systemctl` parses the percent form and converts it before sending.
// We do the conversion ourselves: 1 second = 1_000_000 µs, so 5% = 50_000 µs.
const CPU_QUOTA_5_PERCENT: u64 = 50_000;

// systemd treats `u64::MAX` as "infinity" (no quota), matching the empty
// `CPUQuota=` form used by `systemctl set-property` to clear the limit.
const CPU_QUOTA_UNSET: u64 = u64::MAX;

pub struct SystemdCPUQuotaBackend {
    client: SystemdClient,
}

impl SystemdCPUQuotaBackend {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            client: SystemdClient::new()?,
        })
    }
}

impl NapBackend for SystemdCPUQuotaBackend {
    fn send_stop(&self, pid: pid_t) -> io::Result<()> {
        let scope = systemd_unit_for_pid(pid)?;
        self.client
            .set_property(&scope, "CPUQuotaPerSecUSec", CPU_QUOTA_5_PERCENT)
    }

    fn send_cont(&self, pid: pid_t) -> io::Result<()> {
        let scope = systemd_unit_for_pid(pid)?;
        self.client
            .set_property(&scope, "CPUQuotaPerSecUSec", CPU_QUOTA_UNSET)
    }
}
