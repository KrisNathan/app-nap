use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub struct WindowState {
    pub minimized: bool,
    pub active: bool,
}

#[derive(Debug)]
pub struct Process {
    pub windows: HashMap<String, WindowState>,
    pub is_napping: bool,
}

impl Process {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            is_napping: false,
        }
    }
}
