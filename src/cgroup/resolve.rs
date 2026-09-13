use std::collections::HashSet;

use libc::pid_t;
use std::io;

use crate::cgroup::{Cgroup, proc_util::ancestor_pids_until_systemd};

pub fn related_units(pid: pid_t) -> io::Result<HashSet<Cgroup>> {
    let pids = ancestor_pids_until_systemd(pid)?;

    let mut cgroups: HashSet<Cgroup> = HashSet::new();
    for pid in pids {
        let Ok(cg) = Cgroup::from_pid(pid) else {
            continue;
        };

        cgroups.insert(cg);
    }

    Ok(cgroups)
}
