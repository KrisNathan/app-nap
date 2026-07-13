mod action;
mod config;
mod daemon;
mod inhibit;
mod media;
mod systemd;

use std::{error::Error, sync::Arc};

use crate::{
    config::{ConfigService, toml::TomlConfigService},
    daemon::{daemon::Daemon, dbus::DBusDaemon, tier_policy_set::TierPolicySet},
    inhibit::kde::KdeInhibitService,
    media::mpris::MprisMediaService,
    systemd::dbus_client::SystemdDbusClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut config_service = TomlConfigService::new();
    config_service.load()?;

    let dbus_conn = zbus::connection::Connection::session().await?;
    let systemd_client = Arc::new(SystemdDbusClient::new(dbus_conn.clone().into()));
    let tier_policies = TierPolicySet::from_config(config_service.get_config(), systemd_client);
    let inhibit_service = KdeInhibitService::new(dbus_conn.clone());
    let media_service = MprisMediaService::new(dbus_conn.clone());

    let daemon = Daemon::new(tier_policies, inhibit_service, media_service);
    let dbus_daemon = DBusDaemon::new(daemon);
    let _ = zbus::connection::Builder::session()?
        .name("dev.appnap.AppNap")?
        .serve_at("/dev/appnap/AppNap", dbus_daemon)?
        .build()
        .await?;

    Ok(())
}
