use std::collections::HashMap;

use crate::daemon::{load_tracker::LoadTracker, policy::PolicyKey};

#[derive(Debug)]
pub struct WindowState {
    pub minimized: bool,
    pub active: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Tier {
    Performance,
    Background,
    Nap,
    Unknown, // not reconciled yet
}

/// CPU load sub-state on `background` / `nap`; unused on `performance`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Load {
    Busy,
    Idle,
}

#[derive(Debug)]
pub struct AppState {
    pub windows: HashMap<String, WindowState>,
    pub cgroups: Vec<String>,
    /// Desired tier from window / media / inhibitor events.
    pub tier: Tier,
    /// Last successfully applied effective (tier, load) policy. Stays behind
    /// on failure so the next reconcile retries the transition.
    pub applied: Option<PolicyKey>,
    pub load_tracker: LoadTracker,
}

impl AppState {
    pub fn new(cgroups: Vec<String>) -> Self {
        Self {
            windows: HashMap::new(),
            cgroups,
            tier: Tier::Unknown,
            applied: None,
            load_tracker: LoadTracker::new(),
        }
    }

    pub fn load(&self) -> Load {
        self.load_tracker.load
    }
}
