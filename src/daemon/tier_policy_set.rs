use log::warn;

use crate::{
    action::{self, ActionError},
    config::model::Config,
    daemon::{
        app_state::{Load, Tier},
        tier_policy::{PolicyKey, TierPolicy},
    },
    systemd::dbus_client::SystemdDbusClient,
};

pub struct TierPolicySet {
    performance: TierPolicy,
    background_busy: TierPolicy,
    background_idle: Option<TierPolicy>,
    nap_idle: TierPolicy,
    nap_busy: Option<TierPolicy>,
    systemd: SystemdDbusClient,
}

impl TierPolicySet {
    pub fn from_config(config: &Config, systemd: SystemdDbusClient) -> Self {
        Self {
            performance: TierPolicy::new(action::from_config(&config.tiers.performance.actions)),
            background_busy: TierPolicy::new(action::from_config(&config.tiers.background.actions)),
            background_idle: config
                .tiers
                .background
                .idle
                .as_ref()
                .map(|tier| TierPolicy::new(action::from_config(&tier.actions))),
            nap_idle: TierPolicy::new(action::from_config(&config.tiers.nap.actions)),
            nap_busy: config
                .tiers
                .nap
                .busy
                .as_ref()
                .map(|tier| TierPolicy::new(action::from_config(&tier.actions))),
            systemd,
        }
    }

    /// Effective policy for (tier, load). Load is ignored on performance; a
    /// missing load variant falls back to the tier's base policy.
    /// `None` for `Tier::Unknown` (nothing applied yet).
    pub fn resolve(&self, tier: Tier, load: Load) -> Option<PolicyKey> {
        match (tier, load) {
            (Tier::Performance, _) => Some(PolicyKey::Performance),
            (Tier::Background, Load::Busy) => Some(PolicyKey::BackgroundBusy),
            (Tier::Background, Load::Idle) => Some(if self.background_idle.is_some() {
                PolicyKey::BackgroundIdle
            } else {
                PolicyKey::BackgroundBusy
            }),
            (Tier::Nap, Load::Idle) => Some(PolicyKey::NapIdle),
            (Tier::Nap, Load::Busy) => Some(if self.nap_busy.is_some() {
                PolicyKey::NapBusy
            } else {
                PolicyKey::NapIdle
            }),
            (Tier::Unknown, _) => None,
        }
    }

    pub async fn apply(&self, key: PolicyKey, cgroups: &[String]) -> Result<(), ActionError> {
        let actions = self.policy(key).actions();
        let mut applied = 0;
        let mut apply_err = None;
        for action in actions {
            if let Err(err) = action.apply(cgroups, &self.systemd).await {
                apply_err = Some(err);
                break;
            }
            applied += 1;
        }
        if let Some(err) = apply_err {
            for prev in actions[..applied].iter().rev() {
                if let Err(rollback_err) = prev.revert(cgroups, &self.systemd).await {
                    warn!(
                        "failed to rollback applied action while applying policy={key:?}: {rollback_err}"
                    );
                }
            }
            return Err(err);
        }
        Ok(())
    }

    pub async fn revert(&self, key: PolicyKey, cgroups: &[String]) -> Result<(), ActionError> {
        let actions = self.policy(key).actions();
        let mut reverted = 0;
        let mut revert_err = None;
        for action in actions {
            if let Err(err) = action.revert(cgroups, &self.systemd).await {
                revert_err = Some(err);
                break;
            }
            reverted += 1;
        }
        if let Some(err) = revert_err {
            for prev in actions[..reverted].iter().rev() {
                if let Err(rollback_err) = prev.apply(cgroups, &self.systemd).await {
                    warn!(
                        "failed to rollback reverted action while reverting policy={key:?}: {rollback_err}"
                    );
                }
            }
            return Err(err);
        }
        Ok(())
    }

    fn policy(&self, key: PolicyKey) -> &TierPolicy {
        match key {
            PolicyKey::Performance => &self.performance,
            PolicyKey::BackgroundBusy => &self.background_busy,
            PolicyKey::BackgroundIdle => self
                .background_idle
                .as_ref()
                .unwrap_or(&self.background_busy),
            PolicyKey::NapIdle => &self.nap_idle,
            PolicyKey::NapBusy => self.nap_busy.as_ref().unwrap_or(&self.nap_idle),
        }
    }
}
