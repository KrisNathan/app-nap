use thiserror::Error;

#[derive(Debug, Error)]
pub enum SystemdError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Dbus(#[from] zbus::Error),
}
