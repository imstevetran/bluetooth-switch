use thiserror::Error;

#[derive(Error, Debug)]
pub enum BtError {
    #[error("Bluetooth backend error: {0}")]
    Backend(String),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Permission denied: {0}")]
    Permission(String),

    #[error("Not supported on this platform")]
    UnsupportedPlatform,
}

pub type Result<T> = std::result::Result<T, BtError>;
