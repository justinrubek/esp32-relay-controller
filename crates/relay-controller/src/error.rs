#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Esp(#[from] esp_idf_svc::sys::EspError),
    #[error("hostname is too long")]
    HostnameTooLong,
    #[error("missing required configuration: {0}")]
    MissingConfig(String),
    #[error(transparent)]
    StdIo(#[from] std::io::Error),
    #[error("wifi ssid is too long")]
    SsidTooLong,
    #[error(transparent)]
    StowageProto(#[from] stowage_proto::error::Error),
    #[error("wifi password is too long")]
    PasswordTooLong,
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
