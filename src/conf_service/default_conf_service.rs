use crate::conf_service::{Conf, ConfService};

static DEFAULT_CONF: Conf = Conf {
    nap_backend_type: super::conf::NapBackendType::SystemdCPUQuota,
};

pub struct DefaultConfService {}

impl ConfService for DefaultConfService {
    fn new() -> Self {
        Self {}
    }

    fn get_conf(&self) -> Result<&Conf, std::io::Error> {
        Ok(&DEFAULT_CONF)
    }
}
