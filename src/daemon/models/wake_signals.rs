use std::collections::HashSet;

use crate::cgroup::{UnitName, UnitPath};

#[derive(Default)]
pub struct WakeSignals {
    /// cgroup unit contains these app names
    pub powerdevil_apps: HashSet<String>,
    pub mpris_units: HashSet<UnitPath>,
}

impl WakeSignals {
    pub fn is_inhibiting(&self, unit_paths: &HashSet<UnitPath>) -> bool {
        unit_paths.iter().any(|unit_path| {
            let unit_name = UnitName::from(unit_path).as_str().to_lowercase();
            self.powerdevil_apps
                .iter()
                .any(|app| unit_name.contains(&app.to_lowercase()))
        })
    }

    pub fn is_playing_media(&self, unit_paths: &HashSet<UnitPath>) -> bool {
        unit_paths
            .iter()
            .any(|path| self.mpris_units.contains(path))
    }
}
