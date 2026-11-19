use std::collections::HashSet;

use libc::pid_t;
use std::io;

use crate::cgroup::{
    UnitPath,
    procfs::{ancestor_pids_until_systemd, cgroup_path_from_pid},
};

pub fn related_units(pid: pid_t) -> io::Result<HashSet<UnitPath>> {
    let pids = ancestor_pids_until_systemd(pid)?;

    let mut unit_paths: HashSet<UnitPath> = HashSet::new();
    for pid in pids {
        let Ok(cg) = cgroup_path_from_pid(pid) else {
            continue;
        };

        let Some(unit_path) = UnitPath::from_cgroup_path(cg) else {
            continue;
        };

        unit_paths.insert(unit_path);
    }

    Ok(unit_paths)
}
