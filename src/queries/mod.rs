use crate::model;
use crate::model::{AccountId, BankAccount};
use crate::model::{BankAccountEvent, CheckNumber};
use async_trait::async_trait;
use cqrs_es::persist::GenericQuery;
use cqrs_es::{EventEnvelope, Query, View};
use money2::{Currency, Money};
use postgres_es::PostgresViewRepository;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

pub type BankAccountViewRepository = PostgresViewRepository<BankAccountView, BankAccount>;
pub type BankAccountViewProjection = Arc<BankAccountViewRepository>;

/// Serialize and persist the bank account view after it is updated. This query also provides a
/// 'load' method to deserialize the view on request.
pub type AccountQuery = GenericQuery<BankAccountViewRepository, BankAccountView, BankAccount>;

/// the view for a BankAccount query, for a standard http application this should be designed to
/// reflect the response that will be returned to a user.
#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct BankAccountView {
    pub account_id: Option<AccountId>,
    pub balance: Money,
    pub written_checks: Vec<CheckNumber>,
    pub ledger: Vec<LedgerEntry>,
}

impl Default for BankAccountView {
    fn default() -> Self {
        Self {
            account_id: None,
            balance: Money { currency: Currency::Usd, ..Default::default() },
            written_checks: Vec::default(),
            ledger: Vec::default(),
        }
    }
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

fn make_neg_factor(currency: Currency) -> Money {
    Money::new(-1, 0, currency)
}

/// Updates the CQRS view with events as they are committed.
impl View<BankAccount> for BankAccountView {
    fn update(&mut self, event: &EventEnvelope<BankAccount>) {
        match &event.payload {
            BankAccountEvent::AccountOpened { account_id, .. } => {
                self.account_id = Some(*account_id);
            },

            BankAccountEvent::BalanceDeposited { amount } => {
                self.ledger.push(LedgerEntry::new("deposit", *amount));
                let converted = model::convert_amount(self.balance.currency, *amount);
                self.balance += converted;
            },

            BankAccountEvent::CashWithdrawal { amount } => {
                let debit = make_neg_factor(amount.currency) * *amount;
                self.ledger.push(LedgerEntry::new("ATM withdrawal", debit));
                let converted = model::convert_amount(self.balance.currency, *amount);
                self.balance -= converted;
            },

            BankAccountEvent::CheckWithdrawal { check_nr, amount } => {
                let debit = make_neg_factor(amount.currency) * *amount;
                self.ledger.push(LedgerEntry::new(format!("Check {check_nr}"), debit));
                self.written_checks.push(*check_nr);
                let converted = model::convert_amount(self.balance.currency, *amount);
                self.balance -= converted;
            },

            event => tracing::debug!(?event, "ignoring non-transactional event"),
        }
    }
}

/// A simple CQRS qeury that traces each event for debugging purposes.
#[derive(Debug)]
pub struct EventTracingQuery;

#[async_trait]
impl Query<BankAccount> for EventTracingQuery {
    #[tracing::instrument(level = "debug")]
    async fn dispatch(&self, aggregate_id: &str, events: &[EventEnvelope<BankAccount>]) {
        for event in events {
            match serde_json::to_string_pretty(&event.payload) {
                Ok(payload) => {
                    tracing::info!("EVENT_TRACE: {aggregate_id}-{}: {payload}", event.sequence)
                },
                Err(err) => {
                    tracing::error!(
                        "EVENT_TRACE: failed to convert bank account event to json: {err:?}"
                    )
                },
            }
        }
    }
}
