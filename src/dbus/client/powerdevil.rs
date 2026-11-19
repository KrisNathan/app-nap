use zbus::proxy;

/// ActiveInhibitions provides tuple and zbus doesn't autoconvert to struct so yea
type InhibitionTuple = (String, String, String, String, u32);

#[derive(Debug, Clone)]
pub struct ActiveInhibition {
    /// "idle", "sleep", "sleep:idle", ""
    #[allow(dead_code)]
    pub policies: String,
    /// Desktop file id (e.g. "org.mozilla.firefox")
    pub app_name: String,
    /// Human-readable reason, apparently it's free-form
    #[allow(dead_code)]
    pub reason: String,
    /// Always "block" in current powerdevil
    #[allow(dead_code)]
    pub mode: String,
    /// Active = 0x1, Allowed = 0x2
    #[allow(dead_code)]
    pub flags: u32,
}

impl ActiveInhibition {
    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        self.flags & 0x1 != 0
    }
    #[allow(dead_code)]
    pub fn is_allowed(&self) -> bool {
        self.flags & 0x2 != 0
    }
}

impl From<InhibitionTuple> for ActiveInhibition {
    fn from((policies, app_name, reason, mode, flags): InhibitionTuple) -> Self {
        Self {
            policies,
            app_name,
            reason,
            mode,
            flags,
        }
    }
}

#[proxy(
    interface = "org.kde.Solid.PowerManagement.PolicyAgent",
    default_service = "org.kde.Solid.PowerManagement.PolicyAgent",
    default_path = "/org/kde/Solid/PowerManagement/PolicyAgent",
    gen_blocking = false
)]
pub trait PowerDevilDBus {
    #[zbus(property)]
    fn active_inhibitions(&self) -> zbus::Result<Vec<InhibitionTuple>>;
}
