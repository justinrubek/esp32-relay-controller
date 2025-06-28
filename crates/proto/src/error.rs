#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    FlagsetInvalidBits(#[from] flagset::InvalidBits),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid message type: {0}")]
    InvalidMessageType(u8),

    #[error("invalid UTF-8 string")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),

    #[error("string too long: {0} bytes")]
    StringTooLong(usize),

    #[error("byte array too long: {0} bytes")]
    BytesTooLong(usize),

    #[error("Vector too long: {0} items")]
    VectorTooLong(usize),

    #[error("Insufficient data: expected {expected}, got {actual}")]
    InsufficientData { expected: usize, actual: usize },

    #[error("Protocol error: {0}")]
    Protocol(String),
}

pub type Result<T> = std::result::Result<T, Error>;
