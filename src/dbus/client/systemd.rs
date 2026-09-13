use zbus::{proxy, zvariant::Value};

#[proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1",
    gen_blocking = false
)]
pub trait SystemdDBus {
    fn freeze_unit(&self, unit: &str) -> zbus::Result<()>;
    fn thaw_unit(&self, unit: &str) -> zbus::Result<()>;

    fn set_unit_properties(
        &self,
        unit: &str,
        runtime: bool,
        properties: Vec<(&str, Value<'_>)>,
    ) -> zbus::Result<()>;
}

impl SystemdDBusProxy<'_> {
    pub async fn set_unit_property(
        &self,
        unit: &str,
        name: &str,
        value: impl Into<Value<'static>>,
    ) -> zbus::Result<()> {
        self.set_unit_properties(unit, true, vec![(name, value.into())])
            .await
    }
}
