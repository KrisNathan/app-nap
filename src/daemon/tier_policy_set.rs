use crate::{
    action,
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

    pub fn apply(&self, tier: Tier, app_state: &mut AppState) {
        let policy = self.policy(tier);
        app_state.tier = policy.tier().clone();
        for action in policy.actions() {
            action.apply(&app_state.cgroups, &self.systemd);
        }
    }

    pub fn revert(&self, tier: Tier, app_state: &mut AppState) {
        for action in self.policy(tier).actions() {
            action.revert(&app_state.cgroups, &self.systemd);
        }
    }

    fn policy(&self, tier: Tier) -> &TierPolicy {
        match tier {
            Tier::Performance => &self.performance,
            Tier::Background => &self.background,
            Tier::Nap => &self.nap,
        }
    }
}
