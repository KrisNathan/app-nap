use std::collections::HashSet;

use libc::pid_t;
use tokio::sync::oneshot;

use crate::{cgroup::UnitPath, daemon::models::app_snapshot::AppSnapshot};

#[derive(Debug)]
pub enum ChannelEvent {
    WindowAdded {
        window_id: String,
        pid: pid_t,
    },
    WindowRemoved {
        window_id: String,
        pid: pid_t,
    },
    WindowMinimizedChanged {
        window_id: String,
        pid: pid_t,
        minimized: bool,
    },
    WindowActiveChanged {
        window_id: String,
        pid: pid_t,
        active: bool,
    },
    MediaUnitsChanged(HashSet<UnitPath>),
    InhibitedAppsChanged(HashSet<String>),

    ListApps {
        sender: oneshot::Sender<Vec<AppSnapshot>>,
    },
}
