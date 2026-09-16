use std::collections::HashSet;

use futures::StreamExt;
use log::warn;
use tokio::sync::mpsc;
use zbus::{MatchRule, MessageStream, message::Type};

use crate::{
    daemon::channel_event::ChannelEvent,
    dbus::client::mpris::{MPRIS_PLAYER_PATH, MPRIS_PREFIX, MprisDBusProxy},
};

pub struct MprisWatcher {
    dbus: MprisDBusProxy,
    tx: mpsc::Sender<ChannelEvent>,
}

impl MprisWatcher {
    pub fn new(dbus: MprisDBusProxy, tx: mpsc::Sender<ChannelEvent>) -> Self {
        Self { dbus, tx }
    }

    /// Fetches the units of currently playing players and sends them.
    /// Returns false when the daemon's channel is gone.
    async fn refresh(&self) -> bool {
        let units = self
            .dbus
            .get_playing_player_units()
            .await
            .unwrap_or_else(|e| {
                warn!("failed to poll MPRIS players: {e}");
                HashSet::new()
            });

        self.tx
            .send(ChannelEvent::MediaUnitsChanged(units))
            .await
            .is_ok()
    }

    pub async fn watch(&self) -> zbus::Result<()> {
        // All players share the MediaPlayer2 object path
        // Listen for media property changes (e.g. PlaybackStatus flips)
        // The payload is a trigger only, the real set is refetched in refresh()
        let changes_rule = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.DBus.Properties")?
            .member("PropertiesChanged")?
            .path(MPRIS_PLAYER_PATH)?
            .build();
        let mut changes =
            MessageStream::for_match_rule(changes_rule, self.dbus.connection(), None).await?;

        // Listen for new/removing players
        let mut names = self.dbus.fdo().receive_name_owner_changed().await?;

        if !self.refresh().await {
            return Ok(());
        }

        loop {
            tokio::select! {
                change = changes.next() => {
                    if change.is_none() || !self.refresh().await {
                        return Ok(());
                    }
                }
                name = names.next() => {
                    let Some(signal) = name else { return Ok(()) };
                    let Ok(args) = signal.args() else { continue };
                    if args.name().as_str().starts_with(MPRIS_PREFIX)
                        && !self.refresh().await
                    {
                        return Ok(());
                    }
                }
            }
        }
    }
}
