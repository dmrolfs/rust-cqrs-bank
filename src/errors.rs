use crate::{http_server, model};
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum BankError {
    #[error("{0}")]
    Api(#[from] http_server::ApiError),

    #[error("{0}")]
    BankAccount(#[from] model::BankAccountError),
}
