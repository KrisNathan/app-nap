use libc::pid_t;
use std::{collections::HashMap, error::Error, future::pending};
use zbus::{connection, interface};

struct Window {
    id: String,
    pid: pid_t,
    minimized: bool,
}

struct Process {
    pid: pid_t,
    windows: HashMap<String, Window>,
    is_napping: bool,
}

struct Daemon {
    processes: HashMap<pid_t, Process>,
}

#[interface(name = "dev.appnap.AppNap1")]
impl Daemon {
    fn add_window(&mut self, window_id: &str, pid: pid_t) -> String {
        let buf = format!("AddWindow: window_id={} pid={}", window_id, pid);
        println!("{}", buf);
        buf
    }
    fn remove_window(&self, window_id: &str, pid: pid_t) -> String {
        let buf = format!("RemoveWindow: window_id={} pid={}", window_id, pid);
        println!("{}", buf);
        buf
    }
    fn minimized_changed(&mut self, window_id: &str, pid: pid_t, minimized: bool) -> String {
        let buf = format!(
            "MinimizedChanged: window_id={} pid={} minimized={}",
            window_id, pid, minimized
        );
        println!("{}", buf);
        buf
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let daemon = Daemon {
        processes: HashMap::new(),
    };
    let _conn = connection::Builder::session()?
        .name("dev.appnap.AppNap")?
        .serve_at("/dev/appnap/AppNap", daemon)?
        .build()
        .await?;

    pending::<()>().await;
    Ok(())
}
