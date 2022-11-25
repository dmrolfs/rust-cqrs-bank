#![forbid(unsafe_code)]
#![warn(
    clippy::cargo,
    clippy::suspicious,
    // missing_docs,
    clippy::nursery,
    rust_2018_idioms
)]

mod errors;
mod http_server;
mod model;
mod queries;
mod services;
mod settings;
mod tracing;

pub use self::tracing::{get_subscriber, get_tracing_subscriber, init_subscriber};
pub use settings::{CliOptions, CorrelationSettings, Settings};
