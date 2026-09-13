use crate::{cgroup::Cgroup, dbus::client::systemd::SystemdDBusProxy};

pub async fn apply(cgroup: &Cgroup, systemd: &SystemdDBusProxy<'_>) -> zbus::Result<()> {
    systemd.freeze_unit(cgroup.get_unit_name().as_str()).await
}

pub async fn revert(cgroup: &Cgroup, systemd: &SystemdDBusProxy<'_>) -> zbus::Result<()> {
    systemd.thaw_unit(cgroup.get_unit_name().as_str()).await
}
