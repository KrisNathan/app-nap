use libc::pid_t;

pub enum ChannelEvent {
    AddWindow {
        window_id: String,
        pid: pid_t,
    },
    RemoveWindow {
        window_id: String,
        pid: pid_t,
    },
    MinimizeChanged {
        window_id: String,
        pid: pid_t,
        minimized: bool,
    },
    ActiveChanged {
        window_id: String,
        pid: pid_t,
        active: bool,
    },
    UsageWatchTick,
}
