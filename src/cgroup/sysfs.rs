use std::{fs, io};

use libc::pid_t;

use crate::cgroup::UnitPath;

/// Returns the PIDs of all processes in this cgroup.
/// Utilizes /sys/fs/cgroup to read the cgroup.procs file.
pub fn get_unit_pids(unit_path: &UnitPath) -> io::Result<Vec<pid_t>> {
    let mut pids = Vec::new();
    let mut pending: Vec<std::path::PathBuf> = vec![format!("/sys/fs/cgroup{unit_path}").into()];

    while let Some(path) = pending.pop() {
        pids.extend(
            fs::read_to_string(path.join("cgroup.procs"))?
                .lines()
                .filter_map(|line| line.trim().parse::<pid_t>().ok()),
        );

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                pending.push(entry.path());
            }
        }
    }

    Ok(pids)
}
