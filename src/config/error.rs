use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("cannot read config file {0}: {1}")]
    Read(PathBuf, #[source] std::io::Error),

    #[error("cannot parse config file: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("invalid configuration: {0}")]
    Validation(#[from] ConfigValidationError),
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigValidationError {
    #[error("{field} must be greater than 0 (got {found})")]
    MustBePositive { field: &'static str, found: u64 },

    #[error("{field} must be finite and non-negative (got {found})")]
    InvalidThreshold { field: &'static str, found: f64 },

    #[error(
        "{lower_field} must be less than {upper_field} \
         (got {lower} and {upper})"
    )]
    InvalidThresholdOrder {
        lower_field: &'static str,
        lower: f64,
        upper_field: &'static str,
        upper: f64,
    },
}
