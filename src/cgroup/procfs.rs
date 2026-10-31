use std::{fs, io};

use libc::pid_t;

use crate::cgroup::CgroupPath;

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

pub fn cgroup_path_from_pid(pid: pid_t) -> io::Result<CgroupPath> {
    let contents = fs::read_to_string(format!("/proc/{pid}/cgroup"))?;
    CgroupPath::from_cgroup_full(contents.as_str()).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("not a cgroup v2 line: {}", contents.trim()),
        )
    })
}
