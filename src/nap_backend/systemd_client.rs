use serde::Serialize;
use std::io;

use zbus::{
    blocking::Connection,
    zvariant::{DynamicType, Value},
};

const SYSTEMD_SERVICE: &str = "org.freedesktop.systemd1";
const SYSTEMD_PATH: &str = "/org/freedesktop/systemd1";
const SYSTEMD_INTERFACE: &str = "org.freedesktop.systemd1.Manager";

pub struct SystemdClient {
    conn: Connection,
}

impl SystemdClient {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            conn: Connection::session().map_err(map_zbus_error)?,
        })
    }

    pub fn freeze(&self, unit: &str) -> io::Result<()> {
        self.call_method("FreezeUnit", &(unit,), "FreezeUnit")
    }

    pub fn thaw(&self, unit: &str) -> io::Result<()> {
        self.call_method("ThawUnit", &(unit,), "ThawUnit")
    }

    pub fn set_property<T: Into<Value<'static>>>(
        &self,
        unit: &str,
        name: &str,
        value: T,
    ) -> io::Result<()> {
        let properties: &[(&str, Value<'_>)] = &[(name, value.into())];
        self.call_method(
            "SetUnitProperties",
            &(unit, true, properties),
            "SetUnitProperties",
        )
    }

    fn call_method(
        &self,
        method: &str,
        body: &(impl Serialize + DynamicType),
        label: &str,
    ) -> io::Result<()> {
        self.conn
            .call_method(
                Some(SYSTEMD_SERVICE),
                SYSTEMD_PATH,
                Some(SYSTEMD_INTERFACE),
                method,
                body,
            )
            .map_err(|e| io::Error::other(format!("{label} failed: {e}")))?;
        Ok(())
    }
}

fn map_zbus_error(e: zbus::Error) -> io::Error {
    io::Error::other(format!("failed to connect to session bus: {e}"))
}
