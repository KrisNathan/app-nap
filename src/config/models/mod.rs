use serde::{Deserialize, Serialize};

use crate::config::{
    error::ConfigValidationError,
    models::{cpu_load_polling::CpuLoadPollingConfig, policies::PoliciesConfig},
};

pub mod action;
pub mod cpu_load_polling;
pub mod policies;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub cpu_load_polling: CpuLoadPollingConfig,

    #[serde(default)]
    pub policies: PoliciesConfig,
}

impl Config {
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if self.cpu_load_polling.interval_ms == 0 {
            return Err(ConfigValidationError::MustBePositive {
                field: "cpu_load_polling.interval_ms",
                found: self.cpu_load_polling.interval_ms,
            });
        }

        if self.cpu_load_polling.idle.confirm_ticks == 0 {
            return Err(ConfigValidationError::MustBePositive {
                field: "cpu_load_polling.idle.confirm_ticks",
                found: self.cpu_load_polling.idle.confirm_ticks,
            });
        }

        if self.cpu_load_polling.busy.confirm_ticks == 0 {
            return Err(ConfigValidationError::MustBePositive {
                field: "cpu_load_polling.busy.confirm_ticks",
                found: self.cpu_load_polling.busy.confirm_ticks,
            });
        }

        if self.cpu_load_polling.idle.usage_thres < 0.0
            || !self.cpu_load_polling.idle.usage_thres.is_finite()
        {
            return Err(ConfigValidationError::InvalidThreshold {
                field: "cpu_load_polling.idle.usage_thres",
                found: self.cpu_load_polling.idle.usage_thres,
            });
        }

        if self.cpu_load_polling.busy.usage_thres < 0.0
            || !self.cpu_load_polling.busy.usage_thres.is_finite()
        {
            return Err(ConfigValidationError::InvalidThreshold {
                field: "cpu_load_polling.busy.usage_thres",
                found: self.cpu_load_polling.busy.usage_thres,
            });
        }

        if self.cpu_load_polling.idle.throttle_thres < 0.0
            || !self.cpu_load_polling.idle.throttle_thres.is_finite()
        {
            return Err(ConfigValidationError::InvalidThreshold {
                field: "cpu_load_polling.idle.throttle_thres",
                found: self.cpu_load_polling.idle.throttle_thres,
            });
        }

        if self.cpu_load_polling.busy.throttle_thres < 0.0
            || !self.cpu_load_polling.busy.throttle_thres.is_finite()
        {
            return Err(ConfigValidationError::InvalidThreshold {
                field: "cpu_load_polling.busy.throttle_thres",
                found: self.cpu_load_polling.busy.throttle_thres,
            });
        }

        if self.cpu_load_polling.idle.usage_thres >= self.cpu_load_polling.busy.usage_thres {
            return Err(ConfigValidationError::InvalidThresholdOrder {
                lower_field: "cpu_load_polling.idle.usage_thres",
                lower: self.cpu_load_polling.idle.usage_thres,
                upper_field: "cpu_load_polling.busy.usage_thres",
                upper: self.cpu_load_polling.busy.usage_thres,
            });
        }

        if self.cpu_load_polling.idle.throttle_thres >= self.cpu_load_polling.busy.throttle_thres {
            return Err(ConfigValidationError::InvalidThresholdOrder {
                lower_field: "cpu_load_polling.idle.throttle_thres",
                lower: self.cpu_load_polling.idle.throttle_thres,
                upper_field: "cpu_load_polling.busy.throttle_thres",
                upper: self.cpu_load_polling.busy.throttle_thres,
            });
        }

        Ok(())
    }
}
