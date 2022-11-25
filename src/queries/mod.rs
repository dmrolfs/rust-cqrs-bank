use crate::model::{AccountId, BankAccount};
use money2::Money;
use postgres_es::PostgresViewRepository;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub type BankAccountProjection = Arc<PostgresViewRepository<BankAccountView, BankAccount>>;

/// the view for a BankAccount query, for a standard http application this should be designed to
/// reflect the response that will be returned to a user.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BankAccountView {
    account_id: Option<AccountId>,
    balance: Money,
    // written_checks: Vec<Check>,
    ledger: Vec<LedgerEntry>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub description: String,
    pub amount: Money,
}

impl LedgerEntry {
    pub fn new(description: impl Into<String>, amount: Money) -> Self {
        Self { description: description.into(), amount }
    }
}
