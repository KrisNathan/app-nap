mod action;
mod config;
mod daemon;
mod inhibit;
mod media;
mod systemd;

use std::{error::Error, future::pending};

use crate::{
    config::{ConfigService, toml::TomlConfigService},
    daemon::{dbus::DBusDaemon, service::Daemon, tier_policy_set::TierPolicySet},
    inhibit::kde::KdeInhibitService,
    media::mpris::MprisMediaService,
    systemd::dbus_client::SystemdDbusClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut config_service = TomlConfigService::new();
    config_service.load()?;

    let dbus_conn = zbus::Connection::session().await?; // it's Arc under the hood so .clone is ok apparently
    let systemd_client = SystemdDbusClient::new(dbus_conn.clone());
    let tier_policies = TierPolicySet::from_config(config_service.get_config(), systemd_client);
    let inhibit_service = KdeInhibitService::new(dbus_conn.clone());
    let media_service = MprisMediaService::new(dbus_conn.clone());

    let daemon = Daemon::new(tier_policies, inhibit_service, media_service);
    let dbus_daemon = DBusDaemon::new(daemon);

    dbus_conn
        .object_server()
        .at("/dev/appnap/AppNap", dbus_daemon)
        .await?;
    dbus_conn.request_name("dev.appnap.AppNap").await?;

    pending::<()>().await;
    Ok(())
}
