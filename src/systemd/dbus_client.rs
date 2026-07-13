use serde::Serialize;
use zbus::zvariant::{DynamicType, Value};

use crate::systemd::error::SystemdError;

const SYSTEMD_SERVICE: &str = "org.freedesktop.systemd1";
const SYSTEMD_PATH: &str = "/org/freedesktop/systemd1";
const SYSTEMD_INTERFACE: &str = "org.freedesktop.systemd1.Manager";

pub struct SystemdDbusClient {
    conn: zbus::Connection,
}

impl SystemdDbusClient {
    pub fn new(conn: zbus::Connection) -> Self {
        Self { conn }
    }

    pub async fn freeze(&self, unit: &str) -> Result<(), SystemdError> {
        self.call_method("FreezeUnit", &(unit,)).await
    }

    pub async fn thaw(&self, unit: &str) -> Result<(), SystemdError> {
        self.call_method("ThawUnit", &(unit,)).await
    }

    pub async fn set_property<T: Into<Value<'static>>>(
        &self,
        unit: &str,
        name: &str,
        value: T,
    ) -> Result<(), SystemdError> {
        let properties: &[(&str, Value<'_>)] = &[(name, value.into())];
        self.call_method("SetUnitProperties", &(unit, true, properties))
            .await
    }

    async fn call_method(
        &self,
        method: &str,
        body: &(impl Serialize + DynamicType),
    ) -> Result<(), SystemdError> {
        self.conn
            .call_method(
                Some(SYSTEMD_SERVICE),
                SYSTEMD_PATH,
                Some(SYSTEMD_INTERFACE),
                method,
                body,
            )
            .await?;
        Ok(())
    }
}
