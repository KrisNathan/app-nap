use std::collections::HashSet;

use futures::StreamExt;
use log::warn;
use tokio::sync::mpsc;

use crate::{
    daemon::channel_event::ChannelEvent,
    dbus::client::powerdevil::{ActiveInhibition, PowerDevilDBusProxy},
};

pub struct PowerDevilInhibitor<'a> {
    dbus: PowerDevilDBusProxy<'a>,
    tx: mpsc::Sender<ChannelEvent>,
}

impl<'a> PowerDevilInhibitor<'a> {
    pub fn new(dbus: PowerDevilDBusProxy<'a>, tx: mpsc::Sender<ChannelEvent>) -> Self {
        Self { dbus, tx }
    }

    async fn active_app_names(&self) -> HashSet<String> {
        match self.dbus.active_inhibitions().await {
            Ok(inhibitions) => inhibitions
                .into_iter()
                .map(|tuple| ActiveInhibition::from(tuple).app_name)
                .collect(),
            Err(e) => {
                warn!("failed to fetch ActiveInhibitions: {e}");
                HashSet::new()
            }
        }
    }

    /// Fetches the current set and sends it.
    /// Returns false when the daemon's channel is gone.
    async fn refresh(&self) -> bool {
        let apps = self.active_app_names().await;
        self.tx
            .send(ChannelEvent::InhibitedAppsChanged(apps))
            .await
            .is_ok()
    }

    pub async fn watch(&self) {
        let mut changes = self.dbus.receive_active_inhibitions_changed().await;

        if !self.refresh().await {
            return;
        }

        // PropertiesChanged for ActiveInhibitions; powerdevil invalidates the
        // property instead of sending the value, so the set is always refetched
        while changes.next().await.is_some() {
            if !self.refresh().await {
                return;
            }
        }
    }
}
