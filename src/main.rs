use clap::Parser;
use settings_loader::{LoadingOptions, SettingsLoader};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = bankaccount::tracing::get_tracing_subscriber("info");
    bankaccount::tracing::init_subscriber(subscriber);

    let app_environment = std::env::var(bankaccount::CliOptions::env_app_environment())?;
    let options = bankaccount::CliOptions::parse();
    let settings = bankaccount::Settings::load(&options)?;
    tracing::info!(?options, ?settings, %app_environment, "loaded settings via CLI options and environment");

    todo!()
}
