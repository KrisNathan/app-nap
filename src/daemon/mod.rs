pub mod models;
mod usage_tracker;

use std::collections::{HashMap, HashSet};

use libc::pid_t;
use tokio::sync::mpsc;

use crate::{
    cgroup::Cgroup,
    config::models::{cpu_load_polling::CpuLoadPollingConfig, policies::PoliciesConfig},
    daemon::models::{app_state::AppState, unit_state::UnitState},
};

pub struct Daemon {
    apps: HashMap<pid_t, AppState>,
    units: HashMap<Cgroup, UnitState>,

    /// cgroup unit contains these app names
    powerdevil_apps: HashSet<String>,

    mpris_units: HashSet<Cgroup>,

    policies_config: PoliciesConfig,
    cpu_load_polling_config: CpuLoadPollingConfig,
}

// TODO: separate file
enum ChannelEvent {
    WindowAdded {
        window_id: String,
        pid: pid_t,
    },
    WindowRemoved {
        window_id: String,
        pid: pid_t,
    },
    MinimizedChanged {
        window_id: String,
        pid: pid_t,
        minimized: bool,
    },
    ActiveChanged {
        window_id: String,
        pid: pid_t,
        active: bool,
    },
    MediaUnitsChanged(HashSet<Cgroup>),
    InhibitedAppsChanged(HashSet<String>),
}

pub struct DaemonController {
    daemon: Daemon,
    rx: mpsc::Receiver<ChannelEvent>,
    cpu_tick: tokio::time::Interval,
}
