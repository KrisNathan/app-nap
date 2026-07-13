pub mod mpris;

use std::future::Future;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MediaError {
    #[error(transparent)]
    Dbus(#[from] zbus::Error),

    #[error("failed to resolve cgroup pids: {0}")]
    Cgroup(#[from] std::io::Error),
}

pub trait MediaService: Send + Sync {
    /// True if any MPRIS player whose owner pid lives in `cgroups` is Playing.
    fn is_playing(
        &self,
        cgroups: &[String],
    ) -> impl Future<Output = Result<bool, MediaError>> + Send;
}
