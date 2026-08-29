use log::warn;

use crate::{
    action::{self, ActionError},
    config::model::Config,
    daemon::{
        app_state::{Load, Tier},
        policy::{Policy, PolicyKey},
    },
    systemd::dbus_client::SystemdDbusClient,
};

pub struct Policies {
    performance: Policy,
    background_busy: Policy,
    background_idle: Option<Policy>,
    nap_idle: Policy,
    nap_busy: Option<Policy>,
    systemd: SystemdDbusClient,
}

fn resolve_policy_key(
    tier: Tier,
    load: Load,
    has_background_idle: bool,
    has_nap_busy: bool,
) -> Option<PolicyKey> {
    match (tier, load) {
        (Tier::Performance, _) => Some(PolicyKey::Performance),
        (Tier::Background, Load::Busy) => Some(PolicyKey::BackgroundBusy),
        (Tier::Background, Load::Idle) => Some(if has_background_idle {
            PolicyKey::BackgroundIdle
        } else {
            PolicyKey::BackgroundBusy
        }),
        (Tier::Nap, Load::Idle) => Some(PolicyKey::NapIdle),
        (Tier::Nap, Load::Busy) => Some(if has_nap_busy {
            PolicyKey::NapBusy
        } else {
            PolicyKey::NapIdle
        }),
        (Tier::Unknown, _) => None,
    }
}

impl Policies {
    /// Map config `[tiers.*]` slots into effective policies.
    pub fn from_config(config: &Config, systemd: SystemdDbusClient) -> Self {
        Self {
            performance: Policy::new(action::from_config(&config.tiers.performance.actions)),
            background_busy: Policy::new(action::from_config(&config.tiers.background.actions)),
            background_idle: config
                .tiers
                .background
                .idle
                .as_ref()
                .map(|cfg| Policy::new(action::from_config(&cfg.actions))),
            nap_idle: Policy::new(action::from_config(&config.tiers.nap.actions)),
            nap_busy: config
                .tiers
                .nap
                .busy
                .as_ref()
                .map(|cfg| Policy::new(action::from_config(&cfg.actions))),
            systemd,
        }
    }

    /// Effective policy for (tier, load). Load is ignored on performance; a
    /// missing load variant falls back to the tier's base policy.
    /// `None` for `Tier::Unknown` (nothing applied yet).
    pub fn resolve(&self, tier: Tier, load: Load) -> Option<PolicyKey> {
        resolve_policy_key(
            tier,
            load,
            self.background_idle.is_some(),
            self.nap_busy.is_some(),
        )
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

    fn policy(&self, key: PolicyKey) -> &Policy {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn performance_ignores_load_and_variants() {
        for load in [Load::Busy, Load::Idle] {
            assert_eq!(
                resolve_policy_key(Tier::Performance, load, false, false),
                Some(PolicyKey::Performance)
            );
        }
    }

    #[test]
    fn background_busy_uses_busy_policy() {
        assert_eq!(
            resolve_policy_key(Tier::Background, Load::Busy, true, true),
            Some(PolicyKey::BackgroundBusy)
        );
    }

    #[test]
    fn background_idle_uses_idle_policy_when_defined() {
        assert_eq!(
            resolve_policy_key(Tier::Background, Load::Idle, true, false),
            Some(PolicyKey::BackgroundIdle)
        );
    }

    #[test]
    fn background_idle_falls_back_to_busy_when_not_defined() {
        assert_eq!(
            resolve_policy_key(Tier::Background, Load::Idle, false, true),
            Some(PolicyKey::BackgroundBusy)
        );
    }

    #[test]
    fn nap_idle_uses_idle_policy() {
        assert_eq!(
            resolve_policy_key(Tier::Nap, Load::Idle, false, false),
            Some(PolicyKey::NapIdle)
        );
    }

    #[test]
    fn nap_busy_uses_busy_policy_when_defined() {
        assert_eq!(
            resolve_policy_key(Tier::Nap, Load::Busy, false, true),
            Some(PolicyKey::NapBusy)
        );
    }

    #[test]
    fn nap_busy_falls_back_to_idle_when_not_defined() {
        assert_eq!(
            resolve_policy_key(Tier::Nap, Load::Busy, true, false),
            Some(PolicyKey::NapIdle)
        );
    }

    #[test]
    fn unknown_tier_resolves_to_nothing() {
        for load in [Load::Busy, Load::Idle] {
            assert_eq!(resolve_policy_key(Tier::Unknown, load, true, true), None);
        }
    }
}
