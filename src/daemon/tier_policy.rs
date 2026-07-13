use crate::{action::Action, daemon::process::Tier};

pub struct TierPolicy {
    tier: Tier,
    actions: Vec<Box<dyn Action>>,
}

impl TierPolicy {
    pub fn new(tier: Tier, actions: Vec<Box<dyn Action>>) -> Self {
        Self { tier, actions }
    }

    pub fn apply(&self, app_state: &mut crate::daemon::process::AppState) {
        app_state.tier = self.tier.clone();
        for action in &self.actions {
            action.apply(&app_state.cgroups);
        }
    }

    pub fn revert(&self, app_state: &mut crate::daemon::process::AppState) {
        for action in &self.actions {
            action.revert(&app_state.cgroups);
        }
    }
}
