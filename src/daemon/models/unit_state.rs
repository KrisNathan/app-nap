use std::collections::BTreeSet;

use libc::pid_t;

use crate::{cgroup::UnitName, daemon::models::policy::Policy};

pub struct UnitState {
    pub name: UnitName,
    pub members: BTreeSet<pid_t>,
    pub applied_policy: Option<Policy>,
}
impl UnitState {
    pub fn new(name: UnitName, members: BTreeSet<pid_t>) -> Self {
        Self {
            name,
            members,
            applied_policy: None,
        }
    }
}
