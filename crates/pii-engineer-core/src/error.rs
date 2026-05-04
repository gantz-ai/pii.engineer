use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("ort: {0}")]
    Ort(#[from] ort::Error),

    #[error("tokenizer: {0}")]
    Tokenizer(String),

    #[error("model: {0}")]
    Model(String),
}

pub type Result<T> = std::result::Result<T, Error>;
