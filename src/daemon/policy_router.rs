use crate::{
    actions::Action,
    cgroup::{UnitName, UnitPath},
    config::models::{action::ActionConfig, policies::PoliciesConfig},
    daemon::models::policy::Policy,
};
use futures::future::join_all;

pub struct PolicyRouter {
    performance: Vec<Action>,
    background_idle: Vec<Action>,
    background_busy: Vec<Action>,
    nap_idle: Vec<Action>,
    nap_busy: Vec<Action>,
}

impl PolicyRouter {
    pub async fn from_config(config: &PoliciesConfig, conn: &zbus::Connection) -> Self {
        Self {
            performance: build_actions(&config.performance.actions, conn).await,
            background_idle: build_actions(&config.background_idle.actions, conn).await,
            background_busy: build_actions(&config.background_busy.actions, conn).await,
            nap_idle: build_actions(&config.nap_idle.actions, conn).await,
            nap_busy: build_actions(&config.nap_busy.actions, conn).await,
        }
    }
}
async fn build_actions(actions: &Vec<ActionConfig>, conn: &zbus::Connection) -> Vec<Action> {
    join_all(actions.iter().map(|action_conf| async move {
        Action::from_config(action_conf, conn).await.unwrap() // pray
    }))
    .await
}

impl PolicyRouter {
    fn map_policy_to_actions(&self, policy: Policy) -> &Vec<Action> {
        match policy {
            Policy::Performance => &self.performance,
            Policy::BackgroundIdle => &self.background_idle,
            Policy::BackgroundBusy => &self.background_busy,
            Policy::NapIdle => &self.nap_idle,
            Policy::NapBusy => &self.nap_busy,
        }
    }
    pub async fn apply(&self, policy: Policy, unit_path: &UnitPath, unit_name: &UnitName) {
        for action in self.map_policy_to_actions(policy) {
            action.apply(unit_path, unit_name).await.unwrap(); // pray
        }
    }
    pub async fn revert(&self, policy: Policy, unit_path: &UnitPath, unit_name: &UnitName) {
        // lifo
        for action in self.map_policy_to_actions(policy).iter().rev() {
            action.revert(unit_path, unit_name).await.unwrap(); // pray
        }
    }
}
