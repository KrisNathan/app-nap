use super::{
    NapBackend,
    systemd_unit::{command_failed, systemd_unit_for_pid},
};
use libc::pid_t;
use std::{io, process::Command};

#[derive(Default)]
pub struct SystemdScopeQuotaBackend;

impl NapBackend for SystemdScopeQuotaBackend {
    fn send_stop(&self, pid: pid_t) -> io::Result<()> {
        let scope = systemd_unit_for_pid(pid)?;
        set_cpu_quota(&scope, Some("5%"))
    }

    fn send_cont(&self, pid: pid_t) -> io::Result<()> {
        let scope = systemd_unit_for_pid(pid)?;
        set_cpu_quota(&scope, None)
    }
}

fn set_cpu_quota(unit: &str, quota: Option<&str>) -> io::Result<()> {
    let property = match quota {
        Some(quota) => format!("CPUQuota={quota}"),
        None => "CPUQuota=".to_string(),
    };

    let output = Command::new("systemctl")
        .args(["--user", "set-property", unit, &property])
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        Err(command_failed(
            "systemctl --user set-property",
            output.status.code(),
            &output.stderr,
        ))
    }
}
