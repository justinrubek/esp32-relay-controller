#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Esp(#[from] esp_idf_svc::sys::EspError),
    #[error("missing firmware info for running slot")]
    FirmwareInfoMissing,
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
    #[error("the upstream version can't be compared to the current firmware's")]
    UpstreamVersionInvalid,
    #[error("error writing esp update")]
    EspUpdateError,
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
