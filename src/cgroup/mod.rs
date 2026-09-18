pub mod cpu_stat;
pub mod proc_util;
pub mod resolve;

use libc::pid_t;
use std::fs;
use std::hash::Hash;
use std::io;

use crate::cgroup::cpu_stat::CpuStat;

// This doesn't warrant for a macro.

/// The full cgroup v2 path from `/proc/<pid>/cgroup`, `0::` stripped.
///
/// Example: `/user.slice/…/app.slice/app-konsole-1.scope/main.scope`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CgroupPath(String);
impl CgroupPath {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for CgroupPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// [`CgroupPath`] truncated at (and including) the nearest `app-*` unit.
/// Used for reading in `/sys/fs/cgroup`.
///
/// Example: `/user.slice/…/app.slice/app-konsole-1.scope`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnitPath(String);
impl UnitPath {
    /// Returns the PIDs of all processes in this cgroup.
    /// Utilizes /sys/fs/cgroup to read the cgroup.procs file.
    pub fn get_pids(&self) -> io::Result<Vec<pid_t>> {
        let mut pids = Vec::new();
        let mut pending: Vec<std::path::PathBuf> = vec![format!("/sys/fs/cgroup{self}").into()];

        while let Some(path) = pending.pop() {
            pids.extend(
                fs::read_to_string(path.join("cgroup.procs"))?
                    .lines()
                    .filter_map(|line| line.trim().parse::<pid_t>().ok()),
            );

            for entry in fs::read_dir(path)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    pending.push(entry.path());
                }
            }
        }

        Ok(pids)
    }
}
impl std::fmt::Display for UnitPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The nearest `app-*` unit component alone.
///
/// Example: `app-konsole-1.scope`.
///
/// This is the systemd/D-Bus handle associated with a unit ledger.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnitName(String);
impl UnitName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for UnitName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The three identities of one process cgroup, resolved together.
///
/// Full cgroup path, unit path, unit name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cgroup {
    full: CgroupPath,
    unit_path: UnitPath,
    unit_name: UnitName,
}

impl Hash for Cgroup {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.full.hash(state);
    }
}

impl Cgroup {
    /// Parse the contents of `/proc/<pid>/cgroup`.
    ///
    /// Returns `None` unless the path is fenced under `/app.slice/` **and**
    /// contains an `app-*` unit; the nearest (rightmost) one wins.
    pub fn parse(contents: &str) -> Option<Self> {
        let full = contents.trim().split_once("::")?.1;
        let (unit_name, unit_path) = split_at_nearest_app_unit(full)?;

        Some(Self {
            full: CgroupPath(full.to_owned()),
            unit_path: UnitPath(unit_path.to_owned()),
            unit_name: UnitName(unit_name.to_owned()),
        })
    }

    pub fn from_pid(pid: pid_t) -> io::Result<Self> {
        let contents = fs::read_to_string(format!("/proc/{pid}/cgroup"))?;
        Self::parse(&contents).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("not an app cgroup: {}", contents.trim()),
            )
        })
    }

    pub fn get_full(&self) -> &CgroupPath {
        &self.full
    }

    pub fn get_unit_path(&self) -> &UnitPath {
        &self.unit_path
    }

    pub fn get_unit_name(&self) -> &UnitName {
        &self.unit_name
    }

    pub fn get_cpu_stat(&self) -> io::Result<CpuStat> {
        cpu_stat::get_cpu_stat(self.full.0.as_str())
    }
}

/// `true` for a systemd app unit component: `app-<name>.{scope,service,slice}`.
fn is_app_unit(component: &str) -> bool {
    component.starts_with("app-")
        && (component.ends_with(".scope")
            || component.ends_with(".service")
            || component.ends_with(".slice"))
}

/// Split `full` at the nearest `app-*` unit under `/app.slice/`.
///
/// Returns `(unit name, path through that unit)`.
fn split_at_nearest_app_unit(full: &str) -> Option<(&str, &str)> {
    // Fence: refuse anything outside app.slice before matching names, so an
    // app-* component in session.slice/system.slice can never be a target.
    let inside = full.split("/app.slice/").nth(1)?;
    let fence = full.len() - inside.len();

    // Walk left to right and keep the last match: nearest, not top-level.
    let mut name = None;
    let mut end = fence;
    let mut offset = fence;
    for component in inside.split('/') {
        if is_app_unit(component) {
            name = Some(component);
            end = offset + component.len();
        }
        offset += component.len() + 1;
    }

    Some((name?, &full[..end]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KONSOLE: &str = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-39967.scope/main.scope\n";

    #[test]
    fn resolves_the_three_identities() {
        let cgroup = Cgroup::parse(KONSOLE).unwrap();

        assert_eq!(
            cgroup.get_full().as_str(),
            "/user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-39967.scope/main.scope"
        );
        assert_eq!(
            cgroup.get_unit_path().as_str(),
            "/user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-39967.scope"
        );
        assert_eq!(
            cgroup.get_unit_name().as_str(),
            "app-org.kde.konsole-39967.scope"
        );
    }

    #[test]
    fn a_leaf_app_unit_truncates_to_itself() {
        let cgroup = Cgroup::parse(
            "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-firefox-42.scope\n",
        )
        .unwrap();

        assert_eq!(cgroup.get_unit_path().as_str(), cgroup.get_full().as_str());
        assert_eq!(cgroup.get_unit_name().as_str(), "app-firefox-42.scope");
    }

    #[test]
    fn fine_cgroups_under_one_unit_share_a_unit_path() {
        let main = Cgroup::parse(KONSOLE).unwrap();
        let tab = Cgroup::parse(
            "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-39967.scope/tab(9).scope\n",
        )
        .unwrap();

        assert_ne!(main.get_full(), tab.get_full());
        assert_eq!(main.get_unit_path(), tab.get_unit_path());
    }

    #[test]
    fn nearest_app_unit_wins_over_the_fence_owner() {
        // zed launched from a konsole tab: the foreign app-zed.service is the
        // nearest app-* unit, not konsole's own scope.
        let cgroup = Cgroup::parse(
            "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-konsole.scope/tab(9).scope/app-zed.service\n",
        )
        .unwrap();

        assert_eq!(
            cgroup.get_unit_path().as_str(),
            "/user.slice/user-1000.slice/user@1000.service/app.slice/app-konsole.scope/tab(9).scope/app-zed.service"
        );
        assert_eq!(cgroup.get_unit_name().as_str(), "app-zed.service");
    }

    #[test]
    fn session_slice_is_not_an_app_slice() {
        // The fence matches the `/app.slice/` component, not the substring
        // "app.slice" inside "session.slice".
        assert!(
            Cgroup::parse(
                "0::/user.slice/user-1000.slice/user@1000.service/session.slice/app-foo.service\n"
            )
            .is_none()
        );
    }

    #[test]
    fn refuses_app_unit_outside_the_fence() {
        assert!(
            Cgroup::parse("0::/system.slice/app-foo.service\n").is_none(),
            "an app-* unit outside /app.slice/ must be refused"
        );
    }

    #[test]
    fn refuses_non_app_unit_under_the_fence() {
        assert!(
            Cgroup::parse(
                "0::/user.slice/user-1000.slice/user@1000.service/app.slice/pipewire.service\n"
            )
            .is_none(),
            "a non-app-* unit under /app.slice/ must be refused"
        );
    }

    #[test]
    fn refuses_a_plain_cgroup() {
        assert!(Cgroup::parse("0::/some/plain/cgroup\n").is_none());
    }
}
