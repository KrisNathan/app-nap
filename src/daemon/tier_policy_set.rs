use std::sync::Arc;

use crate::{
    action,
    config::config::Config,
    daemon::{process::Tier, tier_policy::TierPolicy},
    systemd::dbus_client::SystemdDbusClient,
};

pub struct TierPolicySet {
    performance: TierPolicy,
    background: TierPolicy,
    nap: TierPolicy,
}

impl TierPolicySet {
    pub fn from_config(config: &Config, client: Arc<SystemdDbusClient>) -> Self {
        Self {
            performance: TierPolicy::new(
                super::process::Tier::Performance,
                action::from_config(&config.tiers.performance.actions, client.clone()),
            ),
            background: TierPolicy::new(
                super::process::Tier::Background,
                action::from_config(&config.tiers.background.actions, client.clone()),
            ),
            nap: TierPolicy::new(
                super::process::Tier::Nap,
                action::from_config(&config.tiers.nap.actions, client),
            ),
        }
    }

    pub fn get(&self, tier: Tier) -> &TierPolicy {
        match tier {
            Tier::Performance => &self.performance,
            Tier::Background => &self.background,
            Tier::Nap => &self.nap,
        }
    }
}
