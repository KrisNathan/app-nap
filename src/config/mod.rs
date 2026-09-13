use std::path::PathBuf;

pub mod models;

use models::Config;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("cannot read config file {0}: {1}")]
    Read(PathBuf, #[source] std::io::Error),
    #[error("cannot parse config file: {0}")]
    Parse(#[from] toml::de::Error),
}

pub fn load_config_from_file(config_path: PathBuf) -> Result<Config, ConfigError> {
    let text = std::fs::read_to_string(&config_path)
        .map_err(|e| ConfigError::Read(config_path.clone(), e))?;
    let config: Config = toml::from_str(&text)?;
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

    #[test]
    fn loads_example_config() {
        load_config_from_file(PathBuf::from("example/config.toml")).unwrap();
    }
}
