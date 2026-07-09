use super::{NapBackend, systemd_client::SystemdClient};
use crate::systemd::unit::systemd_units_for_pid_tree;
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

    fn set_cpu_quota(&self, pid: pid_t, quota: u64) -> io::Result<()> {
        for unit in systemd_units_for_pid_tree(pid)? {
            self.client
                .set_property(&unit, "CPUQuotaPerSecUSec", quota)?;
        }
        Ok(())
    }
}

impl NapBackend for SystemdCPUQuotaBackend {
    fn send_stop(&self, pid: pid_t) -> io::Result<()> {
        self.set_cpu_quota(pid, CPU_QUOTA_5_PERCENT)
    }

    fn send_cont(&self, pid: pid_t) -> io::Result<()> {
        self.set_cpu_quota(pid, CPU_QUOTA_UNSET)
    }
}
