use std::io;

use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("auth error: {0}")]
    Auth(String),
    #[error("api error: {0}")]
    Api(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("url parse error: {0}")]
    Url(#[from] url::ParseError),
}
