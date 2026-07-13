use std::{fs, io, path::PathBuf};

use crate::config::{Config, ConfigError, ConfigService};

pub struct TomlConfigService {
    config_path: PathBuf,
    config: Config,
}

impl ConfigService for TomlConfigService {
    fn new() -> Self {
        let home = std::env::var_os("HOME").unwrap_or_else(|| std::ffi::OsString::from("."));
        let config_path = PathBuf::from(home)
            .join(".config")
            .join("app-nap")
            .join("app-nap.toml");

        Self {
            config_path,
            config: Config::default(),
        }
    }

    fn load(&mut self) -> Result<(), ConfigError> {
        let content = match fs::read_to_string(&self.config_path) {
            Ok(content) => content,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                self.config = Config::default();
                return Ok(());
            }
            Err(err) => return Err(err.into()),
        };

        self.config = toml::from_str(&content)?;
        Ok(())
    }

    fn get_config(&self) -> &Config {
        &self.config
    }
}
