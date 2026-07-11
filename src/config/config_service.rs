use super::{Config, ConfigError};

pub trait ConfigService {
    fn new() -> Self
    where
        Self: Sized;

    fn load(&mut self) -> Result<(), ConfigError>;

    fn get_config(&self) -> &Config;
}
