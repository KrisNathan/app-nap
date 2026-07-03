use crate::systemd::cgroup::get_related_pids;

use super::NapBackend;
use libc::{SIGCONT, SIGSTOP, pid_t};
use std::io;

#[derive(Default)]
pub struct SystemSignalController;

impl NapBackend for SystemSignalController {
    fn send_stop(&self, pid: pid_t) -> io::Result<()> {
        let pids = get_related_pids(pid)?;

        for p in pids {
            send_signal(p, SIGSTOP)?;
        }

        Ok(())
    }

    fn send_cont(&self, pid: pid_t) -> io::Result<()> {
        let pids = get_related_pids(pid)?;

        for p in pids {
            send_signal(p, SIGCONT)?;
        }

        Ok(())
    }
}

fn send_signal(pid: pid_t, signal: i32) -> io::Result<()> {
    // SAFETY: libc::kill is called with a numeric PID and valid signal constant.
    let rc = unsafe { libc::kill(pid, signal) };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
