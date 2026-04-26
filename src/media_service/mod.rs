use libc::pid_t;
use std::future::Future;

pub trait MediaService: Send + Sync {
    fn list_playing_media_pids(
        &self,
    ) -> impl Future<Output = Result<Vec<pid_t>, zbus::Error>> + Send;
}

mod mpris_media_service;
pub use mpris_media_service::MprisMediaService;
