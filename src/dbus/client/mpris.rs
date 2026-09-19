use libc::pid_t;
use zbus::{Proxy, fdo::DBusProxy, names::OwnedBusName};

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

    pub async fn list_players(&self) -> Result<Vec<OwnedBusName>, zbus::Error> {
        let names = self
            .fdo
            .list_names()
            .await?
            .into_iter()
            .filter(|name| name.starts_with(MPRIS_PREFIX))
            .collect();

        Ok(names)
    }

    /// Returns true when PlaybackStatus is "Playing".
    /// "Paused" and "Stopped" both return false.
    pub async fn is_playing(&self, player: &OwnedBusName) -> Result<bool, zbus::Error> {
        let proxy = Proxy::new(
            &self.conn,
            player.as_ref(),
            MPRIS_PLAYER_PATH,
            MPRIS_PLAYER_INTERFACE,
        )
        .await?;

        Ok(proxy.get_property::<String>("PlaybackStatus").await? == "Playing")
    }

    pub async fn get_pid(&self, player: &OwnedBusName) -> Result<pid_t, zbus::Error> {
        // the conventional UNIX pid_t is i32
        // for some reason zbus uses u32
        Ok(self
            .fdo
            .get_connection_unix_process_id(player.as_ref())
            .await? as pid_t)
    }
}
