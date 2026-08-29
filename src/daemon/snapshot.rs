use libc::pid_t;
use serde::Serialize;
use zbus::zvariant::Type;

use crate::daemon::app_state::AppState;

/// One tracked window, as reported over D-Bus.
#[derive(Debug, Serialize, Type)]
pub struct WindowSnapshot {
    pub window_id: String,
    pub minimized: bool,
    pub active: bool,
}

/// One tracked app, as reported over D-Bus.
///
/// `load` is the raw tracker state; it does not select a policy on the
/// performance tier. `policy` is the last successfully applied policy, so it
/// stays behind the desired tier while a transition keeps failing, and is
/// empty when nothing has been applied yet.
#[derive(Debug, Serialize, Type)]
pub struct AppSnapshot {
    pub pid: pid_t,
    pub name: String,
    pub tier: String,
    pub load: String,
    pub policy: String,
    /// CPU usage from the last polled sample, in core-equivalents.
    pub usage: f64,
    /// CPU throttling from the last polled sample, in core-equivalents.
    pub throttle: f64,
    pub cgroups: Vec<String>,
    pub windows: Vec<WindowSnapshot>,
}

impl AppSnapshot {
    /// Project one app's live state. Windows are sorted so repeated queries
    /// return a stable order.
    pub fn new(pid: pid_t, app_state: &AppState) -> Self {
        let mut windows: Vec<WindowSnapshot> = app_state
            .windows
            .iter()
            .map(|(window_id, window)| WindowSnapshot {
                window_id: window_id.clone(),
                minimized: window.minimized,
                active: window.active,
            })
            .collect();
        windows.sort_by(|a, b| a.window_id.cmp(&b.window_id));

        Self {
            pid,
            name: app_state.name.clone(),
            tier: app_state.tier.as_str().to_owned(),
            load: app_state.load().as_str().to_owned(),
            policy: app_state
                .applied
                .map(|key| key.as_str())
                .unwrap_or_default()
                .to_owned(),
            usage: app_state.load_tracker.usage,
            throttle: app_state.load_tracker.throttle,
            cgroups: app_state.cgroups.clone(),
            windows,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::daemon::{
        app_state::{Tier, WindowState},
        policy::PolicyKey,
    };

    fn app_state() -> AppState {
        let mut app_state = AppState::new("firefox".into(), vec!["/app.slice/app-firefox".into()]);
        app_state.windows.insert(
            "window-b".into(),
            WindowState {
                minimized: true,
                active: false,
            },
        );
        app_state.windows.insert(
            "window-a".into(),
            WindowState {
                minimized: false,
                active: true,
            },
        );
        app_state
    }

    #[test]
    fn projects_state_with_sorted_windows() {
        let mut state = app_state();
        state.tier = Tier::Background;
        state.applied = Some(PolicyKey::BackgroundIdle);
        state.load_tracker.usage = 0.25;
        state.load_tracker.throttle = 0.5;

        let snapshot = AppSnapshot::new(42, &state);

        assert_eq!(snapshot.pid, 42);
        assert_eq!(snapshot.name, "firefox");
        assert_eq!(snapshot.tier, "background");
        assert_eq!(snapshot.load, "busy");
        assert_eq!(snapshot.policy, "background-idle");
        assert_eq!(snapshot.usage, 0.25);
        assert_eq!(snapshot.throttle, 0.5);
        assert_eq!(snapshot.cgroups, vec!["/app.slice/app-firefox".to_string()]);
        let windows: Vec<&str> = snapshot
            .windows
            .iter()
            .map(|window| window.window_id.as_str())
            .collect();
        assert_eq!(windows, vec!["window-a", "window-b"]);
        assert!(snapshot.windows[0].active);
        assert!(snapshot.windows[1].minimized);
    }

    #[test]
    fn unapplied_app_reports_empty_policy() {
        let snapshot = AppSnapshot::new(42, &app_state());

        assert_eq!(snapshot.tier, "unknown");
        assert_eq!(snapshot.policy, "");
    }
}
