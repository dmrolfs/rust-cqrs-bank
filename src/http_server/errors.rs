use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{0}")]
    IO(#[from] std::io::Error),

    #[error("HTTP engine error: {0}")]
    HyperHttp(#[from] hyper::Error),
}
