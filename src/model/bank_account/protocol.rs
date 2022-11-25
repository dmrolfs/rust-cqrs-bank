use crate::model::{AccountId, AtmId, CheckNumber, EmailAddress, MailingAddress};
use cqrs_es::DomainEvent;
use money2::Money;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BankAccountCommand {
    OpenAccount {
        account_id: AccountId,
        user_name: String,
        mailing_address: MailingAddress,
        email: EmailAddress,
    },
    DepositAmount {
        amount: Money,
    },
    WithdrawCash {
        amount: Money,
        atm_id: AtmId,
    },
    DisburseCheck {
        check_nr: CheckNumber,
        amount: Money,
    },
    ChangeMailingAddress {
        new_address: MailingAddress,
    },
    ChangeEmail {
        new_email: EmailAddress,
    },
}

#[derive(Debug, Display, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
pub enum BankAccountEvent {
    AccountOpened {
        account_id: AccountId,
        user_name: String,
        mailing_address: MailingAddress,
        email: EmailAddress,
    },
    BalanceDeposited {
        amount: Money,
    },
    CashWithdrawal {
        amount: Money,
    },
    CheckWithdrawal {
        check_nr: CheckNumber,
        amount: Money,
    },
    MailingAddressUpdated {
        new_address: MailingAddress,
    },
    EmailUpdated {
        new_email: EmailAddress,
    },
}

const VERSION: &str = "1.0";

impl DomainEvent for BankAccountEvent {
    fn event_type(&self) -> String {
        self.to_string()
    }

    fn event_version(&self) -> String {
        VERSION.to_string()
    }
}
