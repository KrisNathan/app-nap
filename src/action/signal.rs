use crate::{action::Action, systemd::cgroup};

pub struct SignalAction {}

impl Action for SignalAction {
    fn apply(&self, cgroups: &[String]) {
        send_signal(cgroups, libc::SIGSTOP);
    }

    fn revert(&self, cgroups: &[String]) {
        send_signal(cgroups, libc::SIGCONT);
    }
}

fn send_signal(cgroups: &[String], signal: libc::c_int) {
    let Ok(pids) = cgroup::get_pids_from_cgroups(cgroups) else {
        return;
    };
    for pid in pids {
        unsafe { libc::kill(pid, signal) };
    }
}
