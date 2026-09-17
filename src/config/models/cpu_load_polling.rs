use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CpuLoadPollingConfig {
    pub interval_ms: u64,
    pub idle: CpuLoadPollingIdleConfig,
    pub busy: CpuLoadPollingBusyConfig,
}
impl Default for CpuLoadPollingConfig {
    fn default() -> Self {
        Self {
            interval_ms: 10000,
            idle: CpuLoadPollingIdleConfig::default(),
            busy: CpuLoadPollingBusyConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
/// Less than this usage AND throttle level is considered idle.
pub struct CpuLoadPollingIdleConfig {
    pub usage_thres: f64,
    pub throttle_thres: f64,
    pub confirm_ticks: u64,
}
impl Default for CpuLoadPollingIdleConfig {
    fn default() -> Self {
        Self {
            usage_thres: 0.10,
            throttle_thres: 0.01,
            confirm_ticks: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
/// More than this usage OR throttle level is considered busy.
pub struct CpuLoadPollingBusyConfig {
    pub usage_thres: f64,
    pub throttle_thres: f64,
    pub confirm_ticks: u64,
}
impl Default for CpuLoadPollingBusyConfig {
    fn default() -> Self {
        Self {
            usage_thres: 0.20,
            throttle_thres: 0.05,
            confirm_ticks: 1,
        }
    }
}
