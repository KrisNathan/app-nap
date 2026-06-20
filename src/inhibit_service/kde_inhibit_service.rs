use zbus::Connection;

use super::InhibitService;

// KDE PowerDevil exposes active idle/sleep inhibitors via the PolicyAgent
// D-Bus interface. `ListInhibitions` returns `aas`: one string list per
// active inhibitor, `[who, why]`, where `who` is the inhibiting app's name
// (e.g. "firefox", "obs") and `why` is a human-readable reason. (PowerDevil
// uses `QList<QStringList>`, which serializes as `aas`, not `a(ss)`.)
const SERVICE: &str = "org.kde.Solid.PowerManagement.PolicyAgent";
const PATH: &str = "/org/kde/Solid/PowerManagement/PolicyAgent";
const INTERFACE: &str = "org.kde.Solid.PowerManagement.PolicyAgent";

pub struct KdeInhibitService {
    conn: Connection,
}

impl InhibitService for KdeInhibitService {
    async fn list_inhibitors(&self) -> Result<Vec<String>, zbus::Error> {
        let reply = self
            .conn
            .call_method(Some(SERVICE), PATH, Some(INTERFACE), "ListInhibitions", &())
            .await?;

        let inhibitions: Vec<Vec<String>> = reply.body().deserialize()?;

        Ok(inhibitions
            .into_iter()
            .filter_map(|entry| entry.into_iter().next())
            .collect())
    }
}

impl KdeInhibitService {
    pub async fn new() -> Result<Self, zbus::Error> {
        Ok(Self {
            conn: Connection::session().await?,
        })
    }
}
