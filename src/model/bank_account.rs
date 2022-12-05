use super::AccountId;
use crate::model::{AtmId, CheckNumber, EmailAddress, MailingAddress, ZERO_MONEY};
use async_trait::async_trait;
use cqrs_es::{Aggregate, DomainEvent};
use money2::Money;
use postgres_es::PostgresCqrs;
use pretty_snowflake::{Id, Label};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

mod errors;
mod protocol;

use crate::services::{BankAccountApi, BankAccountServices};
pub use errors::BankAccountError;
pub use protocol::{BankAccountCommand, BankAccountEvent};

pub type BankAccountAggregate = Arc<PostgresCqrs<BankAccount>>;

pub const AGGREGATE_TYPE: &str = "account";

#[inline]
pub fn generate_id() -> Id<BankAccount> {
    pretty_snowflake::generator::next_id()
}

#[derive(Debug, Default, Clone, Label, PartialEq, Serialize, Deserialize)]
pub struct BankAccount {
    state: BankAccountState,
}

#[async_trait]
impl Aggregate for BankAccount {
    type Command = BankAccountCommand;
    type Event = BankAccountEvent;
    type Error = BankAccountError;
    type Services = BankAccountServices;

    fn aggregate_type() -> String {
        AGGREGATE_TYPE.to_string()
    }

    #[tracing::instrument(level = "trace", skip(services))]
    async fn handle(
        &self, command: Self::Command, services: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        self.state.handle(command, services).await
    }

    fn apply(&mut self, event: Self::Event) {
        if let Some(new_state) = self.state.apply(event) {
            self.state = new_state;
        }
    }
}

#[async_trait]
trait AggregateState {
    type State;

    type Command;
    type Event: DomainEvent;
    type Error: std::error::Error;
    type Services: Send + Sync;

    async fn handle(
        &self, command: Self::Command, services: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error>;

    fn apply(&self, event: Self::Event) -> Option<Self::State>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum BankAccountState {
    Quiescent(QuiescentBankAccount),
    Active(ActiveBankAccount),
    Closed(ClosedBankAccount),
}

impl Default for BankAccountState {
    fn default() -> Self {
        Self::Quiescent(QuiescentBankAccount::default())
    }
}

#[async_trait]
impl AggregateState for BankAccountState {
    type State = Self;
    type Command = <BankAccount as Aggregate>::Command;
    type Event = <BankAccount as Aggregate>::Event;
    type Error = <BankAccount as Aggregate>::Error;
    type Services = <BankAccount as Aggregate>::Services;

    async fn handle(
        &self, command: Self::Command, services: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match self {
            Self::Quiescent(state) => state.handle(command, services).await,
            Self::Active(state) => state.handle(command, services).await,
            Self::Closed(state) => state.handle(command, services).await,
        }
    }

    fn apply(&self, event: Self::Event) -> Option<Self::State> {
        match self {
            Self::Quiescent(state) => state.apply(event),
            Self::Active(state) => state.apply(event),
            Self::Closed(state) => state.apply(event),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
struct QuiescentBankAccount;

#[async_trait]
impl AggregateState for QuiescentBankAccount {
    type State = BankAccountState;
    type Command = <BankAccount as Aggregate>::Command;
    type Event = <BankAccount as Aggregate>::Event;
    type Error = <BankAccount as Aggregate>::Error;
    type Services = <BankAccount as Aggregate>::Services;

    async fn handle(
        &self, command: Self::Command, _services: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            BankAccountCommand::OpenAccount { account_id, user_name, mailing_address, email } => {
                Ok(vec![BankAccountEvent::AccountOpened {
                    account_id,
                    user_name,
                    mailing_address,
                    email,
                }])
            },
            cmd => Err(BankAccountError::RejectedCommand(format!(
                "Unopened account cannot process command: {cmd:?}"
            ))),
        }
    }

    fn apply(&self, event: Self::Event) -> Option<Self::State> {
        match event {
            BankAccountEvent::AccountOpened { account_id, user_name, mailing_address, email } => {
                Some(BankAccountState::Active(ActiveBankAccount {
                    id: account_id.into(),
                    account_id,
                    user_name,
                    balance: Money::default(),
                    mailing_address,
                    email,
                }))
            },

            event => {
                tracing::warn!(?event, "unrecognized bank account event -- ignored");
                None
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ActiveBankAccount {
    id: Id<BankAccount>,
    account_id: AccountId,
    user_name: String,
    balance: Money,
    mailing_address: MailingAddress,
    email: EmailAddress,
}

#[async_trait]
impl AggregateState for ActiveBankAccount {
    type State = BankAccountState;
    type Command = <BankAccount as Aggregate>::Command;
    type Event = <BankAccount as Aggregate>::Event;
    type Error = <BankAccount as Aggregate>::Error;
    type Services = <BankAccount as Aggregate>::Services;

    async fn handle(
        &self, command: Self::Command, services: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            BankAccountCommand::OpenAccount { .. } => Err(BankAccountError::RejectedCommand(
                format!("Active account {} cannot be reopened.", self.account_id),
            )),

            BankAccountCommand::DepositAmount { amount } => {
                Ok(vec![BankAccountEvent::BalanceDeposited { amount }])
            },

            BankAccountCommand::WithdrawCash { amount, atm_id } => {
                self.do_handle_cash_withdrawal(amount, atm_id, services).await
            },

            BankAccountCommand::DisburseCheck { check_nr, amount } => {
                self.do_handle_check_disbursement(check_nr, amount, services).await
            },

            BankAccountCommand::ChangeMailingAddress { new_address } => {
                Ok(vec![BankAccountEvent::MailingAddressUpdated {
                    new_address,
                }])
            },

            BankAccountCommand::ChangeEmail { new_email } => {
                Ok(vec![BankAccountEvent::EmailUpdated { new_email }])
            },
        }
    }

    fn apply(&self, event: Self::Event) -> Option<Self::State> {
        match event {
            BankAccountEvent::BalanceDeposited { amount } => {
                let mut updated = self.clone();
                updated.balance += amount;
                Some(BankAccountState::Active(updated))
            },
            BankAccountEvent::CashWithdrawal { amount } => {
                let mut updated = self.clone();
                updated.balance -= amount; // ignoring negative balance here
                Some(BankAccountState::Active(updated))
            },
            BankAccountEvent::CheckWithdrawal { amount, .. } => {
                let mut updated = self.clone();
                updated.balance -= amount;
                Some(BankAccountState::Active(updated))
            },
            BankAccountEvent::MailingAddressUpdated { new_address } => {
                let mut updated = self.clone();
                updated.mailing_address = new_address;
                Some(BankAccountState::Active(updated))
            },
            BankAccountEvent::EmailUpdated { new_email } => {
                let mut updated = self.clone();
                updated.email = new_email;
                Some(BankAccountState::Active(updated))
            },
            event => {
                tracing::warn!(?event, "unrecognized bank account event -- ignored");
                None
            },
        }
    }
}

impl ActiveBankAccount {
    #[tracing::instrument(level = "trace", skip(self, services))]
    async fn do_handle_cash_withdrawal(
        &self, amount: Money, atm_id: AtmId, services: &<Self as AggregateState>::Services,
    ) -> Result<Vec<<Self as AggregateState>::Event>, <Self as AggregateState>::Error> {
        let remaining_balance = self.check_funds_available(amount)?;
        services.validate_atm_withdrawal(&atm_id, amount).await?;
        tracing::debug!(
            "cash withdrawal from ATM {atm_id} will leave {remaining_balance} in account {}",
            self.account_id
        );
        Ok(vec![BankAccountEvent::CashWithdrawal { amount }])
    }

    #[tracing::instrument(level = "trace", skip(self, services))]
    async fn do_handle_check_disbursement(
        &self, check_nr: CheckNumber, amount: Money, services: &<Self as AggregateState>::Services,
    ) -> Result<Vec<<Self as AggregateState>::Event>, <Self as AggregateState>::Error> {
        let remaining_balance = self.check_funds_available(amount)?;
        services.validate_check(&self.account_id, check_nr).await?;
        tracing::debug!(
            "disbursement of check {check_nr} will leave {remaining_balance} in account {}",
            self.account_id
        );
        Ok(vec![BankAccountEvent::CheckWithdrawal { check_nr, amount }])
    }

    #[tracing::instrument(
    level="trace",
    skip(self),
    fields(account_id=%self.account_id, balance=%self.balance)
    )]
    fn check_funds_available(&self, amount: Money) -> Result<Money, BankAccountError> {
        let balance = self.balance - amount;
        if *ZERO_MONEY <= balance {
            Ok(balance)
        } else {
            Err(BankAccountError::InsufficientFunds(self.account_id, amount))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ClosedBankAccount {
    id: Id<BankAccount>,
    account_id: AccountId,
    user_name: String,
    mailing_address: MailingAddress,
    email: EmailAddress,
}

#[async_trait]
impl AggregateState for ClosedBankAccount {
    type State = BankAccountState;
    type Command = <BankAccount as Aggregate>::Command;
    type Event = <BankAccount as Aggregate>::Event;
    type Error = <BankAccount as Aggregate>::Error;
    type Services = <BankAccount as Aggregate>::Services;

    #[tracing::instrument(level = "trace", skip(_services))]
    async fn handle(
        &self, command: Self::Command, _services: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        Err(BankAccountError::RejectedCommand(format!(
            "Closed account will not accept command: {command:?}"
        )))
    }

    fn apply(&self, event: Self::Event) -> Option<Self::State> {
        tracing::warn!("no events possible dead-end closed account state: {event:?}");
        None
    }
}
