mod conf;
mod default_conf_service;
mod toml_conf_service;

pub use conf::{Conf, NapBackendType};

pub trait ConfService {
    fn new() -> Self
    where
        Self: Sized;
    fn get_conf(&self) -> Result<&Conf, std::io::Error>;
}

pub use default_conf_service::DefaultConfService;
pub use toml_conf_service::TomlConfService;
