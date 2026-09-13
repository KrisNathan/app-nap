use std::collections::BTreeSet;

use libc::pid_t;

use crate::daemon::models::policy::Policy;

pub struct UnitState {
    members: BTreeSet<pid_t>,
    applied_policy: Option<Policy>,
}
