use libc::pid_t;
use zbus::Proxy;

use crate::{
    media::{MediaError, MediaService},
    systemd::cgroup,
};

pub struct MprisMediaService {
    dbus_conn: zbus::Connection,
}

impl MediaService for MprisMediaService {
    async fn is_playing(&self, cgroups: &[String]) -> Result<bool, MediaError> {
        if cgroups.is_empty() {
            return Ok(false);
        }

        let players = self.list_players().await?;
        let playing_players = self.list_playing_players(players).await?;
        let mut pids = Vec::new();

        for player in &playing_players {
            // A single player's PID may be unresolvable if it vanished
            // mid-poll; skip it rather than failing the whole check.
            if let Ok(pid) = self.get_pid_from_player(player).await {
                pids.push(pid as pid_t);
            }
        }

        let related_pids = cgroup::get_pids_from_cgroups(cgroups)?;

        Ok(related_pids.iter().any(|p| pids.contains(p)))
    }
}

impl MprisMediaService {
    pub fn new(dbus_conn: zbus::Connection) -> Self {
        Self { dbus_conn }
    }

    async fn list_players(&self) -> Result<Vec<String>, zbus::Error> {
        let dbus_proxy = self
            .dbus_conn
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "ListNames",
                &(),
            )
            .await?;

        let names: Vec<String> = dbus_proxy.body().deserialize()?;

        let player_name = names
            .iter()
            .filter(|name| name.starts_with("org.mpris.MediaPlayer2."))
            .cloned()
            .collect::<Vec<String>>();

        Ok(player_name)
    }

    /// returns MPRIS dbus player service names
    async fn list_playing_players(&self, players: Vec<String>) -> Result<Vec<String>, zbus::Error> {
        let mut playing_players = Vec::new();

        for player in players {
            let proxy = Proxy::new(
                &self.dbus_conn,
                player.as_str(),
                "/org/mpris/MediaPlayer2",
                "org.mpris.MediaPlayer2.Player",
            )
            .await?;
            let playback_status: String = proxy.get_property("PlaybackStatus").await?;

            if playback_status == "Playing" {
                playing_players.push(player);
            }
        }

        Ok(playing_players)
    }

    async fn get_pid_from_player(&self, player: &str) -> Result<pid_t, zbus::Error> {
        let reply = self
            .dbus_conn
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "GetConnectionUnixProcessID",
                &player,
            )
            .await?;

        let pid: pid_t = reply.body().deserialize()?;
        Ok(pid)
    }
}
