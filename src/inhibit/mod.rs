pub mod kde;

use std::future::Future;

pub trait InhibitService: Send + Sync {
    /// True if any idle inhibitor's `who` matches a substring of one of `cgroups`.
    fn is_inhibiting(&self, cgroups: &[String]) -> impl Future<Output = bool> + Send;
}
