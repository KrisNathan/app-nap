pub mod mpris;

use std::future::Future;

pub trait MediaService: Send + Sync {
    /// True if any MPRIS player whose owner pid lives in `cgroups` is Playing.
    fn is_playing(&self, cgroups: &[String]) -> impl Future<Output = bool> + Send;
}
