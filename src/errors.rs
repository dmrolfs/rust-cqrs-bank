use crate::{application, model};
use anyhow::anyhow;
use cqrs_es::persist::PersistenceError;
use cqrs_es::AggregateError;
use sqlx::Error;
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, ToSchema, Error)]
#[non_exhaustive]
pub enum BankError {
    #[error("{0}")]
    Api(#[from] application::ApiError),

    #[error("Invalid request data: {0}")]
    Validation(#[from] validator::ValidationErrors),

    #[error("{0}")]
    BankAccount(#[from] model::BankAccountError),

    #[error("User violated bank service business rules: {0}")]
    User(#[from] anyhow::Error),

    #[error("Command was rejected due to a conflict with another command on the same aggregate instance - may retry.")]
    AggregateConflict,

    #[error("failure during attempted database read or write: {source}")]
    DatabaseConnection { source: anyhow::Error },

    #[error("failed to deserialize JSON - possibly invalid: {source}")]
    Deserialization { source: anyhow::Error },

    #[error("Encountered a technical failure preventing the command from being applied to the aggregate: {source}")]
    Unexpected { source: anyhow::Error },
}

impl<E> From<AggregateError<E>> for BankError
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(error: AggregateError<E>) -> Self {
        match error {
            AggregateError::UserError(err) => Self::User(anyhow!(err)),
            AggregateError::AggregateConflict => Self::AggregateConflict,
            AggregateError::DatabaseConnectionError(err) => {
                Self::DatabaseConnection { source: anyhow!(err) }
            },
            AggregateError::DeserializationError(err) => {
                Self::Deserialization { source: anyhow!(err) }
            },
            AggregateError::UnexpectedError(err) => Self::Unexpected { source: anyhow!(err) },
        }
    }
}

impl From<PersistenceError> for BankError {
    fn from(error: PersistenceError) -> Self {
        Self::DatabaseConnection { source: error.into() }
    }
}

impl From<axum::extract::rejection::PathRejection> for BankError {
    fn from(error: axum::extract::rejection::PathRejection) -> Self {
        application::ApiError::Path(error).into()
    }
}

impl From<axum::extract::rejection::JsonRejection> for BankError {
    fn from(error: axum::extract::rejection::JsonRejection) -> Self {
        application::ApiError::Json(error).into()
    }
}

impl From<sqlx::Error> for BankError {
    fn from(source: Error) -> Self {
        application::ApiError::Sql { source }.into()
    }
}
