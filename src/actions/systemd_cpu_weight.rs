use crate::cgroup::Cgroup;
use crate::dbus::client::systemd::SystemdDBusProxy;

const CPU_WEIGHT_DEFAULT: u64 = 100;

pub async fn apply(
    cgroup: &Cgroup,
    weight: u64,
    systemd: &SystemdDBusProxy<'_>,
) -> zbus::Result<()> {
    systemd
        .set_unit_property(cgroup.get_unit_name().as_str(), "CPUWeight", weight)
        .await
}

pub async fn revert(cgroup: &Cgroup, systemd: &SystemdDBusProxy<'_>) -> zbus::Result<()> {
    systemd
        .set_unit_property(
            cgroup.get_unit_name().as_str(),
            "CPUWeight",
            CPU_WEIGHT_DEFAULT,
        )
        .await
}
