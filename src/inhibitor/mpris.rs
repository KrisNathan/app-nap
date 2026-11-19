use std::collections::{HashMap, HashSet};

use futures::StreamExt;
use log::warn;
use tokio::sync::mpsc;
use zbus::{MatchRule, MessageStream, message::Type, names::OwnedBusName};

use crate::{
    cgroup::{UnitPath, procfs::cgroup_path_from_pid},
    daemon::channel_event::ChannelEvent,
    dbus::client::mpris::{MPRIS_PLAYER_PATH, MPRIS_PREFIX, MprisDBusProxy},
};

pub struct MprisWatcher {
    dbus: MprisDBusProxy,
    tx: mpsc::Sender<ChannelEvent>,
    /// Last known unit per playing player.
    /// An entry survives a failed poll, so a transient error does not read
    /// as "playback stopped".
    playing: HashMap<OwnedBusName, UnitPath>,
}

/// What the watcher knows about one player.
enum PlayerState {
    /// PlaybackStatus is "Playing" and the unit is resolved.
    Playing(UnitPath),
    /// PlaybackStatus is readable and not "Playing" (Paused or Stopped).
    NotPlaying,
    /// The status or the unit could not be read. The last known unit is kept.
    Unknown,
}

impl MprisWatcher {
    pub fn new(dbus: MprisDBusProxy, tx: mpsc::Sender<ChannelEvent>) -> Self {
        Self {
            dbus,
            tx,
            playing: HashMap::new(),
        }
    }

    /// Resolves the state of one player: reads the playback status and the unit.
    async fn resolve_player_state(&self, player: &OwnedBusName) -> PlayerState {
        let playing = match self.dbus.is_playing(player).await {
            Ok(playing) => playing,
            Err(e) => {
                warn!("failed to get PlaybackStatus for player {player}: {e}");
                return PlayerState::Unknown;
            }
        };

        if !playing {
            return PlayerState::NotPlaying;
        }

        let pid = match self.dbus.get_pid(player).await {
            Ok(pid) => pid,
            Err(e) => {
                warn!("failed to get PID for player {player}: {e}");
                return PlayerState::Unknown;
            }
        };

        let cgroup = match cgroup_path_from_pid(pid) {
            Ok(cgroup) => cgroup,
            Err(e) => {
                warn!("failed to resolve cgroup for player {player} (pid {pid}): {e}");
                return PlayerState::Unknown;
            }
        };

        match UnitPath::from_cgroup_path(cgroup) {
            Some(unit) => PlayerState::Playing(unit),
            None => {
                warn!("not an app cgroup for player {player} (pid {pid})");
                PlayerState::Unknown
            }
        }
    }

    /// Fetches the units of currently playing players and sends them.
    /// A player whose state is unknown keeps its last known unit.
    /// Returns false when the daemon's channel is gone.
    async fn refresh(&mut self) -> bool {
        let players = match self.dbus.list_players().await {
            Ok(players) => players,
            Err(e) => {
                warn!("failed to poll MPRIS players: {e}");
                return true; // don't send MediaUnitsChanged, preserve old state
            }
        };

        let mut states = HashMap::with_capacity(players.len());
        for player in players {
            let state = self.resolve_player_state(&player).await;
            states.insert(player, state);
        }

        update_playing(&mut self.playing, states);

        let units: HashSet<UnitPath> = self.playing.values().cloned().collect();
        self.tx
            .send(ChannelEvent::MediaUnitsChanged(units))
            .await
            .is_ok()
    }

    pub async fn watch(&mut self) -> zbus::Result<()> {
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

/// Updates the playing set with the new per-player states.
/// - `Playing(unit)` replaces the player's entry.
/// - `NotPlaying` drops the player's entry.
/// - `Unknown` keeps the player's last known entry.
/// - A player absent from `states` left the bus, so it is dropped.
fn update_playing(
    playing: &mut HashMap<OwnedBusName, UnitPath>,
    states: HashMap<OwnedBusName, PlayerState>,
) {
    playing.retain(|player, _| states.contains_key(player));

    for (player, state) in states {
        match state {
            PlayerState::Playing(unit) => {
                playing.insert(player, unit);
            }
            PlayerState::NotPlaying => {
                playing.remove(&player);
            }
            PlayerState::Unknown => {} // keep the last known unit
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cgroup::CgroupPath;

    use super::*;

    fn make_player(name: &str) -> OwnedBusName {
        OwnedBusName::try_from(name).unwrap()
    }

    fn resolve_unit(path: &str) -> UnitPath {
        let full = CgroupPath::from_cgroup_full(path).expect("test input must parse");
        UnitPath::from_cgroup_path(full).expect("test input must be an app unit")
    }

    const KONSOLE: &str = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-39967.scope\n";
    const FIREFOX: &str =
        "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-firefox-42.scope\n";

    #[test]
    fn should_replace_the_previous_unit_when_the_player_plays() {
        let spotify = make_player("org.mpris.MediaPlayer2.spotify");
        let mut playing = HashMap::from([(spotify.clone(), resolve_unit(KONSOLE))]);
        let states =
            HashMap::from([(spotify.clone(), PlayerState::Playing(resolve_unit(FIREFOX)))]);

        update_playing(&mut playing, states);

        assert_eq!(playing, HashMap::from([(spotify, resolve_unit(FIREFOX))]));
    }

    #[test]
    fn should_drop_the_player_when_it_is_not_playing() {
        let spotify = make_player("org.mpris.MediaPlayer2.spotify");
        let mut playing = HashMap::from([(spotify.clone(), resolve_unit(KONSOLE))]);
        let states = HashMap::from([(spotify, PlayerState::NotPlaying)]);

        update_playing(&mut playing, states);

        assert!(playing.is_empty());
    }

    #[test]
    fn should_keep_the_last_known_unit_when_the_state_is_unknown() {
        let spotify = make_player("org.mpris.MediaPlayer2.spotify");
        let mut playing = HashMap::from([(spotify.clone(), resolve_unit(KONSOLE))]);
        let states = HashMap::from([(spotify.clone(), PlayerState::Unknown)]);

        update_playing(&mut playing, states);

        assert_eq!(playing, HashMap::from([(spotify, resolve_unit(KONSOLE))]));
    }

    #[test]
    fn should_omit_an_unknown_player_without_history() {
        let spotify = make_player("org.mpris.MediaPlayer2.spotify");
        let mut playing = HashMap::new();
        let states = HashMap::from([(spotify, PlayerState::Unknown)]);

        update_playing(&mut playing, states);

        assert!(playing.is_empty());
    }

    #[test]
    fn should_drop_a_player_that_left_the_bus() {
        let spotify = make_player("org.mpris.MediaPlayer2.spotify");
        let mut playing = HashMap::from([(spotify, resolve_unit(KONSOLE))]);

        update_playing(&mut playing, HashMap::new());

        assert!(playing.is_empty());
    }

    #[test]
    fn should_keep_a_shared_unit_while_one_player_plays_it() {
        let a = make_player("org.mpris.MediaPlayer2.a");
        let b = make_player("org.mpris.MediaPlayer2.b");
        let mut playing = HashMap::from([
            (a.clone(), resolve_unit(KONSOLE)),
            (b.clone(), resolve_unit(KONSOLE)),
        ]);
        let states = HashMap::from([
            (a.clone(), PlayerState::Playing(resolve_unit(KONSOLE))),
            (b, PlayerState::NotPlaying),
        ]);

        update_playing(&mut playing, states);

        assert_eq!(playing, HashMap::from([(a, resolve_unit(KONSOLE))]));
    }
}
