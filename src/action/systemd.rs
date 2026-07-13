use crate::systemd::dbus_client::SystemdDbusClient;

const CPU_QUOTA_UNSET: u64 = u64::MAX;
const CPU_WEIGHT_DEFAULT: u64 = 100;

pub fn cpu_quota_from_percent(percent: u32) -> u64 {
    u64::from(percent) * 10_000
}

pub async fn freeze(systemd: &SystemdDbusClient, cgroups: &[String]) {
    for cgroup in cgroups {
        systemd.freeze(cgroup).await.ok();
    }
}

pub async fn thaw(systemd: &SystemdDbusClient, cgroups: &[String]) {
    for cgroup in cgroups {
        systemd.thaw(cgroup).await.ok();
    }
}

pub async fn set_cpu_quota(systemd: &SystemdDbusClient, cgroups: &[String], quota: u64) {
    for cgroup in cgroups {
        systemd
            .set_property(cgroup, "CPUQuotaPerSecUSec", quota)
            .await
            .ok();
    }
}

pub async fn unset_cpu_quota(systemd: &SystemdDbusClient, cgroups: &[String]) {
    for cgroup in cgroups {
        systemd
            .set_property(cgroup, "CPUQuotaPerSecUSec", CPU_QUOTA_UNSET)
            .await
            .ok();
    }
}

pub async fn set_cpu_weight(systemd: &SystemdDbusClient, cgroups: &[String], weight: u64) {
    for cgroup in cgroups {
        systemd.set_property(cgroup, "CPUWeight", weight).await.ok();
    }
}

pub async fn reset_cpu_weight(systemd: &SystemdDbusClient, cgroups: &[String]) {
    for cgroup in cgroups {
        systemd
            .set_property(cgroup, "CPUWeight", CPU_WEIGHT_DEFAULT)
            .await
            .ok();
    }
}
