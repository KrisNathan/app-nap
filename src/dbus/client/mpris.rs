use std::collections::HashSet;

use libc::pid_t;
use log::warn;
use zbus::{Proxy, fdo::DBusProxy, names::OwnedBusName};

use crate::cgroup::{CgroupPath, UnitPath};

pub const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
pub const MPRIS_PLAYER_PATH: &str = "/org/mpris/MediaPlayer2";
const MPRIS_PLAYER_INTERFACE: &str = "org.mpris.MediaPlayer2.Player";

pub struct MprisDBusProxy {
    conn: zbus::Connection,

    /// fdo = freedesktop.org DBus proxy
    fdo: DBusProxy<'static>,
}
impl MprisDBusProxy {
    pub async fn new(conn: zbus::Connection) -> Result<Self, zbus::Error> {
        let fdo = DBusProxy::new(&conn).await?;
        Ok(Self { conn, fdo })
    }

    pub fn connection(&self) -> &zbus::Connection {
        &self.conn
    }

    pub fn fdo(&self) -> &DBusProxy<'static> {
        &self.fdo
    }

    async fn get_players(&self) -> Result<Vec<OwnedBusName>, zbus::Error> {
        let names = self
            .fdo
            .list_names()
            .await?
            .into_iter()
            .filter(|name| name.starts_with(MPRIS_PREFIX))
            .collect();

        Ok(names)
    }

    async fn is_playing(&self, player: &OwnedBusName) -> Result<bool, zbus::Error> {
        let Ok(proxy) = Proxy::new(
            &self.conn,
            player.as_ref(),
            MPRIS_PLAYER_PATH,
            MPRIS_PLAYER_INTERFACE,
        )
        .await
        else {
            return Ok(false);
        };

        Ok(proxy.get_property::<String>("PlaybackStatus").await? == "Playing")
    }

    async fn get_pid(&self, player: &OwnedBusName) -> Result<pid_t, zbus::Error> {
        // the conventional UNIX pid_t is i32
        // for some reason zbus uses u32
        Ok(self
            .fdo
            .get_connection_unix_process_id(player.as_ref())
            .await? as pid_t)
    }

    pub async fn get_playing_player_units(&self) -> Result<HashSet<UnitPath>, zbus::Error> {
        let mut units = HashSet::<UnitPath>::new();

        for player in self.get_players().await? {
            let playing = self.is_playing(&player).await.unwrap_or_else(|e| {
                warn!("Failed to get PlaybackStatus for player {player}: {e}");
                false
            });

            if !playing {
                continue;
            }

            let pid = match self.get_pid(&player).await {
                Ok(pid) => pid,
                Err(e) => {
                    warn!("Failed to get PID for player {player}: {e}");
                    continue;
                }
            };

            let cg = match CgroupPath::from_pid(pid) {
                Ok(cg) => cg,
                Err(e) => {
                    warn!("Failed to resolve cgroup for player {player} (pid {pid}): {e}");
                    continue;
                }
            };

            let Some(unit_path) = UnitPath::from_cgroup_path(cg) else {
                warn!("Not an app cgroup for player {player} (pid {pid})");
                continue;
            };

            units.insert(unit_path);
        }

        Ok(units)
    }
}
