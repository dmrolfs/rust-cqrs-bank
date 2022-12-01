use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
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
