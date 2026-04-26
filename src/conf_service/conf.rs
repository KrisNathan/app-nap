use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NapBackendType {
    Signal,
    SystemdScope,
}

impl NapBackendType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Signal => "signal",
            Self::SystemdScope => "systemd-scope",
        }
    }
}

impl fmt::Display for NapBackendType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for NapBackendType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "signal" => Ok(Self::Signal),
            "systemd-scope" => Ok(Self::SystemdScope),
            _ => Err(format!("unknown nap backend type: {value}")),
        }
    }
}

impl Serialize for NapBackendType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for NapBackendType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conf {
    pub nap_backend_type: NapBackendType,
}
