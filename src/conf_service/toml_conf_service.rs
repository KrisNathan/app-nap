use std::{fs, io, path::PathBuf};

use crate::conf_service::{Conf, ConfService, DefaultConfService};

pub struct TomlConfService {
    config_path: PathBuf,
    fallback: DefaultConfService,
    loaded_conf: Option<Conf>,
}

impl ConfService for TomlConfService {
    fn new() -> Self {
        let home = std::env::var_os("HOME").unwrap_or(std::ffi::OsString::from("."));
        let config_path = PathBuf::from(home)
            .join(".config")
            .join("app-nap")
            .join("app-nap.toml");
        Self {
            config_path: config_path,
            fallback: DefaultConfService::new(),
            loaded_conf: None,
        }
    }

    fn get_conf(&self) -> Result<&Conf, std::io::Error> {
        match self.loaded_conf {
            Some(ref conf) => Ok(conf),
            None => self.fallback.get_conf(),
        }
    }
}

impl TomlConfService {
    pub fn load(&mut self) -> Result<(), std::io::Error> {
        let content = match fs::read_to_string(&self.config_path) {
            Ok(c) => c,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                self.loaded_conf = None;
                return Ok(());
            }
            Err(e) => {
                return Err(e);
            }
        };
        let conf = toml::from_str::<Conf>(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.loaded_conf = Some(conf);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conf_service::conf::NapBackendType;

    #[test]
    fn load_maps_nap_backend_type_from_string() {
        let config_path =
            std::env::temp_dir().join(format!("app-nap-conf-test-{}.toml", std::process::id()));
        fs::write(&config_path, r#"nap_backend_type = "systemd-scope""#).unwrap();

        let mut service = TomlConfService {
            config_path: config_path.clone(),
            fallback: DefaultConfService::new(),
            loaded_conf: None,
        };

        service.load().unwrap();

        assert_eq!(
            service.get_conf().unwrap().nap_backend_type,
            NapBackendType::SystemdScope
        );

        fs::remove_file(config_path).unwrap();
    }
}
