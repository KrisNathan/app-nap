use libc::pid_t;
use serde::Serialize;
use zbus::zvariant::Type;

use crate::daemon::models::{app_state::AppState, policy::Policy};

#[derive(Debug, Serialize, Type)]
pub struct AppSnapshot {
    pub window_pid: pid_t,
    pub comm: String,
    pub policy: Policy,
    pub usage: f64,
    pub throttle: f64,
    pub window_count: usize,
}

impl From<&AppState> for AppSnapshot {
    fn from(app: &AppState) -> Self {
        Self {
            window_pid: app.get_window_pid(),
            comm: app.get_comm().into(),
            policy: app.get_voted_policy(),
            usage: app.get_usage(),
            throttle: app.get_throttle(),
            window_count: app.get_window_count(),
        }
    }
}
