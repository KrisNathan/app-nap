use std::collections::HashSet;

use crate::cgroup::Cgroup;

#[derive(Default)]
pub struct WakeSignals {
    /// cgroup unit contains these app names
    pub powerdevil_apps: HashSet<String>,
    pub mpris_units: HashSet<Cgroup>,
}

impl WakeSignals {
    pub fn is_inhibiting(&self, cgroups: &HashSet<Cgroup>) -> bool {
        cgroups.iter().any(|cg| {
            self.powerdevil_apps
                .iter()
                .any(|app| cg.get_full().as_str().contains(app.as_str()))
        })
    }

    pub fn is_playing_media(&self, cgroups: &HashSet<Cgroup>) -> bool {
        cgroups.iter().any(|cg| {
            self.mpris_units
                .iter()
                .any(|player| player.get_unit_name() == cg.get_unit_name())
        })
    }
}
