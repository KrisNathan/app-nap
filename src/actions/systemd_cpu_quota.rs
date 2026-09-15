use crate::cgroup::Cgroup;
use crate::dbus::client::systemd::SystemdDBusProxy;

const CPU_QUOTA_UNSET: u64 = u64::MAX;

fn cpu_quota_from_percent(percent: u64) -> u64 {
    u64::from(percent) * 10_000
}

pub async fn apply(
    cgroup: &Cgroup,
    quota_percent: u64,
    systemd: &SystemdDBusProxy<'_>,
) -> zbus::Result<()> {
    systemd
        .set_unit_property(
            cgroup.get_unit_name().as_str(),
            "CPUQuotaPerSecUSec",
            cpu_quota_from_percent(quota_percent),
        )
        .await
}

pub async fn revert(cgroup: &Cgroup, systemd: &SystemdDBusProxy<'_>) -> zbus::Result<()> {
    systemd
        .set_unit_property(
            cgroup.get_unit_name().as_str(),
            "CPUQuotaPerSecUSec",
            CPU_QUOTA_UNSET,
        )
        .await
}
