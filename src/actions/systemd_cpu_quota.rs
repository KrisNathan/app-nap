use crate::cgroup::Cgroup;
use crate::dbus::client::systemd::SystemdDBusProxy;

const CPU_QUOTA_UNSET: u64 = u64::MAX;

pub async fn apply(
    cgroup: &Cgroup,
    quota: u32,
    systemd: &SystemdDBusProxy<'_>,
) -> zbus::Result<()> {
    systemd
        .set_unit_property(cgroup.get_unit_name().as_str(), "CPUQuotaPerSecUSec", quota)
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
