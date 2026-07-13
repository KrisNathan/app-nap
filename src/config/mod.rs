use crate::config::{config::Config, config_error::ConfigError};

pub mod config;
pub mod config_error;
pub mod toml;

pub trait ConfigService {
    fn new() -> Self
    where
        Self: Sized;

    fn load(&mut self) -> Result<(), ConfigError>;

    fn get_config(&self) -> &Config;
}
