use crate::model::{AccountId, AtmId, CheckNumber};
use async_trait::async_trait;
use money2::Money;
use thiserror::Error;

#[async_trait]
pub trait BankAccountApi: Sync + Send {
    async fn validate_atm_withdrawal(
        &self, atm_id: &AtmId, amount: Money,
    ) -> Result<(), BankServiceError>;

    async fn validate_check(
        &self, account_id: &AccountId, check: CheckNumber,
    ) -> Result<(), BankServiceError>;
}

#[derive(Debug, Clone)]
pub enum BankAccountServices {
    HappyPath(HappyPathBankAccountServices),
}

#[async_trait]
impl BankAccountApi for BankAccountServices {
    async fn validate_atm_withdrawal(
        &self, atm_id: &AtmId, amount: Money,
    ) -> Result<(), BankServiceError> {
        match self {
            Self::HappyPath(svc) => svc.validate_atm_withdrawal(atm_id, amount).await,
        }
    }

    async fn validate_check(
        &self, account_id: &AccountId, check: CheckNumber,
    ) -> Result<(), BankServiceError> {
        match self {
            Self::HappyPath(svc) => svc.validate_check(account_id, check).await,
        }
    }
}

#[derive(Debug, Error)]
pub enum BankServiceError {
    #[error("ATM rule violation: {0}")]
    Atm(String),

    #[error("Invalid check {1} for account {0}")]
    InvalidCheck(AccountId, CheckNumber),
}

#[derive(Debug, Copy, Clone)]
pub struct HappyPathBankAccountServices;

#[async_trait]
impl BankAccountApi for HappyPathBankAccountServices {
    async fn validate_atm_withdrawal(
        &self, _atm_id: &AtmId, _amount: Money,
    ) -> Result<(), BankServiceError> {
        Ok(())
    }

    async fn validate_check(
        &self, _account_id: &AccountId, _check: CheckNumber,
    ) -> Result<(), BankServiceError> {
        Ok(())
    }
}

impl From<HappyPathBankAccountServices> for BankAccountServices {
    fn from(svc: HappyPathBankAccountServices) -> Self {
        Self::HappyPath(svc)
    }
}
