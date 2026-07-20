use crate::config::{config_error::ConfigError, model::Config};

pub mod config_error;
pub mod model;
pub mod toml;

pub trait ConfigService {
    fn new() -> Self
    where
        Self: Sized;

    fn load(&mut self) -> Result<(), ConfigError>;

    fn get_config(&self) -> &Config;
}
