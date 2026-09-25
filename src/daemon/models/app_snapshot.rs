use libc::pid_t;
use serde::Serialize;
use zbus::zvariant::Type;

use crate::daemon::models::{policy::Policy, window_group::WindowGroup};

#[derive(Debug, Serialize, Type)]
pub struct AppSnapshot {
    pub window_pid: pid_t,
    pub comm: String,
    pub policy: Policy,
    pub usage: f64,
    pub throttle: f64,
    pub window_count: usize,
}

impl From<&WindowGroup> for AppSnapshot {
    fn from(group: &WindowGroup) -> Self {
        Self {
            window_pid: group.window_pid(),
            comm: group.comm().into(),
            policy: group.policy_vote(),
            usage: group.usage(),
            throttle: group.throttle(),
            window_count: group.window_count(),
        }
    }
}
