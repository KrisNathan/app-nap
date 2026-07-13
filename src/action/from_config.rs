use log::warn;

use crate::action::systemd::cpu_quota_from_percent;
use crate::action::{Action, ECoreAction};
use crate::config::model::Action as ActionConfig;

pub fn from_config(config: &[ActionConfig]) -> Vec<Action> {
    config
        .iter()
        .filter_map(|action| match action {
            ActionConfig::Signal => Some(Action::Signal),
            ActionConfig::Ecore => match ECoreAction::new() {
                Ok(action) => Some(Action::Ecore(Box::new(action))),
                Err(err) => {
                    warn!("skipping ecore action: {err}");
                    None
                }
            },
            ActionConfig::SystemdFreeze => Some(Action::SystemdFreeze),
            ActionConfig::SystemdCpuQuota { percent } => Some(Action::SystemdCpuQuota {
                quota: cpu_quota_from_percent(*percent),
            }),
            ActionConfig::SystemdCpuWeight { weight } => {
                Some(Action::SystemdCpuWeight { weight: *weight })
            }
        })
        .collect()
}
