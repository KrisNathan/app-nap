use libc::pid_t;
use zbus::{Connection, Proxy};

use super::MediaService;

pub struct MprisMediaService {
    conn: Connection,
}

impl MediaService for MprisMediaService {
    async fn list_playing_media_pids(&self) -> Result<Vec<libc::pid_t>, zbus::Error> {
        let players = self.list_players().await?;
        let playing_players = self.list_playing_players(players).await?;
        let mut pids = Vec::new();

        for player in playing_players {
            let pid = self.get_pid_from_player(&player).await?;
            pids.push(pid as pid_t);
        }

        Ok(pids)
    }
}

impl MprisMediaService {
    pub async fn new() -> Result<Self, zbus::Error> {
        Ok(MprisMediaService {
            conn: Connection::session().await?,
        })
    }

    // returns MPRIS dbus player service names
    pub async fn list_players(&self) -> Result<Vec<String>, zbus::Error> {
        let dbus_proxy = self
            .conn
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

    // returns MPRIS dbus player service names
    pub async fn list_playing_players(
        &self,
        players: Vec<String>,
    ) -> Result<Vec<String>, zbus::Error> {
        let mut playing_players = Vec::new();

        for player in players {
            let proxy = Proxy::new(
                &self.conn,
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

    pub async fn get_pid_from_player(&self, player: &str) -> Result<u32, zbus::Error> {
        let reply = self
            .conn
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "GetConnectionUnixProcessID",
                &player,
            )
            .await?;

        let pid: u32 = reply.body().deserialize()?;
        Ok(pid)
    }
}
