use crate::{cgroup::UnitName, dbus::client::systemd::SystemdDBusProxy};

pub async fn apply(unit_name: &UnitName, systemd: &SystemdDBusProxy<'_>) -> zbus::Result<()> {
    systemd.freeze_unit(unit_name.as_str()).await
}

pub async fn revert(unit_name: &UnitName, systemd: &SystemdDBusProxy<'_>) -> zbus::Result<()> {
    systemd.thaw_unit(unit_name.as_str()).await
}
