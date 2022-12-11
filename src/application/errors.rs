use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Error, ToSchema)]
pub enum ApiError {
    #[error("Invalid URL path input: {0}")]
    Path(#[from] axum::extract::rejection::PathRejection),

    #[error("Invalid JSON payload: {0}")]
    Json(#[from] axum::extract::rejection::JsonRejection),

    #[error("{source}")]
    IO {
        #[from]
        source: std::io::Error,
        // backtrace: Backtrace,
    },

    #[error("HTTP engine error: {source}")]
    HyperHttp {
        #[from]
        source: hyper::Error,
        // backtrace: Backtrace,
    },

    #[error("failed database operation: {source} ")]
    Sql {
        #[from]
        source: sqlx::Error,
        // backtrace: Backtrace,
    },

    #[error("failed joining with thread: {0}")]
    Join(#[from] tokio::task::JoinError),
}
