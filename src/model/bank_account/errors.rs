use crate::model::AccountId;
use crate::services::BankServiceError;
use money2::Money;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BankAccountError {
    #[error("bank account not found id: {0}")]
    NotFound(AccountId),

    #[error("{1} funds not available in account, {0}")]
    InsufficientFunds(AccountId, Money),

    #[error("{0}")]
    BankServiceError(#[from] BankServiceError),

    #[error("Rejected command: {0}")]
    RejectedCommand(String),
}
