use money2::{Currency, Money};
use once_cell::sync::Lazy;
use pretty_snowflake::{Id, Label, Labeling};
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod bank_account;

pub use bank_account::{
    BankAccount, BankAccountAggregate, BankAccountCommand, BankAccountError, BankAccountEvent,
};

pub static ZERO_MONEY: Lazy<Money> = Lazy::new(|| Money::new(0, 2, Currency::Usd));

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct AccountId(i64);

impl AccountId {
    pub fn new(id: impl Into<i64>) -> Self {
        Self(id.into())
    }

    pub const fn as_num(&self) -> i64 {
        self.0
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Id<BankAccount>> for AccountId {
    fn from(id: Id<BankAccount>) -> Self {
        Self::new(id.num())
    }
}

impl From<AccountId> for Id<BankAccount> {
    fn from(account_id: AccountId) -> Self {
        Self::new(
            <BankAccount as Label>::labeler().label(),
            account_id.as_num(),
            &pretty_snowflake::generator::prettifier(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct MailingAddress(String);

impl MailingAddress {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for MailingAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct EmailAddress(String);

impl EmailAddress {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for EmailAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct AtmId(String);

impl AtmId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for AtmId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct CheckNumber(u32);

impl CheckNumber {
    pub fn new(check_nr: impl Into<u32>) -> Self {
        Self(check_nr.into())
    }

    pub const fn as_u32(&self) -> u32 {
        self.0
    }
}

impl fmt::Display for CheckNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
