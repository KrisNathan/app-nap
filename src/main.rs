mod conf_service;
mod daemon;
mod media_service;
mod nap_backend;

use std::{error::Error, future::pending, sync::Arc};

use conf_service::{ConfService, NapBackendType};
use zbus::connection;

use crate::{
    conf_service::TomlConfService,
    daemon::Daemon,
    media_service::MprisMediaService,
    nap_backend::{
        NapBackend, SystemSignalController, SystemdFreezeBackend, SystemdScopeQuotaBackend,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut conf_service = TomlConfService::new();
    conf_service.load()?;
    let conf = conf_service.get_conf()?.clone();

    let nap_backend: Arc<dyn NapBackend> = match conf.nap_backend_type {
        NapBackendType::SystemdScope => Arc::new(SystemdScopeQuotaBackend),
        NapBackendType::SystemdFreeze => Arc::new(SystemdFreezeBackend),
        NapBackendType::Signal => Arc::new(SystemSignalController),
    };

    let media_service = MprisMediaService::new().await?;
    let daemon = Daemon::new(nap_backend, Arc::new(media_service));
    let _conn = connection::Builder::session()?
        .name("dev.appnap.AppNap")?
        .serve_at("/dev/appnap/AppNap", daemon)?
        .build()
        .await?;

    pending::<()>().await;
    Ok(())
}
