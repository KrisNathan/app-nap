use log::warn;
use tokio::{sync::mpsc, time::Interval};

use crate::daemon::{Daemon, channel_event::ChannelEvent};

pub struct EventLoop {
    daemon: Daemon,
    rx: mpsc::Receiver<ChannelEvent>,
    cpu_tick: Interval,
}
impl EventLoop {
    pub fn new(daemon: Daemon, cpu_tick: Interval, rx: mpsc::Receiver<ChannelEvent>) -> Self {
        Self {
            daemon,
            rx,
            cpu_tick,
        }
    }
    pub async fn serve(&mut self) {
        loop {
            tokio::select! {
                event = self.rx.recv() => {
                    let Some(event) = event else { return };
                    self.handle_event(event).await;
                }
                // The timer only ticks while cpu-load polling has targets
                // otherwise the daemon is fully event-driven.
                _ = self.cpu_tick.tick(), if self.daemon.has_polled_apps() => {
                    self.handle_cpu_tick().await;
                }
            }
        }
    }
    async fn handle_event(&mut self, event: ChannelEvent) {
        match event {
            ChannelEvent::InhibitedAppsChanged(apps) => {
                self.daemon.inhibited_apps_changed(apps).await;
            }

            ChannelEvent::MediaUnitsChanged(cgroups) => {
                self.daemon.media_units_changed(cgroups).await;
            }

            ChannelEvent::WindowAdded {
                window_id,
                pid,
                minimized,
                active,
            } => {
                self.daemon
                    .window_added(window_id, pid, minimized, active)
                    .await;
            }

            ChannelEvent::WindowRemoved { window_id, pid } => {
                self.daemon.window_removed(window_id, pid).await;
            }
            ChannelEvent::WindowMinimizedChanged {
                window_id,
                pid,
                minimized,
            } => {
                self.daemon
                    .window_minimized_changed(window_id, pid, minimized)
                    .await;
            }
            ChannelEvent::WindowActiveChanged {
                window_id,
                pid,
                active,
            } => {
                self.daemon
                    .window_active_changed(window_id, pid, active)
                    .await;
            }

            ChannelEvent::ListApps { sender } => {
                let snapshot = self.daemon.list_apps();
                if let Err(_) = sender.send(snapshot) {
                    warn!("ListApps requester disconnected before the snapshot was delivered");
                }
            }
        }
    }
    async fn handle_cpu_tick(&mut self) {
        self.daemon.cpu_load_tick().await;
    }
}
