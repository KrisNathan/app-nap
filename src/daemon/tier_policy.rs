use crate::{action::Action, daemon::process::Tier};

pub struct TierPolicy {
    tier: Tier,
    actions: Vec<Action>,
}

impl TierPolicy {
    pub fn new(tier: Tier, actions: Vec<Action>) -> Self {
        Self { tier, actions }
    }

    pub fn actions(&self) -> &[Action] {
        &self.actions
    }

    pub fn tier(&self) -> &Tier {
        &self.tier
    }
}
