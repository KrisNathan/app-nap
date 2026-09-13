use zbus::proxy;

#[proxy(
    interface = "org.kde.Solid.PowerManagement.PolicyAgent",
    default_service = "org.kde.Solid.PowerManagement.PolicyAgent",
    default_path = "/org/kde/Solid/PowerManagement/PolicyAgent",
    gen_blocking = false
)]
pub trait PowerDevilDBus {
    fn list_inhibitions(&self) -> zbus::Result<Vec<(String, String)>>; // (app_name, reason)
}
