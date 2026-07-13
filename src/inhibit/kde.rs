use crate::inhibit::InhibitService;

const SERVICE: &str = "org.kde.Solid.PowerManagement.PolicyAgent";
const PATH: &str = "/org/kde/Solid/PowerManagement/PolicyAgent";
const INTERFACE: &str = "org.kde.Solid.PowerManagement.PolicyAgent";

pub struct KdeInhibitService {
    dbus_conn: zbus::Connection,
}

impl InhibitService for KdeInhibitService {
    async fn is_inhibiting(&self, cgroups: &[String]) -> bool {
        if cgroups.is_empty() {
            return false;
        }
        let Ok(inhibitors) = self.list_inhibitors().await else {
            return false;
        };
        inhibitors
            .iter()
            .any(|who| cgroups.iter().any(|cgroup| cgroup.contains(who)))
    }
}

impl KdeInhibitService {
    pub fn new(dbus_conn: zbus::Connection) -> Self {
        Self { dbus_conn }
    }

    async fn list_inhibitors(&self) -> Result<Vec<String>, zbus::Error> {
        let reply = self
            .dbus_conn
            .call_method(Some(SERVICE), PATH, Some(INTERFACE), "ListInhibitions", &())
            .await?;

        let inhibitions: Vec<Vec<String>> = reply.body().deserialize()?;

        Ok(inhibitions
            .into_iter()
            .filter_map(|entry| entry.into_iter().next())
            .collect())
    }
}
