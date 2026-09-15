use libc::pid_t;
use tokio::sync::mpsc;
use zbus::{fdo, interface};

use crate::daemon::channel_event::ChannelEvent;

pub struct DBusDaemon {
    tx: mpsc::Sender<ChannelEvent>,
}

impl DBusDaemon {
    pub fn new(tx: mpsc::Sender<ChannelEvent>) -> Self {
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
    async fn add_window(&self, window_id: &str, pid: pid_t) -> fdo::Result<()> {
        validate_input(window_id, pid)?;
        self.enqueue(ChannelEvent::WindowAdded {
            window_id: window_id.into(),
            pid,
        })
        .await
    }

    async fn remove_window(&self, window_id: &str, pid: pid_t) -> fdo::Result<()> {
        validate_input(window_id, pid)?;
        self.enqueue(ChannelEvent::WindowRemoved {
            window_id: window_id.into(),
            pid,
        })
        .await
    }

    async fn minimized_changed(
        &self,
        window_id: &str,
        pid: pid_t,
        minimized: bool,
    ) -> fdo::Result<()> {
        validate_input(window_id, pid)?;
        self.enqueue(ChannelEvent::WindowMinimizedChanged {
            window_id: window_id.into(),
            pid,
            minimized,
        })
        .await
    }

    async fn active_changed(&self, window_id: &str, pid: pid_t, active: bool) -> fdo::Result<()> {
        validate_input(window_id, pid)?;
        self.enqueue(ChannelEvent::WindowActiveChanged {
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
