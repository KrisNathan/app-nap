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

impl Tier {
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Performance => "performance",
            Tier::Background => "background",
            Tier::Nap => "nap",
            Tier::Unknown => "unknown",
        }
    }
}

/// CPU load sub-state on `background` / `nap`; unused on `performance`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Load {
    Busy,
    Idle,
}

impl Load {
    pub fn as_str(self) -> &'static str {
        match self {
            Load::Busy => "busy",
            Load::Idle => "idle",
        }
    }
}

#[derive(Debug)]
pub struct AppState {
    /// `comm` of the window pid, cached for reporting only.
    pub name: String,
    pub windows: HashMap<String, WindowState>,
    pub cgroups: Vec<String>,
    /// Desired tier from window / media / inhibitor events.
    pub tier: Tier,
    /// Last successfully applied effective (tier, load) policy.
    /// Stays behind on failure so the next reconcile retries the transition.
    /// Cleared when the related cgroup set changes so apply runs against the new units.
    pub applied: Option<PolicyKey>,
    pub load_tracker: LoadTracker,
}

impl AppState {
    pub fn new(name: String, cgroups: Vec<String>) -> Self {
        Self {
            name,
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
