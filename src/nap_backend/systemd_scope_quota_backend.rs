use super::NapBackend;
use libc::pid_t;
use std::io;
use std::process::Command;

#[derive(Default)]
pub struct SystemdScopeQuotaBackend;

impl NapBackend for SystemdScopeQuotaBackend {
    fn send_stop(&self, pid: pid_t) -> io::Result<()> {
        let scope = systemd_unit_for_pid(pid)?;
        set_cpu_quota(&scope, Some("5%"))
    }

    fn send_cont(&self, pid: pid_t) -> io::Result<()> {
        let scope = systemd_unit_for_pid(pid)?;
        set_cpu_quota(&scope, None)
    }
}

fn systemd_unit_for_pid(pid: pid_t) -> io::Result<String> {
    if pid <= 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "pid must be > 0",
        ));
    }

    let output = Command::new("ps")
        .args(["-o", "cgroup=", "-p", &pid.to_string()])
        .output()?;

    if !output.status.success() {
        return Err(command_failed("ps", output.status.code(), &output.stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_systemd_unit(&stdout).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("no systemd scope or service found for pid {pid}"),
        )
    })
}

fn parse_systemd_unit(cgroup_output: &str) -> Option<String> {
    cgroup_output
        .lines()
        .flat_map(|line| line.trim().rsplit('/'))
        .find(|segment| segment.ends_with(".scope") || segment.ends_with(".service"))
        .map(ToOwned::to_owned)
}

fn set_cpu_quota(unit: &str, quota: Option<&str>) -> io::Result<()> {
    let property = match quota {
        Some(quota) => format!("CPUQuota={quota}"),
        None => "CPUQuota=".to_string(),
    };

    let output = Command::new("systemctl")
        .args(["--user", "set-property", unit, &property])
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        Err(command_failed(
            "systemctl --user set-property",
            output.status.code(),
            &output.stderr,
        ))
    }
}

fn command_failed(command: &str, code: Option<i32>, stderr: &[u8]) -> io::Error {
    let stderr = String::from_utf8_lossy(stderr);
    let message = match code {
        Some(code) => format!("{command} failed with exit code {code}: {}", stderr.trim()),
        None => format!("{command} was terminated by signal: {}", stderr.trim()),
    };
    io::Error::other(message)
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
