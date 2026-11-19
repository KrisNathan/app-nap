use serde::{Deserialize, Serialize};

use crate::config::{
    error::ConfigValidationError,
    models::{cpu_load_polling::CpuLoadPollingConfig, policies::PoliciesConfig},
};

pub mod action;
pub mod cpu_load_polling;
pub mod policies;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    type ThresholdSetter = fn(&mut Config, f64);

    fn assert_must_be_positive(config: &Config, expected_field: &str) {
        match config.validate() {
            Err(ConfigValidationError::MustBePositive { field, found }) => {
                assert_eq!(field, expected_field);
                assert_eq!(found, 0);
            }
            result => panic!("expected MustBePositive for {expected_field}, got {result:?}"),
        }
    }

    fn assert_invalid_threshold(config: &Config, expected_field: &str) {
        match config.validate() {
            Err(ConfigValidationError::InvalidThreshold { field, .. }) => {
                assert_eq!(field, expected_field);
            }
            result => panic!("expected InvalidThreshold for {expected_field}, got {result:?}"),
        }
    }

    fn assert_invalid_order(config: &Config, expected_lower: &str, expected_upper: &str) {
        match config.validate() {
            Err(ConfigValidationError::InvalidThresholdOrder {
                lower_field,
                upper_field,
                ..
            }) => {
                assert_eq!(lower_field, expected_lower);
                assert_eq!(upper_field, expected_upper);
            }
            result => panic!(
                "expected InvalidThresholdOrder for {expected_lower} and {expected_upper}, got {result:?}"
            ),
        }
    }

    #[test]
    fn accepts_default_config() {
        assert!(Config::default().validate().is_ok());
    }

    #[test]
    fn rejects_zero_interval_ms() {
        let mut config = Config::default();
        config.cpu_load_polling.interval_ms = 0;

        assert_must_be_positive(&config, "cpu_load_polling.interval_ms");
    }

    #[test]
    fn rejects_zero_idle_confirm_ticks() {
        let mut config = Config::default();
        config.cpu_load_polling.idle.confirm_ticks = 0;

        assert_must_be_positive(&config, "cpu_load_polling.idle.confirm_ticks");
    }

    #[test]
    fn rejects_zero_busy_confirm_ticks() {
        let mut config = Config::default();
        config.cpu_load_polling.busy.confirm_ticks = 0;

        assert_must_be_positive(&config, "cpu_load_polling.busy.confirm_ticks");
    }

    #[test]
    fn rejects_invalid_thresholds() {
        let cases: [(&str, ThresholdSetter); 4] = [
            ("cpu_load_polling.idle.usage_thres", |config, value| {
                config.cpu_load_polling.idle.usage_thres = value;
            }),
            ("cpu_load_polling.busy.usage_thres", |config, value| {
                config.cpu_load_polling.busy.usage_thres = value;
            }),
            ("cpu_load_polling.idle.throttle_thres", |config, value| {
                config.cpu_load_polling.idle.throttle_thres = value;
            }),
            ("cpu_load_polling.busy.throttle_thres", |config, value| {
                config.cpu_load_polling.busy.throttle_thres = value;
            }),
        ];

        for (field, set) in cases {
            for value in [-0.1, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                let mut config = Config::default();
                set(&mut config, value);
                assert_invalid_threshold(&config, field);
            }
        }
    }

    #[test]
    fn rejects_equal_and_reversed_usage_thresholds() {
        for (idle, busy) in [(0.2, 0.2), (0.3, 0.2)] {
            let mut config = Config::default();
            config.cpu_load_polling.idle.usage_thres = idle;
            config.cpu_load_polling.busy.usage_thres = busy;

            assert_invalid_order(
                &config,
                "cpu_load_polling.idle.usage_thres",
                "cpu_load_polling.busy.usage_thres",
            );
        }
    }

    #[test]
    fn rejects_equal_and_reversed_throttle_thresholds() {
        for (idle, busy) in [(0.05, 0.05), (0.06, 0.05)] {
            let mut config = Config::default();
            config.cpu_load_polling.idle.throttle_thres = idle;
            config.cpu_load_polling.busy.throttle_thres = busy;

            assert_invalid_order(
                &config,
                "cpu_load_polling.idle.throttle_thres",
                "cpu_load_polling.busy.throttle_thres",
            );
        }
    }

    #[test]
    fn accepts_multicore_usage_thresholds() {
        let mut config = Config::default();
        config.cpu_load_polling.idle.usage_thres = 2.0;
        config.cpu_load_polling.busy.usage_thres = 4.0;

        assert!(config.validate().is_ok());
    }
}
