use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("failed to build HTTP client: {0}")]
    ClientBuild(String),

    #[error("failed to build thread pool : {0}")]
    Threadpool(String),

    #[error("root URL must have a host, got: {0}")]
    MissingHost(String),
}
