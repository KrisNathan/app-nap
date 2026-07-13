use libc::pid_t;
use zbus::{fdo, interface};

use crate::{daemon::daemon::Daemon, inhibit::InhibitService, media::MediaService};

pub struct DBusDaemon<I, M>
where
    I: InhibitService,
    M: MediaService,
{
    daemon: Daemon<I, M>,
}

impl<I, M> DBusDaemon<I, M>
where
    I: InhibitService,
    M: MediaService,
{
    pub fn new(daemon: Daemon<I, M>) -> Self {
        Self { daemon }
    }
}

#[interface(name = "dev.appnap.AppNap1")]
impl<I, M> DBusDaemon<I, M>
where
    I: InhibitService + 'static,
    M: MediaService + 'static,
{
    async fn add_window(&mut self, window_id: &str, pid: pid_t) -> fdo::Result<()> {
        validate_input(window_id, pid)?;
        self.daemon.add_window(window_id, pid).await;
        Ok(())
    }

    async fn remove_window(&mut self, window_id: &str, pid: pid_t) -> fdo::Result<()> {
        validate_input(window_id, pid)?;
        self.daemon.remove_window(window_id, pid).await;
        Ok(())
    }

    async fn minimized_changed(
        &mut self,
        window_id: &str,
        pid: pid_t,
        _minimized: bool,
    ) -> fdo::Result<()> {
        validate_input(window_id, pid)?;
        self.daemon
            .window_minimize_changed(window_id, pid, _minimized)
            .await;
        Ok(())
    }

    async fn active_changed(
        &mut self,
        window_id: &str,
        pid: pid_t,
        _active: bool,
    ) -> fdo::Result<()> {
        validate_input(window_id, pid)?;
        self.daemon
            .window_active_changed(window_id, pid, _active)
            .await;
        Ok(())
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
