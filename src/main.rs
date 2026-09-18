mod actions;
mod cgroup;
mod config;
mod daemon;
mod dbus;
mod inhibitor;

use std::{error::Error, time::Duration};

use tokio::sync::mpsc;

use crate::{
    config::load_config,
    daemon::{Daemon, EventLoop, channel_event::ChannelEvent},
    dbus::client::mpris::MprisDBusProxy,
    dbus::client::powerdevil::PowerDevilDBusProxy,
    dbus::server::DBusDaemon,
    inhibitor::{mpris::MprisWatcher, powerdevil::PowerDevilInhibitor},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut logger =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    logger.target(env_logger::Target::Stdout).init();

    let conf = load_config().unwrap(); // pray

    let dbus_conn = zbus::Connection::session().await?; // it's Arc under the hood so .clone is to be expected
    let (tx, rx) = mpsc::channel::<ChannelEvent>(32);

    let powerdevil =
        PowerDevilInhibitor::new(PowerDevilDBusProxy::new(&dbus_conn).await?, tx.clone());
    tokio::spawn(async move { powerdevil.watch().await });

    let mpris = MprisWatcher::new(MprisDBusProxy::new(dbus_conn.clone()).await?, tx.clone());
    tokio::spawn(async move {
        if let Err(e) = mpris.watch().await {
            log::warn!("mpris watcher exited: {e}");
        }
    });

    let daemon = Daemon::new(&conf, &dbus_conn).await?;
    let mut event_loop = EventLoop::new(
        daemon,
        Duration::from_millis(conf.cpu_load_polling.interval_ms),
        rx,
    );

    let dbus_daemon = DBusDaemon::new(tx);

    dbus_conn
        .object_server()
        .at("/dev/appnap/AppNap", dbus_daemon)
        .await?;
    dbus_conn.request_name("dev.appnap.AppNap").await?;

    event_loop.serve().await;
    Ok(())
}
