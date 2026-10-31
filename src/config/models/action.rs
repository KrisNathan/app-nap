use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ActionConfig {
    Ecore,
    SystemdFreeze,
    SystemdCpuQuota { percent: u64 },
    SystemdCpuWeight { weight: u64 },
}
