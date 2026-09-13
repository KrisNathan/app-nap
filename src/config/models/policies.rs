use serde::{Deserialize, Serialize};

use crate::config::models::action::ActionConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoliciesConfig {
    pub performance: PolicyConfig,
    pub background_idle: PolicyConfig,
    pub background_busy: PolicyConfig,
    pub nap_idle: PolicyConfig,
    pub nap_busy: PolicyConfig,
}
impl Default for PoliciesConfig {
    fn default() -> Self {
        Self {
            performance: PolicyConfig {
                actions: vec![ActionConfig::SystemdCpuWeight { weight: 100 }],
            },
            background_idle: PolicyConfig {
                actions: vec![
                    ActionConfig::SystemdCpuWeight { weight: 1 },
                    ActionConfig::Ecore,
                ],
            },
            background_busy: PolicyConfig {
                actions: vec![ActionConfig::SystemdCpuWeight { weight: 1 }],
            },
            nap_idle: PolicyConfig {
                actions: vec![
                    ActionConfig::SystemdCpuQuota { percent: 10 },
                    ActionConfig::Ecore,
                ],
            },
            nap_busy: PolicyConfig {
                actions: vec![
                    ActionConfig::SystemdCpuQuota { percent: 50 },
                    ActionConfig::Ecore,
                ],
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    actions: Vec<ActionConfig>,
}
