use serde::{Deserialize, Serialize};

use crate::config::models::{cpu_load_polling::CpuLoadPollingConfig, policies::PoliciesConfig};

pub mod action;
pub mod cpu_load_polling;
pub mod cpu_sample;
pub mod policies;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub cpu_load_polling: CpuLoadPollingConfig,

    #[serde(default)]
    pub tiers: PoliciesConfig,
}
