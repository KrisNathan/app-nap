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

impl PolicyKey {
    pub fn as_str(self) -> &'static str {
        match self {
            PolicyKey::Performance => "performance",
            PolicyKey::BackgroundBusy => "background-busy",
            PolicyKey::BackgroundIdle => "background-idle",
            PolicyKey::NapIdle => "nap-idle",
            PolicyKey::NapBusy => "nap-busy",
        }
    }
}

/// Action list for one effective policy.
pub struct Policy {
    actions: Vec<Action>,
}

impl Policy {
    pub fn new(actions: Vec<Action>) -> Self {
        Self { actions }
    }

    pub fn actions(&self) -> &[Action] {
        &self.actions
    }
}
