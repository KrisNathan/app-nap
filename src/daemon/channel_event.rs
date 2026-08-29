use libc::pid_t;
use tokio::sync::oneshot;

use crate::daemon::snapshot::AppSnapshot;

pub enum ChannelEvent {
    AddWindow {
        window_id: String,
        pid: pid_t,
    },
    RemoveWindow {
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
    /// Read-only query: the daemon answers with a snapshot of every app it
    /// tracks.
    ListApps {
        reply: oneshot::Sender<Vec<AppSnapshot>>,
    },
}
