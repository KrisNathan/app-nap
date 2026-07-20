use std::collections::HashMap;

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
    Unknown, // no successfully applied tier yet
}

#[derive(Debug)]
pub struct AppState {
    pub windows: HashMap<String, WindowState>,
    pub cgroups: Vec<String>,
    pub tier: Tier,
}

impl AppState {
    pub fn new(cgroups: Vec<String>) -> Self {
        Self {
            windows: HashMap::new(),
            cgroups,
            tier: Tier::Unknown,
        }
    }
}
