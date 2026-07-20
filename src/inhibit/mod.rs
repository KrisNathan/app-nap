pub mod kde;

use std::future::Future;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum InhibitError {
    #[error(transparent)]
    Dbus(#[from] zbus::Error),
}

pub trait InhibitService: Send + Sync {
    /// True if any idle inhibitor's `who` matches a substring of one of `cgroups`.
    fn is_inhibiting(
        &self,
        cgroups: &[String],
    ) -> impl Future<Output = Result<bool, InhibitError>> + Send;
}
