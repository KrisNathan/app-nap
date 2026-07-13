use log::warn;

use crate::systemd::{cgroup::systemd_unit_name, dbus_client::SystemdDbusClient};

const CPU_QUOTA_UNSET: u64 = u64::MAX;
const CPU_WEIGHT_DEFAULT: u64 = 100;

pub fn cpu_quota_from_percent(percent: u32) -> u64 {
    u64::from(percent) * 10_000
}

fn unit_name<'a>(cgroup: &'a str, operation: &str) -> Option<&'a str> {
    match systemd_unit_name(cgroup) {
        Some(unit) => Some(unit),
        None => {
            warn!("skipping {operation} for non-app cgroup={cgroup}");
            None
        }
    }
}

pub async fn freeze(systemd: &SystemdDbusClient, cgroups: &[String]) {
    for cgroup in cgroups {
        let Some(unit) = unit_name(cgroup, "freeze") else {
            continue;
        };
        if let Err(err) = systemd.freeze(unit).await {
            warn!("failed to freeze unit={unit} cgroup={cgroup}: {err}");
        }
    }
}

pub async fn thaw(systemd: &SystemdDbusClient, cgroups: &[String]) {
    for cgroup in cgroups {
        let Some(unit) = unit_name(cgroup, "thaw") else {
            continue;
        };
        if let Err(err) = systemd.thaw(unit).await {
            warn!("failed to thaw unit={unit} cgroup={cgroup}: {err}");
        }
    }
}

pub async fn set_cpu_quota(systemd: &SystemdDbusClient, cgroups: &[String], quota: u64) {
    for cgroup in cgroups {
        let Some(unit) = unit_name(cgroup, "set CPU quota") else {
            continue;
        };
        if let Err(err) = systemd
            .set_property(unit, "CPUQuotaPerSecUSec", quota)
            .await
        {
            warn!("failed to set CPU quota for unit={unit} cgroup={cgroup}: {err}");
        }
    }
}

pub async fn unset_cpu_quota(systemd: &SystemdDbusClient, cgroups: &[String]) {
    for cgroup in cgroups {
        let Some(unit) = unit_name(cgroup, "unset CPU quota") else {
            continue;
        };
        if let Err(err) = systemd
            .set_property(unit, "CPUQuotaPerSecUSec", CPU_QUOTA_UNSET)
            .await
        {
            warn!("failed to unset CPU quota for unit={unit} cgroup={cgroup}: {err}");
        }
    }
}

pub async fn set_cpu_weight(systemd: &SystemdDbusClient, cgroups: &[String], weight: u64) {
    for cgroup in cgroups {
        let Some(unit) = unit_name(cgroup, "set CPU weight") else {
            continue;
        };
        if let Err(err) = systemd.set_property(unit, "CPUWeight", weight).await {
            warn!("failed to set CPU weight for unit={unit} cgroup={cgroup}: {err}");
        }
    }
}

pub async fn reset_cpu_weight(systemd: &SystemdDbusClient, cgroups: &[String]) {
    for cgroup in cgroups {
        let Some(unit) = unit_name(cgroup, "reset CPU weight") else {
            continue;
        };
        if let Err(err) = systemd
            .set_property(unit, "CPUWeight", CPU_WEIGHT_DEFAULT)
            .await
        {
            warn!("failed to reset CPU weight for unit={unit} cgroup={cgroup}: {err}");
        }
    }
}
