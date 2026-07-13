use log::warn;

use crate::systemd::dbus_client::SystemdDbusClient;

const CPU_QUOTA_UNSET: u64 = u64::MAX;
const CPU_WEIGHT_DEFAULT: u64 = 100;

pub fn cpu_quota_from_percent(percent: u32) -> u64 {
    u64::from(percent) * 10_000
}

pub async fn freeze(systemd: &SystemdDbusClient, cgroups: &[String]) {
    for cgroup in cgroups {
        if let Err(err) = systemd.freeze(cgroup).await {
            warn!("failed to freeze cgroup={cgroup}: {err}");
        }
    }
}

pub async fn thaw(systemd: &SystemdDbusClient, cgroups: &[String]) {
    for cgroup in cgroups {
        if let Err(err) = systemd.thaw(cgroup).await {
            warn!("failed to thaw cgroup={cgroup}: {err}");
        }
    }
}

pub async fn set_cpu_quota(systemd: &SystemdDbusClient, cgroups: &[String], quota: u64) {
    for cgroup in cgroups {
        if let Err(err) = systemd
            .set_property(cgroup, "CPUQuotaPerSecUSec", quota)
            .await
        {
            warn!("failed to set CPU quota for cgroup={cgroup}: {err}");
        }
    }
}

pub async fn unset_cpu_quota(systemd: &SystemdDbusClient, cgroups: &[String]) {
    for cgroup in cgroups {
        if let Err(err) = systemd
            .set_property(cgroup, "CPUQuotaPerSecUSec", CPU_QUOTA_UNSET)
            .await
        {
            warn!("failed to unset CPU quota for cgroup={cgroup}: {err}");
        }
    }
}

pub async fn set_cpu_weight(systemd: &SystemdDbusClient, cgroups: &[String], weight: u64) {
    for cgroup in cgroups {
        if let Err(err) = systemd.set_property(cgroup, "CPUWeight", weight).await {
            warn!("failed to set CPU weight for cgroup={cgroup}: {err}");
        }
    }
}

pub async fn reset_cpu_weight(systemd: &SystemdDbusClient, cgroups: &[String]) {
    for cgroup in cgroups {
        if let Err(err) = systemd
            .set_property(cgroup, "CPUWeight", CPU_WEIGHT_DEFAULT)
            .await
        {
            warn!("failed to reset CPU weight for cgroup={cgroup}: {err}");
        }
    }
}
