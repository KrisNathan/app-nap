use super::SignalController;
use libc::{SIGCONT, SIGSTOP, pid_t};
use std::io;

#[derive(Default)]
pub struct SystemSignalController;

impl SignalController for SystemSignalController {
    fn send_stop(&self, pid: pid_t) -> io::Result<()> {
        send_signal(pid, SIGSTOP)
    }

    fn send_cont(&self, pid: pid_t) -> io::Result<()> {
        send_signal(pid, SIGCONT)
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
