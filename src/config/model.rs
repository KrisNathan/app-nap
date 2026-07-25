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

/// Actions for one config tier slot. `[tiers.x]` is the base; optional
/// `[tiers.x.idle]` / `[tiers.x.busy]` override it for that load.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TierConfig {
    #[serde(default)]
    pub actions: Vec<Action>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idle: Option<Box<TierConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub busy: Option<Box<TierConfig>>,
}

fn default_performance_tier() -> TierConfig {
    TierConfig {
        actions: vec![Action::SystemdCpuWeight { weight: 100 }],
        ..Default::default()
    }
}

fn default_background_tier() -> TierConfig {
    TierConfig {
        actions: vec![Action::SystemdCpuWeight {
            weight: default_cpu_weight(),
        }],
        ..Default::default()
    }
}

fn default_nap_tier() -> TierConfig {
    TierConfig {
        actions: vec![Action::SystemdCpuQuota {
            percent: default_cpu_quota_percent(),
        }],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tiers {
    #[serde(default = "default_performance_tier")]
    pub performance: TierConfig,
    #[serde(default = "default_background_tier")]
    pub background: TierConfig,
    #[serde(default = "default_nap_tier")]
    pub nap: TierConfig,
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

fn default_interval_ms() -> u64 {
    10_000
}

/// Quiet-but-chatty apps (a browser ticking timers, a chat client polling)
/// idle around 0.05-0.10, so the idle ceiling sits above that band. Busy keeps
/// a 0.10-wide dead band above it, which is what stops idle/busy flapping.
fn default_idle_threshold() -> f64 {
    0.10
}

fn default_busy_threshold() -> f64 {
    0.20
}

fn default_throttle_idle_max() -> f64 {
    0.01
}

fn default_throttle_busy() -> f64 {
    0.05
}

fn default_ttl_idle() -> u32 {
    2
}

fn default_ttl_busy() -> u32 {
    1
}

/// Polling settings for CPU load on background/nap apps.
/// Usage and throttle are in core-equivalents (1.0 = one fully busy core).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuLoadPollingConfig {
    #[serde(default = "default_interval_ms")]
    pub interval_ms: u64,
    #[serde(default = "default_idle_threshold")]
    pub idle_threshold: f64,
    #[serde(default = "default_busy_threshold")]
    pub busy_threshold: f64,
    #[serde(default = "default_throttle_idle_max")]
    pub throttle_idle_max: f64,
    #[serde(default = "default_throttle_busy")]
    pub throttle_busy: f64,
    #[serde(default = "default_ttl_idle")]
    pub ttl_idle: u32,
    #[serde(default = "default_ttl_busy")]
    pub ttl_busy: u32,
}

impl Default for CpuLoadPollingConfig {
    fn default() -> Self {
        Self {
            interval_ms: default_interval_ms(),
            idle_threshold: default_idle_threshold(),
            busy_threshold: default_busy_threshold(),
            throttle_idle_max: default_throttle_idle_max(),
            throttle_busy: default_throttle_busy(),
            ttl_idle: default_ttl_idle(),
            ttl_busy: default_ttl_busy(),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub tiers: Tiers,
    #[serde(default)]
    pub cpu_load_polling: CpuLoadPollingConfig,
}

#[cfg(test)]
mod tests {
    use super::{Action, Config};

    #[test]
    fn parses_load_variants_and_cpu_load_polling() {
        let config: Config = toml::from_str(
            r#"
[cpu_load_polling]
interval_ms = 5000
ttl_idle = 3

[tiers.background]
actions = [{ type = "systemd-cpu-weight", weight = 1 }]

[tiers.background.idle]
actions = [{ type = "ecore" }]

[tiers.nap]
actions = [{ type = "systemd-cpu-quota", percent = 10 }]

[tiers.nap.busy]
actions = [{ type = "systemd-cpu-quota", percent = 50 }]
"#,
        )
        .unwrap();

        assert_eq!(config.cpu_load_polling.interval_ms, 5000);
        assert_eq!(config.cpu_load_polling.ttl_idle, 3);
        // Unset keys keep defaults.
        assert!((config.cpu_load_polling.idle_threshold - 0.10).abs() < f64::EPSILON);

        let background_idle = config.tiers.background.idle.unwrap();
        assert_eq!(background_idle.actions, vec![Action::Ecore]);
        let nap_busy = config.tiers.nap.busy.unwrap();
        assert_eq!(
            nap_busy.actions,
            vec![Action::SystemdCpuQuota { percent: 50 }]
        );
    }

    #[test]
    fn load_variants_are_optional() {
        let config: Config = toml::from_str("").unwrap();
        assert!(config.tiers.background.idle.is_none());
        assert!(config.tiers.nap.busy.is_none());
    }

    #[test]
    fn example_config_parses() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/example/app-nap.toml");
        let content = std::fs::read_to_string(path).unwrap();
        let config: Config = toml::from_str(&content).unwrap();
        assert!(config.tiers.background.idle.is_some());
        assert!(config.tiers.nap.busy.is_some());
    }
}
