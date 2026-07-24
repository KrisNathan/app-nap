use tokio::sync::mpsc::Sender;

use crate::daemon::channel_event::ChannelEvent;

pub async fn init(tx: Sender<ChannelEvent>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
    loop {
        interval.tick().await;
        tx.send(ChannelEvent::UsageWatchTick).await;
    }
}
