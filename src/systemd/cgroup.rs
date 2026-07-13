use libc::pid_t;
use std::{fs, io};

use super::proc::ancestor_pids_until_systemd;

pub fn get_process_cgroup(pid: pid_t) -> io::Result<String> {
    let cgroup = fs::read_to_string(format!("/proc/{pid}/cgroup"))?;
    Ok(cgroup)
}

pub fn trim_hierarchy_id(cgroup: &str) -> &str {
    cgroup.trim().split(':').next_back().unwrap_or("")
}

/// It is an "app" cgroup if:
/// - starts with "app-"
/// - ends with ".scope" or ".service"
/// - in app.slice/
pub fn is_app_scope_cgroup(trimmed_cgroup: &str) -> bool {
    let last = trimmed_cgroup.rsplit('/').next().unwrap_or("");
    last.starts_with("app-")
        && (last.ends_with(".scope") || last.ends_with(".service"))
        && trimmed_cgroup.contains("/app.slice/")
}

/// Collect distinct app cgroups in `pid`'s process tree.
/// Trimmed, compatible with systemd cgroup naming.
///
/// Walks ancestors until systemd and returns each unique trimmed app-scope
/// path. Covers Chromium-style splits where the window pid and its children
/// live in different units.
pub fn get_related_cgroups(pid: pid_t) -> io::Result<Vec<String>> {
    let mut related = Vec::new();

    for ancestor in ancestor_pids_until_systemd(pid)? {
        let Ok(cgroup) = get_process_cgroup(ancestor) else {
            continue;
        };
        let trimmed = trim_hierarchy_id(&cgroup).to_owned();
        if !is_app_scope_cgroup(&trimmed) || related.iter().any(|seen| seen == &trimmed) {
            continue;
        }
        related.push(trimmed);
    }

    if related.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no related app cgroups found for pid tree of {pid}"),
        ));
    }

    Ok(related)
}

/// Resolve every PID currently in the given app cgroups.
///
/// Unions `cgroup.procs` from each path. Membership is live and should not be
/// cached on app state — re-read when reconciling media playback.
pub fn get_pids_from_cgroups(cgroups: &[String]) -> io::Result<Vec<pid_t>> {
    let mut related = Vec::new();

    for cgroup in cgroups {
        let Ok(pids) = get_pids_from_cgroup(cgroup) else {
            continue;
        };
        for p in pids {
            if !related.contains(&p) {
                related.push(p);
            }
        }
    }

    if related.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no pids found in provided app cgroups",
        ));
    }

    Ok(related)
}

/// Get PIDs in a cgroup.
/// ONLY for "app" cgroup.
pub fn get_pids_from_cgroup(cgroup: &str) -> io::Result<Vec<pid_t>> {
    let trimmed_cgroup = trim_hierarchy_id(cgroup);

    if !is_app_scope_cgroup(trimmed_cgroup) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("refusing to enumerate non-app cgroup: {trimmed_cgroup}"),
        ));
    }

    let procs_path = format!("/sys/fs/cgroup{trimmed_cgroup}/cgroup.procs");

    Ok(fs::read_to_string(procs_path)?
        .lines()
        .map(String::from)
        .filter_map(|line| line.trim().parse::<pid_t>().ok())
        .collect())
}

#[cfg(test)]
mod tests {
    use crate::systemd::cgroup::trim_hierarchy_id;

    use super::{get_pids_from_cgroup, get_process_cgroup};
    use std::io;

    #[test]
    fn reads_cgroup_for_current_process() {
        let cgroup = get_process_cgroup(unsafe { libc::getpid() }).unwrap();
        assert!(!cgroup.is_empty());
    }

    #[test]
    fn returns_error_for_invalid_pid() {
        let err = get_process_cgroup(-1).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn trims_hierarchy_id() {
        let cgroup = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-flatpak-com.brave.Browser-1760258369.scope\n";
        assert_eq!(
            trim_hierarchy_id(cgroup),
            "/user.slice/user-1000.slice/user@1000.service/app.slice/app-flatpak-com.brave.Browser-1760258369.scope"
        );
    }

    #[test]
    fn refuses_root_cgroup() {
        let err = get_pids_from_cgroup("0::/\n").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn refuses_user_session_cgroup() {
        let cgroup = "0::/user.slice/user-1000.slice/user@1000.service\n";
        let err = get_pids_from_cgroup(cgroup).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn refuses_system_slice() {
        let cgroup = "0::/system.slice/sshd.service\n";
        let err = get_pids_from_cgroup(cgroup).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn refuses_app_unit_outside_app_slice() {
        // An app-* unit that is not under app.slice is not nap-eligible.
        let cgroup = "0::/user.slice/user-1000.slice/user@1000.service/app-flatpak-com.brave.Browser-1.scope\n";
        let err = get_pids_from_cgroup(cgroup).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn refuses_non_app_unit_under_app_slice() {
        // A non-app-* unit under app.slice is not nap-eligible.
        let cgroup =
            "0::/user.slice/user-1000.slice/user@1000.service/app.slice/pipewire.service\n";
        let err = get_pids_from_cgroup(cgroup).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn refuses_plain_cgroup() {
        let cgroup = "0::/some/plain/cgroup\n";
        let err = get_pids_from_cgroup(cgroup).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }
}
