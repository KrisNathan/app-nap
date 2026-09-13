use std::collections::BTreeSet;

use libc::pid_t;

use crate::daemon::models::policy::Policy;

pub struct UnitState {
    pub members: BTreeSet<pid_t>,
    pub applied_policy: Option<Policy>,
}
impl UnitState {
    pub fn new(members: BTreeSet<pid_t>) -> Self {
        Self {
            members,
            applied_policy: None,
        }
    }
}
