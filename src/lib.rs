#![forbid(unsafe_code)]
#![warn(
    clippy::cargo,
    clippy::suspicious,
    // missing_docs,
    clippy::nursery,
    rust_2018_idioms
)]

pub mod application;
mod errors;
mod model;
mod queries;
mod services;
mod settings;
pub mod tracing;

pub use application::{ApiError, Application};
pub use model::{AccountId, BankAccount, CheckNumber};
pub use settings::{CliOptions, CorrelationSettings, Settings};
