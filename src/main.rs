mod nap_backend;

use libc::pid_t;
use nap_backend::{NapBackend, SystemSignalController};
use std::{collections::HashMap, error::Error, future::pending, sync::Arc};
use zbus::{connection, fdo, interface};

#[derive(Clone, Copy, Debug)]
struct WindowState {
    minimized: bool,
}

#[derive(Debug)]
struct Process {
    windows: HashMap<String, WindowState>,
    is_napping: bool,
}

impl Process {
    fn new() -> Self {
        Self {
            windows: HashMap::new(),
            is_napping: false,
        }
    }
}

struct Daemon {
    processes: HashMap<pid_t, Process>,
    nap_backend: Arc<dyn NapBackend>,
}

impl Daemon {
    fn new(nap_backend: Arc<dyn NapBackend>) -> Self {
        Self {
            processes: HashMap::new(),
            nap_backend,
        }
    }

    fn validate_input(window_id: &str, pid: pid_t) -> fdo::Result<()> {
        if window_id.is_empty() {
            return Err(fdo::Error::InvalidArgs(
                "window_id must not be empty".into(),
            ));
        }
        if pid <= 0 {
            return Err(fdo::Error::InvalidArgs("pid must be > 0".into()));
        }
        Ok(())
    }

    fn reconcile_pid(&mut self, pid: pid_t) -> fdo::Result<()> {
        let media_playing = Self::is_media_playing(pid);

        let Some(process) = self.processes.get_mut(&pid) else {
            return Ok(());
        };

        let has_active_window = process.windows.values().any(|window| !window.minimized);

        if has_active_window {
            if process.is_napping {
                self.nap_backend.send_cont(pid).map_err(|err| {
                    fdo::Error::Failed(format!("failed to SIGCONT pid {pid}: {err}"))
                })?;
                process.is_napping = false;
            }
            return Ok(());
        }

        if !process.is_napping && !media_playing {
            self.nap_backend
                .send_stop(pid)
                .map_err(|err| fdo::Error::Failed(format!("failed to SIGSTOP pid {pid}: {err}")))?;
            process.is_napping = true;
        }

        Ok(())
    }

    fn is_media_playing(_pid: pid_t) -> bool {
        false
    }
}

#[interface(name = "dev.appnap.AppNap1")]
impl Daemon {
    fn add_window(&mut self, window_id: &str, pid: pid_t) -> fdo::Result<()> {
        Self::validate_input(window_id, pid)?;

        let process = self.processes.entry(pid).or_insert_with(Process::new);
        process
            .windows
            .entry(window_id.to_string())
            .or_insert(WindowState { minimized: false });

        self.reconcile_pid(pid)
    }

    fn remove_window(&mut self, window_id: &str, pid: pid_t) -> fdo::Result<()> {
        Self::validate_input(window_id, pid)?;

        let mut should_remove_process = false;
        if let Some(process) = self.processes.get_mut(&pid) {
            process.windows.remove(window_id);
            should_remove_process = process.windows.is_empty();
        }
        if should_remove_process {
            self.processes.remove(&pid);
            return Ok(());
        }

        self.reconcile_pid(pid)
    }

    fn minimized_changed(
        &mut self,
        window_id: &str,
        pid: pid_t,
        minimized: bool,
    ) -> fdo::Result<()> {
        Self::validate_input(window_id, pid)?;

        let process = self
            .processes
            .get_mut(&pid)
            .ok_or_else(|| fdo::Error::Failed(format!("unknown pid {pid}")))?;
        let window = process.windows.get_mut(window_id).ok_or_else(|| {
            fdo::Error::Failed(format!("unknown window {window_id} for pid {pid}"))
        })?;

        window.minimized = minimized;
        self.reconcile_pid(pid)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let daemon = Daemon::new(Arc::new(SystemSignalController));
    let _conn = connection::Builder::session()?
        .name("dev.appnap.AppNap")?
        .serve_at("/dev/appnap/AppNap", daemon)?
        .build()
        .await?;

    pending::<()>().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io, sync::Mutex};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum SignalAction {
        Stop(pid_t),
        Cont(pid_t),
    }

    #[derive(Default)]
    struct MockNapBackend {
        actions: Mutex<Vec<SignalAction>>,
    }

    impl MockNapBackend {
        fn actions(&self) -> Vec<SignalAction> {
            self.actions.lock().expect("lock poisoned").clone()
        }
    }

    impl NapBackend for MockNapBackend {
        fn send_stop(&self, pid: pid_t) -> io::Result<()> {
            self.actions
                .lock()
                .expect("lock poisoned")
                .push(SignalAction::Stop(pid));
            Ok(())
        }

        fn send_cont(&self, pid: pid_t) -> io::Result<()> {
            self.actions
                .lock()
                .expect("lock poisoned")
                .push(SignalAction::Cont(pid));
            Ok(())
        }
    }

    fn daemon_with_mock() -> (Daemon, Arc<MockNapBackend>) {
        let mock = Arc::new(MockNapBackend::default());
        let daemon = Daemon::new(mock.clone());
        (daemon, mock)
    }

    #[test]
    fn active_window_prevents_stop() {
        let (mut daemon, mock) = daemon_with_mock();
        let pid = 4242;

        daemon
            .add_window("w1", pid)
            .expect("add_window should succeed");
        daemon
            .minimized_changed("w1", pid, false)
            .expect("minimized_changed should succeed");

        assert_eq!(mock.actions(), vec![]);
    }

    #[test]
    fn all_windows_inactive_stops_once() {
        let (mut daemon, mock) = daemon_with_mock();
        let pid = 4242;

        daemon
            .add_window("w1", pid)
            .expect("add_window should succeed");
        daemon
            .add_window("w2", pid)
            .expect("add_window should succeed");
        daemon
            .minimized_changed("w1", pid, true)
            .expect("minimized_changed should succeed");
        daemon
            .minimized_changed("w2", pid, true)
            .expect("minimized_changed should succeed");
        daemon
            .minimized_changed("w2", pid, true)
            .expect("minimized_changed should succeed");

        assert_eq!(mock.actions(), vec![SignalAction::Stop(pid)]);
    }

    #[test]
    fn reactivating_after_stop_resumes_once() {
        let (mut daemon, mock) = daemon_with_mock();
        let pid = 4242;

        daemon
            .add_window("w1", pid)
            .expect("add_window should succeed");
        daemon
            .minimized_changed("w1", pid, true)
            .expect("minimized_changed should succeed");
        daemon
            .minimized_changed("w1", pid, false)
            .expect("minimized_changed should succeed");
        daemon
            .minimized_changed("w1", pid, false)
            .expect("minimized_changed should succeed");

        assert_eq!(
            mock.actions(),
            vec![SignalAction::Stop(pid), SignalAction::Cont(pid)]
        );
    }

    #[test]
    fn minimized_changed_errors_for_unknown_window() {
        let (mut daemon, _mock) = daemon_with_mock();
        let pid = 4242;

        daemon
            .add_window("w1", pid)
            .expect("add_window should succeed");
        let err = daemon
            .minimized_changed("missing", pid, true)
            .expect_err("unknown window should error");

        match err {
            fdo::Error::Failed(msg) => assert!(msg.contains("unknown window")),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn remove_last_window_prunes_process() {
        let (mut daemon, _mock) = daemon_with_mock();
        let pid = 4242;

        daemon
            .add_window("w1", pid)
            .expect("add_window should succeed");
        daemon
            .remove_window("w1", pid)
            .expect("remove_window should succeed");

        assert!(!daemon.processes.contains_key(&pid));
    }

    #[test]
    fn closed_window_resumes_napping_process_without_removing_window() {
        let (mut daemon, mock) = daemon_with_mock();
        let pid = 4242;

        daemon
            .add_window("w1", pid)
            .expect("add_window should succeed");
        daemon
            .minimized_changed("w1", pid, true)
            .expect("minimized_changed should succeed");
        daemon.closed("w1", pid).expect("closed should succeed");

        assert_eq!(
            mock.actions(),
            vec![SignalAction::Stop(pid), SignalAction::Cont(pid)]
        );
        assert!(daemon.processes[&pid].windows.contains_key("w1"));
    }

    #[test]
    fn closed_one_of_multiple_windows_resumes_then_remove_can_stop_again() {
        let (mut daemon, mock) = daemon_with_mock();
        let pid = 4242;

        daemon
            .add_window("w1", pid)
            .expect("add_window should succeed");
        daemon
            .add_window("w2", pid)
            .expect("add_window should succeed");
        daemon
            .minimized_changed("w1", pid, true)
            .expect("minimized_changed should succeed");
        daemon
            .minimized_changed("w2", pid, true)
            .expect("minimized_changed should succeed");
        daemon.closed("w1", pid).expect("closed should succeed");
        daemon
            .remove_window("w1", pid)
            .expect("remove_window should succeed");

        assert_eq!(
            mock.actions(),
            vec![
                SignalAction::Stop(pid),
                SignalAction::Cont(pid),
                SignalAction::Stop(pid)
            ]
        );
        assert!(daemon.processes[&pid].windows.contains_key("w2"));
    }
}
