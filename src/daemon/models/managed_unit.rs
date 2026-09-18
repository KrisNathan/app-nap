use std::collections::BTreeSet;

use libc::pid_t;

use crate::{cgroup::UnitName, daemon::models::policy::Policy};

pub struct ManagedUnit {
    pub name: UnitName,
    pub voters: BTreeSet<pid_t>,
    pub applied_policy: Option<Policy>,
}
impl ManagedUnit {
    pub fn new(name: UnitName, voters: BTreeSet<pid_t>) -> Self {
        Self {
            name,
            voters,
            applied_policy: None,
        }
    }
}
