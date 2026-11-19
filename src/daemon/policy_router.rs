use crate::{
    actions::{Action, ActionError},
    cgroup::{UnitName, UnitPath},
    config::models::{action::ActionConfig, policies::PoliciesConfig},
    daemon::models::policy::Policy,
};
use futures::future::try_join_all;
use log::warn;

pub struct PolicyRouter {
    performance: Vec<Action>,
    background_idle: Vec<Action>,
    background_busy: Vec<Action>,
    nap_idle: Vec<Action>,
    nap_busy: Vec<Action>,
}

impl PolicyRouter {
    pub async fn from_config(
        config: &PoliciesConfig,
        conn: &zbus::Connection,
    ) -> zbus::Result<Self> {
        Ok(Self {
            performance: build_actions(&config.performance.actions, conn).await?,
            background_idle: build_actions(&config.background_idle.actions, conn).await?,
            background_busy: build_actions(&config.background_busy.actions, conn).await?,
            nap_idle: build_actions(&config.nap_idle.actions, conn).await?,
            nap_busy: build_actions(&config.nap_busy.actions, conn).await?,
        })
    }
}
async fn build_actions(
    actions: &[ActionConfig],
    conn: &zbus::Connection,
) -> zbus::Result<Vec<Action>> {
    try_join_all(
        actions
            .iter()
            .map(|action_conf| async move { Action::from_config(action_conf, conn).await }),
    )
    .await
}

impl PolicyRouter {
    fn map_policy_to_actions(&self, policy: Policy) -> &[Action] {
        match policy {
            Policy::Performance => &self.performance,
            Policy::BackgroundIdle => &self.background_idle,
            Policy::BackgroundBusy => &self.background_busy,
            Policy::NapIdle => &self.nap_idle,
            Policy::NapBusy => &self.nap_busy,
        }
    }

    pub async fn apply(
        &self,
        policy: Policy,
        unit_path: &UnitPath,
        unit_name: &UnitName,
    ) -> Result<(), ActionError> {
        let mut applied = Vec::new();

        for action in self.map_policy_to_actions(policy) {
            match action.apply(unit_path, unit_name).await {
                Ok(()) => applied.push(action),
                Err(error) => {
                    rollback_applied(&applied, unit_path, unit_name).await;
                    return Err(error);
                }
            }
        }

        Ok(())
    }

    pub async fn revert(
        &self,
        policy: Policy,
        unit_path: &UnitPath,
        unit_name: &UnitName,
    ) -> Result<(), ActionError> {
        let mut reverted = Vec::new();

        for action in self.map_policy_to_actions(policy).iter().rev() {
            match action.revert(unit_path, unit_name).await {
                Ok(()) => reverted.push(action),
                Err(error) => {
                    restore_reverted(&reverted, unit_path, unit_name).await;
                    return Err(error);
                }
            }
        }

        Ok(())
    }
}

async fn rollback_applied(actions: &[&Action], unit_path: &UnitPath, unit_name: &UnitName) {
    for action in actions.iter().rev() {
        if let Err(error) = action.revert(unit_path, unit_name).await {
            warn!("failed to roll back action for unit {unit_name}: {error}");
        }
    }
}

async fn restore_reverted(actions: &[&Action], unit_path: &UnitPath, unit_name: &UnitName) {
    for action in actions.iter().rev() {
        if let Err(error) = action.apply(unit_path, unit_name).await {
            warn!("failed to restore action for unit {unit_name}: {error}");
        }
    }
}
