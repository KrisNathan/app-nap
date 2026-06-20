use libc::pid_t;
use std::{fs, io};

pub fn get_process_cgroup(pid: pid_t) -> io::Result<String> {
    let cgroup = fs::read_to_string(format!("/proc/{pid}/cgroup"))?;
    Ok(cgroup)
}

#[cfg(test)]
mod tests {
    use super::get_process_cgroup;
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
}
