use super::{
    NapBackend,
    systemd_unit::{command_failed, systemd_unit_for_pid},
};
use libc::pid_t;
use std::{io, process::Command};

#[derive(Default)]
pub struct SystemdFreezeBackend;

impl NapBackend for SystemdFreezeBackend {
    fn send_stop(&self, pid: pid_t) -> io::Result<()> {
        let unit = systemd_unit_for_pid(pid)?;
        run_systemctl("freeze", &unit)
    }

    fn send_cont(&self, pid: pid_t) -> io::Result<()> {
        let unit = systemd_unit_for_pid(pid)?;
        run_systemctl("thaw", &unit)
    }
}

fn run_systemctl(verb: &str, unit: &str) -> io::Result<()> {
    let output = Command::new("systemctl")
        .args(["--user", verb, unit])
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        Err(command_failed(
            &format!("systemctl --user {verb}"),
            output.status.code(),
            &output.stderr,
        ))
    }
}
