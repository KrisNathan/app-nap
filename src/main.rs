use libc::pid_t;
use std::{error::Error, future::pending};
use zbus::{connection, interface};

struct Daemon {}

#[interface(name = "dev.appnap.AppNap1")]
impl Daemon {
    fn update_window(&self, window_id: &str, pid: pid_t, active: bool) -> String {
        let buf = format!(
            "update_window: window_id={} pid={} active={}",
            window_id, pid, active
        );
        println!("{}", buf);
        buf
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let daemon = Daemon {};
    let _conn = connection::Builder::session()?
        .name("dev.appnap.AppNap")?
        .serve_at("/dev/appnap/AppNap", daemon)?
        .build()
        .await?;

    pending::<()>().await;
    Ok(())
}
