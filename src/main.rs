mod action;
mod config;
mod daemon;
mod inhibit;
mod media;
mod systemd;

use std::{error::Error, future::pending};
use tokio::sync::mpsc;

use crate::{
    config::{ConfigService, toml::TomlConfigService},
    daemon::{Daemon, channel_event::ChannelEvent, dbus::DBusDaemon, policies::Policies},
    inhibit::kde::KdeInhibitService,
    media::mpris::MprisMediaService,
    systemd::dbus_client::SystemdDbusClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut logger =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    logger.target(env_logger::Target::Stdout).init();

    let mut config_service = TomlConfigService::new();
    config_service.load()?;

    let dbus_conn = zbus::Connection::session().await?; // it's Arc under the hood so .clone is ok apparently
    let systemd_client = SystemdDbusClient::new(dbus_conn.clone());
    let policies = Policies::from_config(config_service.get_config(), systemd_client);
    let cpu_load_polling = config_service.get_config().cpu_load_polling.clone();
    let inhibit_service = KdeInhibitService::new(dbus_conn.clone());
    let media_service = MprisMediaService::new(dbus_conn.clone());

    let (tx, rx) = mpsc::channel::<ChannelEvent>(32);

    let mut daemon = Daemon::new(policies, inhibit_service, media_service, cpu_load_polling);
    tokio::spawn(async move {
        daemon.run(rx).await;
    });

    let dbus_daemon = DBusDaemon::new(tx.clone());

    dbus_conn
        .object_server()
        .at("/dev/appnap/AppNap", dbus_daemon)
        .await?;
    dbus_conn.request_name("dev.appnap.AppNap").await?;

    pending::<()>().await;
    Ok(())
}
