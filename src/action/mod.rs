mod ecore;
mod from_config;
mod signal;
mod systemd;

pub use ecore::ECoreAction;
pub use from_config::from_config;

use crate::systemd::dbus_client::SystemdDbusClient;

pub enum Action {
    Signal,
    Ecore(Box<ECoreAction>),
    SystemdFreeze,
    SystemdCpuQuota { quota: u64 },
    SystemdCpuWeight { weight: u64 },
}

impl Action {
    pub async fn apply(&self, cgroups: &[String], systemd: &SystemdDbusClient) {
        match self {
            Self::Signal => signal::stop(cgroups),
            Self::Ecore(action) => action.apply(cgroups),
            Self::SystemdFreeze => systemd::freeze(systemd, cgroups).await,
            Self::SystemdCpuQuota { quota } => {
                systemd::set_cpu_quota(systemd, cgroups, *quota).await
            }
            Self::SystemdCpuWeight { weight } => {
                systemd::set_cpu_weight(systemd, cgroups, *weight).await
            }
        }
    }

    pub async fn revert(&self, cgroups: &[String], systemd: &SystemdDbusClient) {
        match self {
            Self::Signal => signal::cont(cgroups),
            Self::Ecore(action) => action.revert(cgroups),
            Self::SystemdFreeze => systemd::thaw(systemd, cgroups).await,
            Self::SystemdCpuQuota { .. } => systemd::unset_cpu_quota(systemd, cgroups).await,
            Self::SystemdCpuWeight { .. } => systemd::reset_cpu_weight(systemd, cgroups).await,
        }
    }
}
