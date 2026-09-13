use crate::{
    actions::ecore::EcoreAction, cgroup::Cgroup, config::models::action::ActionConfig,
    dbus::client::systemd::SystemdDBusProxy,
};

use thiserror::Error;
use tokio::io;

mod ecore;
mod systemd_cpu_quota;
mod systemd_cpu_weight;
mod systemd_freeze;

#[derive(Debug, Error)]
pub enum ActionError {
    #[error("signal action failed: {0}")]
    Signal(#[source] io::Error),

    #[error("E-core action failed: {0}")]
    Ecore(#[source] io::Error),

    #[error("systemd freeze action failed: {0}")]
    SystemdFreeze(#[source] zbus::Error),

    #[error("systemd CPU quota action failed: {0}")]
    SystemdCpuQuota(#[source] zbus::Error),

    #[error("systemd CPU weight action failed: {0}")]
    SystemdCpuWeight(#[source] zbus::Error),
}

// cpu_set_t is 128 bytes, so Ecore dwarfs the other variants; the enum has
// few long-lived instances, so the size doesn't matter.
#[allow(clippy::large_enum_variant)]
pub enum Action {
    Ecore {
        ecore: Box<EcoreAction>,
    },
    SystemdFreeze {
        systemd: SystemdDBusProxy<'static>,
    },
    SystemdCpuQuota {
        systemd: SystemdDBusProxy<'static>,
        percent: u64,
    },
    SystemdCpuWeight {
        systemd: SystemdDBusProxy<'static>,
        weight: u64,
    },
}

impl Action {
    pub async fn from_config(config: &ActionConfig, conn: &zbus::Connection) -> zbus::Result<Self> {
        Ok(match config {
            ActionConfig::Ecore => Action::Ecore {
                ecore: EcoreAction::new().unwrap().into(),
            },
            ActionConfig::SystemdFreeze => Action::SystemdFreeze {
                systemd: SystemdDBusProxy::new(conn).await?,
            },
            ActionConfig::SystemdCpuQuota { percent } => Action::SystemdCpuQuota {
                systemd: SystemdDBusProxy::new(conn).await?,
                percent: *percent,
            },
            ActionConfig::SystemdCpuWeight { weight } => Action::SystemdCpuWeight {
                systemd: SystemdDBusProxy::new(conn).await?,
                weight: *weight,
            },
        })
    }

    pub async fn apply(&self, cgroup: &Cgroup) -> Result<(), ActionError> {
        match self {
            Action::Ecore { ecore } => ecore.apply(cgroup).map_err(ActionError::Ecore),
            Action::SystemdFreeze { systemd } => systemd_freeze::apply(cgroup, systemd)
                .await
                .map_err(ActionError::SystemdFreeze),
            Action::SystemdCpuQuota { systemd, percent } => systemd
                .set_unit_property(
                    cgroup.get_unit_name().as_str(),
                    "CPUQuotaPerSecUSec",
                    u64::from(*percent) * 10_000,
                )
                .await
                .map_err(ActionError::SystemdCpuQuota),
            Action::SystemdCpuWeight { systemd, weight } => systemd
                .set_unit_property(
                    cgroup.get_unit_name().as_str(),
                    "CPUWeight",
                    u64::from(*weight),
                )
                .await
                .map_err(ActionError::SystemdCpuWeight),
        }
    }

    pub async fn revert(&self, cgroup: &Cgroup) -> Result<(), ActionError> {
        match self {
            Action::Ecore { ecore } => ecore.revert(cgroup).map_err(ActionError::Ecore),
            Action::SystemdFreeze { systemd } => systemd_freeze::revert(cgroup, systemd)
                .await
                .map_err(ActionError::SystemdFreeze),
            Action::SystemdCpuQuota { .. } | Action::SystemdCpuWeight { .. } => Ok(()),
        }
    }
}
