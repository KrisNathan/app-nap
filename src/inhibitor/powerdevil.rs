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

    async fn active_app_names(&self) -> zbus::Result<HashSet<String>> {
        Ok(self
            .dbus
            .active_inhibitions()
            .await?
            .into_iter()
            .map(|tuple| ActiveInhibition::from(tuple).app_name)
            .collect())
    }

    /// Fetches the current set and sends it.
    /// Returns false when the daemon's channel is gone.
    async fn refresh(&self) -> bool {
        let apps = match self.active_app_names().await {
            Ok(apps) => apps,
            Err(e) => {
                warn!("failed to fetch ActiveInhibitions: {e}");
                return true; // don't send InhibittedAppsChanged, preserve old state
            }
        };
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
