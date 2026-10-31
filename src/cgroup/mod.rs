pub mod cpu_stat;
pub mod procfs;
pub mod resolve;
pub mod sysfs;

use crate::partial_eq_str;

// This doesn't warrant for a macro.

/// The full cgroup v2 path from `/proc/<pid>/cgroup`, `0::` stripped.
///
/// Example: `/user.slice/…/app.slice/app-konsole-1.scope/main.scope`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CgroupPath(String);
impl CgroupPath {
    pub fn from_cgroup_full(cgroup: &str) -> Option<Self> {
        let cgroup = cgroup.trim().split_once("::")?;
        Some(Self(cgroup.1.to_owned()))
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
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for UnitPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl UnitPath {
    pub fn from_cgroup_path(cgroup: CgroupPath) -> Option<Self> {
        Some(Self(split_at_nearest_app_unit(&cgroup.0)?.to_owned()))
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
impl From<&UnitPath> for UnitName {
    fn from(value: &UnitPath) -> Self {
        Self(value.0.rsplit('/').next().unwrap_or_default().to_owned())
    }
}
impl From<UnitPath> for UnitName {
    fn from(value: UnitPath) -> Self {
        Self::from(&value)
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
/// Returns the path through that unit.
fn split_at_nearest_app_unit(full: &str) -> Option<&str> {
    // Fence: refuse anything outside app.slice before matching names, so an
    // app-* component in session.slice/system.slice can never be a target.
    let inside = full.split("/app.slice/").nth(1)?;
    let fence = full.len() - inside.len();

    // Walk left to right and keep the last match: nearest, not top-level.
    let mut end = None;
    let mut offset = fence;
    for component in inside.split('/') {
        if is_app_unit(component) {
            end = Some(offset + component.len());
        }
        offset += component.len() + 1;
    }

    Some(&full[..end?])
}

partial_eq_str!(CgroupPath);
partial_eq_str!(UnitPath);
partial_eq_str!(UnitName);

#[cfg(test)]
mod tests {
    use super::*;

    const KONSOLE: &str = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-39967.scope/main.scope\n";
    const KONSOLE_TAB: &str = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-39967.scope/tab(9).scope\n";
    const KONSOLE_UNIT: &str =
        "/user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-39967.scope";
    const FIREFOX: &str =
        "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-firefox-42.scope\n";
    const ZED_IN_TAB: &str = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-konsole.scope/tab(9).scope/app-zed.service\n";

    /// Runs both resolution steps. Panics when the contents do not parse, so
    /// the refusal tests below exercise the fence and not the `0::` split.
    fn resolve_unit(contents: &str) -> Option<UnitPath> {
        let full = CgroupPath::from_cgroup_full(contents).expect("test input must parse");
        UnitPath::from_cgroup_path(full)
    }

    #[test]
    fn parses_the_full_path_from_proc_cgroup() {
        assert_eq!(
            CgroupPath::from_cgroup_full(KONSOLE).unwrap(),
            "/user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-39967.scope/main.scope"
        );
    }

    #[test]
    fn refuses_a_line_without_the_unified_hierarchy() {
        assert!(CgroupPath::from_cgroup_full("2:cpu:/user.slice\n").is_none());
    }

    #[test]
    fn resolves_the_unit_path() {
        assert_eq!(resolve_unit(KONSOLE).unwrap(), KONSOLE_UNIT);
    }

    #[test]
    fn the_unit_name_is_the_last_component_of_the_unit_path() {
        let unit_path = resolve_unit(KONSOLE).unwrap();

        assert_eq!(
            UnitName::from(&unit_path),
            "app-org.kde.konsole-39967.scope"
        );
    }

    #[test]
    fn a_leaf_app_unit_truncates_to_itself() {
        let unit_path = resolve_unit(FIREFOX).unwrap();

        assert_eq!(
            unit_path,
            "/user.slice/user-1000.slice/user@1000.service/app.slice/app-firefox-42.scope"
        );
        assert_eq!(UnitName::from(&unit_path), "app-firefox-42.scope");
    }

    #[test]
    fn fine_cgroups_under_one_unit_resolve_to_the_same_unit() {
        let main = resolve_unit(KONSOLE).unwrap();
        let tab = resolve_unit(KONSOLE_TAB).unwrap();

        assert_eq!(main, KONSOLE_UNIT);
        assert_eq!(main, tab);
    }

    #[test]
    fn nearest_app_unit_wins_over_the_fence_owner() {
        // zed launched from a konsole tab: the foreign app-zed.service is the
        // nearest app-* unit, not konsole's own scope.
        let unit_path = resolve_unit(ZED_IN_TAB).unwrap();

        assert_eq!(
            unit_path,
            "/user.slice/user-1000.slice/user@1000.service/app.slice/app-konsole.scope/tab(9).scope/app-zed.service"
        );
        assert_eq!(UnitName::from(&unit_path), "app-zed.service");
    }

    #[test]
    fn session_slice_is_not_an_app_slice() {
        // The fence matches the `/app.slice/` component, not the substring
        // "app.slice" inside "session.slice".
        assert!(
            resolve_unit(
                "0::/user.slice/user-1000.slice/user@1000.service/session.slice/app-foo.service\n"
            )
            .is_none()
        );
    }

    #[test]
    fn refuses_app_unit_outside_the_fence() {
        assert!(
            resolve_unit("0::/system.slice/app-foo.service\n").is_none(),
            "an app-* unit outside /app.slice/ must be refused"
        );
    }

    #[test]
    fn refuses_non_app_unit_under_the_fence() {
        assert!(
            resolve_unit(
                "0::/user.slice/user-1000.slice/user@1000.service/app.slice/pipewire.service\n"
            )
            .is_none(),
            "a non-app-* unit under /app.slice/ must be refused"
        );
    }

    #[test]
    fn refuses_a_plain_cgroup() {
        assert!(resolve_unit("0::/some/plain/cgroup\n").is_none());
    }
}
