use std::path::PathBuf;

pub mod error;
pub mod models;

use models::Config;

use crate::config::error::ConfigError;

pub fn load_config_from_file(config_path: PathBuf) -> Result<Config, ConfigError> {
    let text = std::fs::read_to_string(&config_path)
        .map_err(|e| ConfigError::Read(config_path.clone(), e))?;
    let config: Config = toml::from_str(&text)?;
    config.validate()?;
    Ok(config)
}

pub fn load_config() -> Result<Config, ConfigError> {
    let home = std::env::var_os("HOME").unwrap_or_else(|| std::ffi::OsString::from("."));
    let config_path = PathBuf::from(home)
        .join(".config")
        .join("app-nap")
        .join("config.toml");
    load_config_from_file(config_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::models::action::ActionConfig;

    #[test]
    fn loads_example_config() {
        let config = load_config_from_file(PathBuf::from("example/config.toml")).unwrap();

        assert_eq!(config.cpu_load_polling.interval_ms, 10000);
        assert_eq!(config.cpu_load_polling.idle.usage_thres, 0.10);
        assert_eq!(config.cpu_load_polling.idle.throttle_thres, 0.01);
        assert_eq!(config.cpu_load_polling.idle.confirm_ticks, 2);
        assert_eq!(config.cpu_load_polling.busy.usage_thres, 0.20);
        assert_eq!(config.cpu_load_polling.busy.throttle_thres, 0.05);
        assert_eq!(config.cpu_load_polling.busy.confirm_ticks, 1);

        assert_eq!(
            config.policies.performance.actions,
            vec![ActionConfig::SystemdCpuWeight { weight: 100 }]
        );
        assert_eq!(
            config.policies.background_busy.actions,
            vec![ActionConfig::SystemdCpuWeight { weight: 1 }]
        );
        assert_eq!(
            config.policies.background_idle.actions,
            vec![
                ActionConfig::SystemdCpuWeight { weight: 1 },
                ActionConfig::Ecore,
            ]
        );
        assert_eq!(
            config.policies.nap_idle.actions,
            vec![
                ActionConfig::SystemdCpuQuota { percent: 10 },
                ActionConfig::Ecore,
            ]
        );
        assert_eq!(
            config.policies.nap_busy.actions,
            vec![
                ActionConfig::SystemdCpuQuota { percent: 50 },
                ActionConfig::Ecore,
            ]
        );
    }

    #[test]
    fn rejects_unknown_config_keys() {
        assert!(toml::from_str::<Config>("[tiers.performance]\nactions = []\n").is_err());
        assert!(toml::from_str::<Config>("[cpu_load_polling.idle]\nmin_usage = 0.10\n").is_err());
    }
}
