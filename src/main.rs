mod conf_service;
mod daemon;
mod inhibit_service;
mod media_service;
mod nap_backend;
mod systemd;

use std::{error::Error, future::pending, sync::Arc};

use conf_service::{ConfService, NapBackendType};
use zbus::connection;

use crate::{
    conf_service::TomlConfService,
    daemon::Daemon,
    inhibit_service::KdeInhibitService,
    media_service::MprisMediaService,
    nap_backend::{
        ECoreBackend, NapBackend, SystemSignalController, SystemdFreezeBackend,
        SystemdScopeQuotaBackend,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut conf_service = TomlConfService::new();
    conf_service.load()?;
    let conf = conf_service.get_conf()?.clone();

    let nap_backend: Arc<dyn NapBackend> = match conf.nap_backend_type {
        NapBackendType::SystemdScope => Arc::new(SystemdScopeQuotaBackend::new()?),
        NapBackendType::SystemdFreeze => Arc::new(SystemdFreezeBackend::new()?),
        NapBackendType::Signal => Arc::new(SystemSignalController),
        NapBackendType::ECore => Arc::new(ECoreBackend::new()?),
    };

    let media_service = MprisMediaService::new().await?;
    let inhibit_service = KdeInhibitService::new().await?;
    let daemon = Daemon::new(
        nap_backend,
        Arc::new(media_service),
        Arc::new(inhibit_service),
    );
    let _conn = connection::Builder::session()?
        .name("dev.appnap.AppNap")?
        .serve_at("/dev/appnap/AppNap", daemon)?
        .build()
        .await?;

    pending::<()>().await;
    Ok(())
}
