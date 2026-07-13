pub mod ecore;
mod from_config;
pub mod signal;
pub mod systemd_cpu_quota;
pub mod systemd_cpu_weight;
pub mod systemd_freeze;

pub use from_config::from_config;

pub trait Action: Send + Sync {
    fn apply(&self, cgroups: &[String]);
    fn revert(&self, cgroups: &[String]);
}
