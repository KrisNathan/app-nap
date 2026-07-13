use crate::{
    action::{self, ActionError},
    config::model::Config,
    daemon::{
        process::{AppState, Tier},
        tier_policy::TierPolicy,
    },
    systemd::dbus_client::SystemdDbusClient,
};

pub struct TierPolicySet {
    performance: TierPolicy,
    background: TierPolicy,
    nap: TierPolicy,
    systemd: SystemdDbusClient,
}

impl TierPolicySet {
    pub fn from_config(config: &Config, systemd: SystemdDbusClient) -> Self {
        Self {
            performance: TierPolicy::new(
                Tier::Performance,
                action::from_config(&config.tiers.performance.actions),
            ),
            background: TierPolicy::new(
                Tier::Background,
                action::from_config(&config.tiers.background.actions),
            ),
            nap: TierPolicy::new(Tier::Nap, action::from_config(&config.tiers.nap.actions)),
            systemd,
        }
    }

    pub async fn apply(&self, tier: Tier, app_state: &mut AppState) -> Result<(), ActionError> {
        let policy = self.policy(tier);
        for action in policy.actions() {
            action.apply(&app_state.cgroups, &self.systemd).await?;
        }
        app_state.tier = policy.tier().clone();
        Ok(())
    }

    pub async fn revert(&self, tier: Tier, app_state: &mut AppState) -> Result<(), ActionError> {
        for action in self.policy(tier).actions() {
            action.revert(&app_state.cgroups, &self.systemd).await?;
        }
        Ok(())
    }

    fn policy(&self, tier: Tier) -> &TierPolicy {
        match tier {
            Tier::Performance => &self.performance,
            Tier::Background => &self.background,
            Tier::Nap => &self.nap,
        }
    }
}
