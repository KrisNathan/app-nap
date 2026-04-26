mod process;

pub use process::{Process, WindowState};

use crate::{media_service::MediaService, nap_backend::NapBackend};
use libc::pid_t;
use std::{collections::HashMap, sync::Arc};
use zbus::{fdo, interface};

pub struct Daemon<MS: MediaService> {
    processes: HashMap<pid_t, Process>,
    nap_backend: Arc<dyn NapBackend>,
    media_service: Arc<MS>,
}

impl<MS: MediaService> Daemon<MS> {
    pub fn new(nap_backend: Arc<dyn NapBackend>, media_service: Arc<MS>) -> Self {
        Self {
            processes: HashMap::new(),
            nap_backend,
            media_service,
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

    async fn reconcile_pid(&mut self, pid: pid_t) -> fdo::Result<()> {
        let media_playing = self.is_media_playing(pid).await;

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

    async fn is_media_playing(&self, pid: pid_t) -> bool {
        let playing_pids = self
            .media_service
            .list_playing_media_pids()
            .await
            .unwrap_or_default();
        playing_pids.contains(&pid)
    }
}

#[interface(name = "dev.appnap.AppNap1")]
impl<MS> Daemon<MS>
where
    MS: MediaService + 'static,
{
    async fn add_window(&mut self, window_id: &str, pid: pid_t) -> fdo::Result<()> {
        Self::validate_input(window_id, pid)?;

        let process = self.processes.entry(pid).or_insert_with(Process::new);
        process
            .windows
            .entry(window_id.to_string())
            .or_insert(WindowState { minimized: false });

        self.reconcile_pid(pid).await
    }

    async fn remove_window(&mut self, window_id: &str, pid: pid_t) -> fdo::Result<()> {
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

        self.reconcile_pid(pid).await
    }

    async fn minimized_changed(
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
        self.reconcile_pid(pid).await
    }
}
