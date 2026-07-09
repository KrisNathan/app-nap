use libc::pid_t;
use std::io;

use super::cgroup::{get_process_cgroup, is_app_scope_cgroup, trim_hierarchy_id};
use super::proc::ancestor_pids_until_systemd;

pub fn parse_systemd_unit(cgroup_output: &str) -> Option<String> {
    let path = trim_hierarchy_id(cgroup_output);
    if !is_app_scope_cgroup(path) {
        return None;
    }
    path.rsplit('/').next().map(ToOwned::to_owned)
}

pub fn systemd_unit_for_pid(pid: pid_t) -> io::Result<String> {
    let output = get_process_cgroup(pid)?;

    parse_systemd_unit(&output).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("no systemd scope or service found for pid {pid}"),
        )
    })
}

/// App units for `pid` and its ancestors up to (but not including) systemd.
///
/// Deduped in ancestor order. Needed when an app splits across units (e.g.
/// Brave's Chromium self-scope + desktop-launch service).
pub fn systemd_units_for_pid_tree(pid: pid_t) -> io::Result<Vec<String>> {
    let mut units = Vec::new();

    for ancestor in ancestor_pids_until_systemd(pid)? {
        let Ok(unit) = systemd_unit_for_pid(ancestor) else {
            continue;
        };
        if !units.iter().any(|existing| existing == &unit) {
            units.push(unit);
        }
    }

    if units.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no systemd app unit found in pid tree of {pid}"),
        ));
    }

    Ok(units)
}

#[cfg(test)]
mod tests {
    use super::parse_systemd_unit;

    #[test]
    fn parses_scope_from_ps_cgroup_output() {
        let output = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-flatpak-com.brave.Browser-1760258369.scope\n";

        assert_eq!(
            parse_systemd_unit(output).as_deref(),
            Some("app-flatpak-com.brave.Browser-1760258369.scope")
        );
    }

    #[test]
    fn parses_escaped_service_unit() {
        let output = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-brave\\x2dbrowser@a4b0be57c333446d9befeb10984db717.service\n";

        assert_eq!(
            parse_systemd_unit(output).as_deref(),
            Some("app-brave\\x2dbrowser@a4b0be57c333446d9befeb10984db717.service")
        );
    }

    #[test]
    fn refuses_session_slice_service() {
        let output =
            "0::/user.slice/user-1000.slice/user@1000.service/session.slice/pipewire.service\n";

        assert_eq!(parse_systemd_unit(output), None);
    }

    #[test]
    fn returns_none_without_systemd_unit() {
        assert_eq!(parse_systemd_unit("0::/some/plain/cgroup\n"), None);
    }
}
