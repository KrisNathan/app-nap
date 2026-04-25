use libc::pid_t;
use std::io;

pub trait SignalController: Send + Sync {
    fn send_stop(&self, pid: pid_t) -> io::Result<()>;
    fn send_cont(&self, pid: pid_t) -> io::Result<()>;
}
