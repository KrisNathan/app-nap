mod ecore;
mod from_config;
mod signal;
mod systemd;

pub use ecore::ECoreAction;
pub use from_config::from_config;

use std::io;

use thiserror::Error;

use crate::systemd::{dbus_client::SystemdDbusClient, error::SystemdError};

#[derive(Debug, Error)]
pub enum ActionError {
    #[error("signal action failed: {0}")]
    Signal(#[source] io::Error),

    #[error("E-core action failed: {0}")]
    Ecore(#[source] io::Error),

    #[error("systemd freeze action failed: {0}")]
    SystemdFreeze(#[source] SystemdError),

    #[error("systemd CPU quota action failed: {0}")]
    SystemdCpuQuota(#[source] SystemdError),

    #[error("systemd CPU weight action failed: {0}")]
    SystemdCpuWeight(#[source] SystemdError),
}

pub enum Action {
    Signal,
    Ecore(Box<ECoreAction>),
    SystemdFreeze,
    SystemdCpuQuota { quota: u64 },
    SystemdCpuWeight { weight: u64 },
}

impl Action {
    pub async fn apply(
        &self,
        cgroups: &[String],
        systemd: &SystemdDbusClient,
    ) -> Result<(), ActionError> {
        match self {
            Self::Signal => signal::stop(cgroups).map_err(ActionError::Signal),
            Self::Ecore(action) => action.apply(cgroups).map_err(ActionError::Ecore),
            Self::SystemdFreeze => systemd::freeze(systemd, cgroups)
                .await
                .map_err(ActionError::SystemdFreeze),
            Self::SystemdCpuQuota { quota } => systemd::set_cpu_quota(systemd, cgroups, *quota)
                .await
                .map_err(ActionError::SystemdCpuQuota),
            Self::SystemdCpuWeight { weight } => systemd::set_cpu_weight(systemd, cgroups, *weight)
                .await
                .map_err(ActionError::SystemdCpuWeight),
        }
    }

    pub async fn revert(
        &self,
        cgroups: &[String],
        systemd: &SystemdDbusClient,
    ) -> Result<(), ActionError> {
        match self {
            Self::Signal => signal::cont(cgroups).map_err(ActionError::Signal),
            Self::Ecore(action) => action.revert(cgroups).map_err(ActionError::Ecore),
            Self::SystemdFreeze => systemd::thaw(systemd, cgroups)
                .await
                .map_err(ActionError::SystemdFreeze),
            Self::SystemdCpuQuota { .. } => systemd::unset_cpu_quota(systemd, cgroups)
                .await
                .map_err(ActionError::SystemdCpuQuota),
            Self::SystemdCpuWeight { .. } => systemd::reset_cpu_weight(systemd, cgroups)
                .await
                .map_err(ActionError::SystemdCpuWeight),
        }
    }
}
