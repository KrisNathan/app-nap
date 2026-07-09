mod process;

pub use process::{Process, WindowState};

use crate::{
    inhibit_service::InhibitService, media_service::MediaService, nap_backend::NapBackend,
    systemd::cgroup,
};
use libc::pid_t;
use std::{collections::HashMap, io, sync::Arc};
use zbus::{fdo, interface};

pub struct Daemon<MS: MediaService, IS: InhibitService> {
    processes: HashMap<pid_t, Process>,
    inactive_backend: Arc<dyn NapBackend>,
    nap_backend: Arc<dyn NapBackend>,
    media_service: Arc<MS>,
    inhibit_service: Arc<IS>,
}

impl<MS: MediaService, IS: InhibitService> Daemon<MS, IS> {
    pub fn new(
        inactive_backend: Arc<dyn NapBackend>,
        nap_backend: Arc<dyn NapBackend>,
        media_service: Arc<MS>,
        inhibit_service: Arc<IS>,
    ) -> Self {
        Self {
            processes: HashMap::new(),
            inactive_backend,
            nap_backend,
            media_service,
            inhibit_service,
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
        let inhibited = self.is_inhibiting(pid).await;

        let Some(process) = self.processes.get_mut(&pid) else {
            return Ok(());
        };

        println!("{process:?}");

        let has_active_window = process.windows.values().any(|window| window.active);
        let has_unminimized_window = process.windows.values().any(|window| !window.minimized);
        let keep_awake = media_playing || inhibited;

        // Unfocused tier: lower CPUWeight unless focused or hard keep-awake.
        if has_active_window {
            self.inactive_backend.send_cont(pid).map_err(|err| {
                fdo::Error::Failed(format!(
                    "failed to set state to active for pid {pid}: {err}"
                ))
            })?;
        } else {
            self.inactive_backend.send_stop(pid).map_err(|err| {
                fdo::Error::Failed(format!(
                    "failed to set state to inactive for pid {pid}: {err}"
                ))
            })?;
        }

        // Minimized tier: hard nap only when every window is minimized.
        if process.is_napping && (has_unminimized_window || keep_awake) {
            self.nap_backend
                .send_cont(pid)
                .map_err(|err| fdo::Error::Failed(format!("failed to SIGCONT pid {pid}: {err}")))?;
            process.is_napping = false;
        } else if !has_unminimized_window && !process.is_napping && !keep_awake {
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

    // Don't throttle apps inhibiting idle (screen recording, streaming, video call)
    // Who is usually desktop/flatpak id (e.g. "com.obsproject.Studio", "firefox")
    // Luckily it's a substring of cgroup (e.g. `app-flatpak-com.obsproject.Studio-<id>.scope`)
    async fn is_inhibiting(&self, pid: pid_t) -> bool {
        let Ok(cgroup) = cgroup::get_process_cgroup(pid) else {
            return false;
        };
        let Ok(inhibitors) = self.inhibit_service.list_inhibitors().await else {
            return false;
        };
        inhibitors.iter().any(|who| cgroup.contains(who))
    }

    fn cleanup_pid(&self, pid: pid_t, was_napping: bool) -> fdo::Result<()> {
        Self::cleanup_backend(pid, &self.inactive_backend, "inactive")?;
        if was_napping {
            Self::cleanup_backend(pid, &self.nap_backend, "nap")?;
        }
        Ok(())
    }

    fn cleanup_backend(pid: pid_t, backend: &Arc<dyn NapBackend>, label: &str) -> fdo::Result<()> {
        match backend.send_cont(pid) {
            Ok(()) => Ok(()),
            Err(_) if Self::is_process_gone(pid) => Ok(()),
            Err(err) => Err(fdo::Error::Failed(format!(
                "failed to cleanup {label} state for pid {pid}: {err}"
            ))),
        }
    }

    fn is_process_gone(pid: pid_t) -> bool {
        let result = unsafe { libc::kill(pid, 0) };
        result == -1 && io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
    }
}

#[interface(name = "dev.appnap.AppNap1")]
impl<MS, IS> Daemon<MS, IS>
where
    MS: MediaService + 'static,
    IS: InhibitService + 'static,
{
    async fn add_window(&mut self, window_id: &str, pid: pid_t) -> fdo::Result<()> {
        Self::validate_input(window_id, pid)?;

        let process = self.processes.entry(pid).or_insert_with(Process::new);
        process
            .windows
            .entry(window_id.to_string())
            .or_insert(WindowState {
                minimized: false,
                active: true,
            });

        self.reconcile_pid(pid).await
    }

    async fn remove_window(&mut self, window_id: &str, pid: pid_t) -> fdo::Result<()> {
        Self::validate_input(window_id, pid)?;

        if let Some(process) = self.processes.get_mut(&pid) {
            process.windows.remove(window_id);

            if process.windows.is_empty() {
                let was_napping = process.is_napping;
                self.cleanup_pid(pid, was_napping)?;
                self.processes.remove(&pid);
                return Ok(());
            }
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

    async fn active_changed(
        &mut self,
        window_id: &str,
        pid: pid_t,
        active: bool,
    ) -> fdo::Result<()> {
        Self::validate_input(window_id, pid)?;

        let process = self
            .processes
            .get_mut(&pid)
            .ok_or_else(|| fdo::Error::Failed(format!("unknown pid {pid}")))?;
        let window = process.windows.get_mut(window_id).ok_or_else(|| {
            fdo::Error::Failed(format!("unknown window {window_id} for pid {pid}"))
        })?;

        window.active = active;
        self.reconcile_pid(pid).await
    }
}
