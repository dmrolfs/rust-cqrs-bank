use money2::{Currency, Exchange, ExchangeRates, Money};
use once_cell::sync::Lazy;
use pretty_snowflake::{Id, Label, Labeling};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;
use utoipa::{IntoParams, ToSchema};

pub mod bank_account;

pub use bank_account::{
    BankAccount, BankAccountAggregate, BankAccountCommand, BankAccountError, BankAccountEvent,
};

pub static ZERO_MONEY: Lazy<Money> = Lazy::new(|| Money::new(0, 2, Currency::Usd));

static EXCHANGE_RATES: Lazy<ExchangeRates> = Lazy::new(|| {
    let rates = std::fs::read_to_string("./resources/eurofxref.csv")
        .expect("failed to load exchange rate file");
    ExchangeRates::from_str(&rates).expect("failed to parse exchange rate file")
});

pub fn convert_amount(currency: Currency, amount: Money) -> Money {
    if currency == amount.currency {
        amount
    } else {
        amount.exchange(currency, &EXCHANGE_RATES)
    }
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    ToSchema,
    IntoParams,
    Serialize,
    Deserialize,
)]
#[schema(example = json!(7006077196242653184_u64))]
#[into_params(names("account_id"))]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, ToSchema, Serialize, Deserialize)]
#[schema(example = json!("12 Seahawks Way, Renton, WA 98056"))]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, ToSchema, Serialize, Deserialize)]
#[schema(example = json!("otis@example.com"))]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, ToSchema, Serialize, Deserialize)]
#[schema(example = json!("abc_123"))]
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

impl std::str::FromStr for AtmId {
    type Err = Infallible;

    fn from_str(rep: &str) -> Result<Self, Self::Err> {
        Ok(rep.into())
    }
}

impl From<&str> for AtmId {
    fn from(rep: &str) -> Self {
        Self::new(rep)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ToSchema, Serialize, Deserialize)]
#[schema(example = json!("1082"))]
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_account_id_to_aggregate_id_conversion_works() {
        let aggregate_id = bank_account::generate_id();
        let account_id: AccountId = aggregate_id.clone().into();
        assert_eq!(account_id, AccountId(aggregate_id.num()));
        assert_eq!(account_id.0, aggregate_id.num());

        let actual: Id<BankAccount> = account_id.into();
        assert_eq!(actual, aggregate_id);
        assert_eq!(actual.num(), aggregate_id.num());
        assert_eq!(actual.pretty(), aggregate_id.pretty());
    }
}
