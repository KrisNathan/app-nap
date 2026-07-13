use std::io;

use crate::systemd::cgroup;

pub fn stop(cgroups: &[String]) -> io::Result<()> {
    send_signal(cgroups, libc::SIGSTOP, "SIGSTOP")
}

pub fn cont(cgroups: &[String]) -> io::Result<()> {
    send_signal(cgroups, libc::SIGCONT, "SIGCONT")
}

fn send_signal(cgroups: &[String], signal: libc::c_int, signal_name: &str) -> io::Result<()> {
    let pids = cgroup::get_pids_from_cgroups(cgroups).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("failed to resolve PIDs for signal={signal_name} cgroups={cgroups:?}: {err}"),
        )
    })?;

    for pid in pids {
        // SAFETY: `libc::kill` is called with a numeric PID enumerated from the
        // cgroup and a valid signal constant. ESRCH (process gone) is tolerated.
        if unsafe { libc::kill(pid, signal) } == -1 {
            let err = io::Error::last_os_error();
            if err.raw_os_error() != Some(libc::ESRCH) {
                return Err(io::Error::new(
                    err.kind(),
                    format!("failed to send signal={signal_name} pid={pid}: {err}"),
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::stop;

    #[test]
    fn reports_invalid_cgroup_as_failure() {
        assert!(stop(&["/not-an-app-cgroup".to_string()]).is_err());
    }
}
