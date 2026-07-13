use std::sync::Arc;

use log::warn;

use crate::action::Action;
use crate::action::ecore::ECoreAction;
use crate::action::signal::SignalAction;
use crate::action::systemd_cpu_quota::{SystemdCpuQuotaAction, cpu_quota_from_percent};
use crate::action::systemd_cpu_weight::SystemdCpuWeightAction;
use crate::action::systemd_freeze::SystemdFreezeAction;

use crate::config::config::Action as ActionConfig;
use crate::systemd::dbus_client::SystemdDbusClient;

pub fn from_config(
    config: &[ActionConfig],
    client: Arc<SystemdDbusClient>,
) -> Vec<Box<dyn Action>> {
    config
        .iter()
        .filter_map(|action| match action {
            ActionConfig::Signal => Some(Box::new(SignalAction {}) as Box<dyn Action>),
            ActionConfig::Ecore => match ECoreAction::new() {
                Ok(action) => Some(Box::new(action)),
                Err(err) => {
                    warn!("skipping ecore action: {err}");
                    None
                }
            },
            ActionConfig::SystemdFreeze => {
                Some(Box::new(SystemdFreezeAction::new(client.clone())))
            }
            ActionConfig::SystemdCpuQuota { percent } => Some(Box::new(SystemdCpuQuotaAction::new(
                client.clone(),
                cpu_quota_from_percent(*percent),
            ))),
            ActionConfig::SystemdCpuWeight { weight } => Some(Box::new(
                SystemdCpuWeightAction::new(client.clone(), *weight),
            )),
        })
        .collect()
}
