use crate::cgroup::UnitName;
use crate::dbus::client::systemd::SystemdDBusProxy;

const CPU_WEIGHT_DEFAULT: u64 = 100;

pub async fn apply(
    unit_name: &UnitName,
    weight: u64,
    systemd: &SystemdDBusProxy<'_>,
) -> zbus::Result<()> {
    systemd
        .set_unit_property(unit_name.as_str(), "CPUWeight", weight)
        .await
}

pub async fn revert(unit_name: &UnitName, systemd: &SystemdDBusProxy<'_>) -> zbus::Result<()> {
    systemd
        .set_unit_property(unit_name.as_str(), "CPUWeight", CPU_WEIGHT_DEFAULT)
        .await
}
