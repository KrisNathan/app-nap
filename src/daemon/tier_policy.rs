use crate::action::Action;

/// Effective policy selected by (tier, load). Load variants fall back to the
/// tier's base policy when not configured, so the key already reflects that
/// collapse: equal keys mean equal action sets.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PolicyKey {
    Performance,
    BackgroundBusy,
    BackgroundIdle,
    NapIdle,
    NapBusy,
}

pub struct TierPolicy {
    actions: Vec<Action>,
}

impl TierPolicy {
    pub fn new(actions: Vec<Action>) -> Self {
        Self { actions }
    }

    pub fn actions(&self) -> &[Action] {
        &self.actions
    }
}
