use crate::systemd::cgroup;
use log::warn;

pub fn stop(cgroups: &[String]) {
    send_signal(cgroups, libc::SIGSTOP, "SIGSTOP");
}

pub fn cont(cgroups: &[String]) {
    send_signal(cgroups, libc::SIGCONT, "SIGCONT");
}

fn send_signal(cgroups: &[String], signal: libc::c_int, signal_name: &str) {
    let pids = match cgroup::get_pids_from_cgroups(cgroups) {
        Ok(pids) => pids,
        Err(err) => {
            warn!("failed to resolve PIDs for signal={signal_name} cgroups={cgroups:?}: {err}");
            return;
        }
    };

    for pid in pids {
        if unsafe { libc::kill(pid, signal) } == -1 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() != Some(libc::ESRCH) {
                warn!("failed to send signal={signal_name} pid={pid}: {err}");
            }
        }
    }
}
