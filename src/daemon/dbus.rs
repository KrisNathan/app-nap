use libc::pid_t;
use tokio::sync::mpsc::Sender;
use zbus::{fdo, interface};

use crate::daemon::channel_event::ChannelEvent;

pub struct DBusDaemon {
    tx: Sender<ChannelEvent>,
}

impl DBusDaemon {
    pub fn new(tx: Sender<ChannelEvent>) -> Self {
        Self { tx }
    }

    async fn enqueue(&self, event: ChannelEvent) -> fdo::Result<()> {
        self.tx
            .send(event)
            .await
            .map_err(|_| fdo::Error::Failed("daemon event loop is not running".into()))
    }
}

#[interface(name = "dev.appnap.AppNap1")]
impl DBusDaemon {
    async fn add_window(&mut self, window_id: &str, pid: pid_t) -> fdo::Result<()> {
        validate_input(window_id, pid)?;
        self.enqueue(ChannelEvent::AddWindow {
            window_id: window_id.into(),
            pid,
        })
        .await
    }

    async fn remove_window(&mut self, window_id: &str, pid: pid_t) -> fdo::Result<()> {
        validate_input(window_id, pid)?;
        self.enqueue(ChannelEvent::RemoveWindow {
            window_id: window_id.into(),
            pid,
        })
        .await
    }

    async fn minimized_changed(
        &mut self,
        window_id: &str,
        pid: pid_t,
        minimized: bool,
    ) -> fdo::Result<()> {
        validate_input(window_id, pid)?;
        self.enqueue(ChannelEvent::MinimizedChanged {
            window_id: window_id.into(),
            pid,
            minimized,
        })
        .await
    }

    async fn active_changed(
        &mut self,
        window_id: &str,
        pid: pid_t,
        active: bool,
    ) -> fdo::Result<()> {
        validate_input(window_id, pid)?;
        self.enqueue(ChannelEvent::ActiveChanged {
            window_id: window_id.into(),
            pid,
            active,
        })
        .await
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
