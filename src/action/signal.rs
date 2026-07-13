use crate::systemd::cgroup;

pub fn stop(cgroups: &[String]) {
    send_signal(cgroups, libc::SIGSTOP);
}

pub fn cont(cgroups: &[String]) {
    send_signal(cgroups, libc::SIGCONT);
}

fn send_signal(cgroups: &[String], signal: libc::c_int) {
    let Ok(pids) = cgroup::get_pids_from_cgroups(cgroups) else {
        return;
    };
    for pid in pids {
        unsafe { libc::kill(pid, signal) };
    }
}
