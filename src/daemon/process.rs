use libc::pid_t;
use std::{collections::HashMap, fs, io};

#[derive(Clone, Copy, Debug)]
pub struct WindowState {
    pub minimized: bool,
}

#[derive(Debug)]
pub struct Process {
    pub windows: HashMap<String, WindowState>,
    pub is_napping: bool,
}

impl Process {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            is_napping: false,
        }
    }
}

// Reads `/proc/<pid>/cgroup`, which contains the systemd unit path. The
// inhibitor `who` string (e.g. "com.obsproject.Studio" for Flatpak OBS,
// "firefox" for native Firefox) appears as a substring of the unit name
// (e.g. `app-flatpak-com.obsproject.Studio-<id>.scope`), so we match by
// substring rather than against `/proc/<pid>/comm` (which is truncated to
// 15 chars and doesn't carry the app ID for Flatpaks).
pub fn read_process_cgroup(pid: pid_t) -> io::Result<String> {
    let cgroup = fs::read_to_string(format!("/proc/{pid}/cgroup"))?;
    Ok(cgroup)
}

#[cfg(test)]
mod tests {
    use super::read_process_cgroup;
    use std::io;

    #[test]
    fn reads_cgroup_for_current_process() {
        let cgroup = read_process_cgroup(unsafe { libc::getpid() }).unwrap();
        assert!(!cgroup.is_empty());
    }

    #[test]
    fn returns_error_for_invalid_pid() {
        let err = read_process_cgroup(-1).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}
