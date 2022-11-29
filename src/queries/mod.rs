use crate::model::{AccountId, BankAccount};
use crate::model::{BankAccountEvent, CheckNumber};
use async_trait::async_trait;
use cqrs_es::persist::GenericQuery;
use cqrs_es::{EventEnvelope, Query, View};
use money2::Money;
use postgres_es::PostgresViewRepository;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub type BankAccountViewRepository = PostgresViewRepository<BankAccountView, BankAccount>;
pub type BankAccountViewProjection = Arc<BankAccountViewRepository>;

/// Serialize and persist the bank account view after it is updated. This query also provides a
/// 'load' method to deserialize the view on request.
pub type AccountQuery = GenericQuery<BankAccountViewRepository, BankAccountView, BankAccount>;

/// the view for a BankAccount query, for a standard http application this should be designed to
/// reflect the response that will be returned to a user.
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BankAccountView {
    account_id: Option<AccountId>,
    balance: Money,
    written_checks: Vec<CheckNumber>,
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

/// Updates the CQRS view with events as they are committed.
impl View<BankAccount> for BankAccountView {
    fn update(&mut self, event: &EventEnvelope<BankAccount>) {
        match &event.payload {
            BankAccountEvent::AccountOpened { account_id, .. } => {
                self.account_id = Some(account_id.clone());
            },

            BankAccountEvent::BalanceDeposited { amount } => {
                self.ledger.push(LedgerEntry::new("deposit", *amount));
                self.balance += *amount;
            },

            BankAccountEvent::CashWithdrawal { amount } => {
                self.ledger.push(LedgerEntry::new("ATM withdrawal", *amount));
                self.balance -= *amount;
            },

            BankAccountEvent::CheckWithdrawal { check_nr, amount } => {
                self.ledger.push(LedgerEntry::new(format!("Check {check_nr}"), *amount));
                self.written_checks.push(*check_nr);
                self.balance -= *amount;
            },

            event => tracing::debug!(?event, "ignoring non-transactional event"),
        }
    }
}

/// A simple CQRS qeury that traces each event for debugging purposes.
pub struct EventTracingQuery;

#[async_trait]
impl Query<BankAccount> for EventTracingQuery {
    async fn dispatch(&self, aggregate_id: &str, events: &[EventEnvelope<BankAccount>]) {
        for event in events {
            match serde_json::to_string_pretty(&event.payload) {
                Ok(payload) => tracing::info!("{aggregate_id}-{}: {payload}", event.sequence),
                Err(err) => {
                    tracing::error!("failed to convert bank account event to json: {err:?}")
                },
            }
        }
    }
}
