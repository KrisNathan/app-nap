use std::io;

use crate::systemd::{
    cgroup::systemd_unit_name, dbus_client::SystemdDbusClient, error::SystemdError,
};

// systemd exposes CPU quota over D-Bus as `CPUQuotaPerSecUSec` (u64 microseconds
// per second), not the `CPUQuota=5%` string syntax that `systemctl set-property`
// accepts. `systemctl` parses the percent form and converts it before sending.
// We do the conversion ourselves: 1 second = 1_000_000 us, so 1% = 10_000 us.
// systemd treats `u64::MAX` as "infinity" (no quota), matching the empty
// `CPUQuota=` form used by `systemctl set-property` to clear the limit.
const CPU_QUOTA_UNSET: u64 = u64::MAX;
const CPU_WEIGHT_DEFAULT: u64 = 100;

pub fn cpu_quota_from_percent(percent: u32) -> u64 {
    u64::from(percent) * 10_000
}

fn unit_names<'a>(cgroups: &'a [String], operation: &str) -> Result<Vec<&'a str>, SystemdError> {
    if cgroups.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("cannot {operation}: no app cgroups provided"),
        )
        .into());
    }

    cgroups
        .iter()
        .map(|cgroup| {
            systemd_unit_name(cgroup).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("cannot {operation} non-app cgroup={cgroup}"),
                )
                .into()
            })
        })
        .collect()
}

pub async fn freeze(systemd: &SystemdDbusClient, cgroups: &[String]) -> Result<(), SystemdError> {
    for unit in unit_names(cgroups, "freeze")? {
        systemd.freeze(unit).await?;
    }
    Ok(())
}

pub async fn thaw(systemd: &SystemdDbusClient, cgroups: &[String]) -> Result<(), SystemdError> {
    for unit in unit_names(cgroups, "thaw")? {
        systemd.thaw(unit).await?;
    }
    Ok(())
}

pub async fn set_cpu_quota(
    systemd: &SystemdDbusClient,
    cgroups: &[String],
    quota: u64,
) -> Result<(), SystemdError> {
    for unit in unit_names(cgroups, "set CPU quota")? {
        systemd
            .set_property(unit, "CPUQuotaPerSecUSec", quota)
            .await?;
    }
    Ok(())
}

pub async fn unset_cpu_quota(
    systemd: &SystemdDbusClient,
    cgroups: &[String],
) -> Result<(), SystemdError> {
    for unit in unit_names(cgroups, "unset CPU quota")? {
        systemd
            .set_property(unit, "CPUQuotaPerSecUSec", CPU_QUOTA_UNSET)
            .await?;
    }
    Ok(())
}

pub async fn set_cpu_weight(
    systemd: &SystemdDbusClient,
    cgroups: &[String],
    weight: u64,
) -> Result<(), SystemdError> {
    for unit in unit_names(cgroups, "set CPU weight")? {
        systemd.set_property(unit, "CPUWeight", weight).await?;
    }
    Ok(())
}

pub async fn reset_cpu_weight(
    systemd: &SystemdDbusClient,
    cgroups: &[String],
) -> Result<(), SystemdError> {
    for unit in unit_names(cgroups, "reset CPU weight")? {
        systemd
            .set_property(unit, "CPUWeight", CPU_WEIGHT_DEFAULT)
            .await?;
    }
    Ok(())
}
