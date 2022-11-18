#![forbid(unsafe_code)]
#![warn(
    clippy::cargo,
    clippy::suspicious,
    // missing_docs,
    clippy::nursery,
    rust_2018_idioms
)]

mod settings;
pub mod tracing;

pub use settings::{CliOptions, CorrelationSettings, Settings};
