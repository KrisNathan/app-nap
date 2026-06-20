use std::future::Future;

// `who` strings reported by the inhibit portal (e.g. "firefox", "obs"). The
// daemon maps these to tracked pids by comparing against `/proc/<pid>/comm`.
pub trait InhibitService: Send + Sync {
    fn list_inhibitors(&self) -> impl Future<Output = Result<Vec<String>, zbus::Error>> + Send;
}

mod kde_inhibit_service;
pub use kde_inhibit_service::KdeInhibitService;
