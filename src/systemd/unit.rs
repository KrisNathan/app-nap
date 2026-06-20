use libc::pid_t;
use std::io;

use super::cgroup::get_process_cgroup;

pub fn parse_systemd_unit(cgroup_output: &str) -> Option<String> {
    cgroup_output
        .lines()
        .flat_map(|line| line.trim().rsplit('/'))
        .find(|segment| segment.ends_with(".scope") || segment.ends_with(".service"))
        .map(ToOwned::to_owned)
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
    fn parses_service_from_ps_cgroup_output() {
        let output =
            "0::/user.slice/user-1000.slice/user@1000.service/session.slice/pipewire.service\n";

        assert_eq!(
            parse_systemd_unit(output).as_deref(),
            Some("pipewire.service")
        );
    }

    #[test]
    fn returns_none_without_systemd_unit() {
        assert_eq!(parse_systemd_unit("0::/some/plain/cgroup\n"), None);
    }
}
