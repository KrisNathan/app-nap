use libc::pid_t;
use std::{fs, io};

/// Walk from `pid` up the parent chain, stopping before `systemd` (or pid 1).
///
/// Chromium-based apps (e.g. RPM Brave) often split across two app units: the
/// window pid lives in a self-created `app-org.chromium.Chromium-*.scope`
/// while children remain in the desktop-launch `app-*.service`. Climbing to
/// the user systemd instance collects every process in that launch tree.
pub fn ancestor_pids_until_systemd(pid: pid_t) -> io::Result<Vec<pid_t>> {
    let mut pids = Vec::new();
    let mut current = pid;

    while current > 1 {
        if process_comm(current)? == "systemd" {
            break;
        }
        pids.push(current);
        current = process_ppid(current)?;
    }

    Ok(pids)
}

fn process_ppid(pid: pid_t) -> io::Result<pid_t> {
    let status = fs::read_to_string(format!("/proc/{pid}/status"))?;
    for line in status.lines() {
        if let Some(value) = line.strip_prefix("PPid:") {
            return value
                .trim()
                .parse()
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err));
        }
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("PPid not found for pid {pid}"),
    ))
}

pub fn process_comm(pid: pid_t) -> io::Result<String> {
    let comm = fs::read_to_string(format!("/proc/{pid}/comm"))?;
    Ok(comm.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::{ancestor_pids_until_systemd, process_comm, process_ppid};

    #[test]
    fn reads_ppid_and_comm_for_self() {
        let pid = unsafe { libc::getpid() };
        let ppid = process_ppid(pid).unwrap();
        assert!(ppid > 0);
        assert!(!process_comm(pid).unwrap().is_empty());
    }

    #[test]
    fn ancestor_walk_includes_self_and_stops() {
        let pid = unsafe { libc::getpid() };
        let ancestors = ancestor_pids_until_systemd(pid).unwrap();
        assert_eq!(ancestors.first().copied(), Some(pid));
        assert!(
            ancestors
                .iter()
                .all(|&p| process_comm(p).unwrap() != "systemd")
        );
    }
}
