use log::warn;

use crate::{
    action::{self, ActionError},
    config::model::Config,
    daemon::{
        app_state::{AppState, Tier},
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
        let Some(policy) = self.policy(tier) else {
            return Ok(());
        };
        let actions = policy.actions();
        let mut applied = 0;
        let mut apply_err = None;
        for action in actions {
            if let Err(err) = action.apply(&app_state.cgroups, &self.systemd).await {
                apply_err = Some(err);
                break;
            }
            applied += 1;
        }
        if let Some(err) = apply_err {
            for prev in actions[..applied].iter().rev() {
                if let Err(rollback_err) = prev.revert(&app_state.cgroups, &self.systemd).await {
                    warn!(
                        "failed to rollback applied action while applying tier={tier:?}: {rollback_err}"
                    );
                }
            }
            return Err(err);
        }
        app_state.tier = *policy.tier();
        Ok(())
    }

    pub async fn revert(&self, tier: Tier, app_state: &mut AppState) -> Result<(), ActionError> {
        let Some(policy) = self.policy(tier) else {
            return Ok(());
        };
        let actions = policy.actions();
        let mut reverted = 0;
        let mut revert_err = None;
        for action in actions {
            if let Err(err) = action.revert(&app_state.cgroups, &self.systemd).await {
                revert_err = Some(err);
                break;
            }
            reverted += 1;
        }
        if let Some(err) = revert_err {
            for prev in actions[..reverted].iter().rev() {
                if let Err(rollback_err) = prev.apply(&app_state.cgroups, &self.systemd).await {
                    warn!(
                        "failed to rollback reverted action while reverting tier={tier:?}: {rollback_err}"
                    );
                }
            }
            return Err(err);
        }
        Ok(())
    }

    fn policy(&self, tier: Tier) -> Option<&TierPolicy> {
        match tier {
            Tier::Performance => Some(&self.performance),
            Tier::Background => Some(&self.background),
            Tier::Nap => Some(&self.nap),
            Tier::Unknown => None,
        }
    }
}
