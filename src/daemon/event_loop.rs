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
                // The timer only ticks while cpu-load polling has targets;
                // otherwise the daemon is fully event-driven.
                _ = self.cpu_tick.tick(), if self.has_cpu_load_apps() => {
                    self.handle_cpu_tick().await;
                }
            }
        }
    }
    async fn handle_event(&mut self, event: ChannelEvent) {}
    async fn handle_cpu_tick(&mut self) {}
}
