use std::collections::HashSet;

use libc::pid_t;

use crate::cgroup::Cgroup;

#[derive(Debug, Clone)]
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
    MediaUnitsChanged(HashSet<Cgroup>),
    InhibitedAppsChanged(HashSet<String>),
}
