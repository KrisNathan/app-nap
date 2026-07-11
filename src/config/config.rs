use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Action {
    Signal,
    SystemdFreeze,
    Ecore,
    SystemdCpuQuota {
        #[serde(default = "default_cpu_quota_percent")]
        percent: u32,
    },
    SystemdCpuWeight {
        #[serde(default = "default_cpu_weight")]
        weight: u64,
    },
}

fn default_cpu_quota_percent() -> u32 {
    10
}

fn default_cpu_weight() -> u64 {
    1
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Tier {
    #[serde(default)]
    pub actions: Vec<Action>,
}

fn default_performance_tier() -> Tier {
    Tier {
        actions: vec![Action::SystemdCpuWeight { weight: 100 }],
    }
}

fn default_background_tier() -> Tier {
    Tier {
        actions: vec![Action::SystemdCpuWeight {
            weight: default_cpu_weight(),
        }],
    }
}

fn default_nap_tier() -> Tier {
    Tier {
        actions: vec![Action::SystemdCpuQuota {
            percent: default_cpu_quota_percent(),
        }],
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tiers {
    #[serde(default = "default_performance_tier")]
    pub performance: Tier,
    #[serde(default = "default_background_tier")]
    pub background: Tier,
    #[serde(default = "default_nap_tier")]
    pub nap: Tier,
}

impl Default for Tiers {
    fn default() -> Self {
        Self {
            performance: default_performance_tier(),
            background: default_background_tier(),
            nap: default_nap_tier(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub tiers: Tiers,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tiers: Tiers::default(),
        }
    }
}
