#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("string longer than nvs partition limit")]
    StringTooLarge,
    #[error(transparent)]
    StdIo(#[from] std::io::Error),
}
