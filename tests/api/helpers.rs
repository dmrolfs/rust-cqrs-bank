use axum::http::StatusCode;
pub use bankaccount::tracing::TEST_TRACING;
use once_cell::sync::Lazy;
use pretty_assertions::assert_eq;
use settings_loader::common::database::DatabaseSettings;
use settings_loader::SettingsLoader;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use tokio_test::assert_ok;

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TEST_TRACING);

    let settings = {
        let mut options = bankaccount::CliOptions::default();
        options.config = Some("./tests/data/settings.yaml".into());
        assert_ok!(
            bankaccount::Settings::load(&options),
            "Failed to load configuration."
        )
    };

    // configure_database(&settings.database).await;

    let application = assert_ok!(
        bankaccount::Application::build(&settings).await,
        "Failed to build application."
    );
    let application_port = application.port();

    let _ = tokio::spawn(application.run_until_stopped());

    let api_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build();
    let api_client = assert_ok!(api_client);

    let test_app = TestApp {
        http_address: format!("http://{}", settings.http_api.server.address()),
        port: application_port,
        // db_pool: bankaccount::application::get_connection_pool(&settings.database),
        api_client,
    };

    test_app
}

#[tracing::instrument(level = "trace", skip(settings))]
async fn configure_database(settings: &DatabaseSettings) -> PgPool {
    let mut connection = assert_ok!(
        PgConnection::connect_with(&settings.pg_connect_options_without_db()).await,
        "Failed to connect to Postgres."
    );

    assert_ok!(
        connection
            .execute(&*format!(
                r##"CREATE DATABASE "{}";"##,
                settings.database_name
            ))
            .await,
        "Failed to create database."
    );

    let connection_pool = assert_ok!(
        PgPool::connect_with(settings.pg_connect_options_with_db()).await,
        "Failed to connect to Postgres."
    );

    assert_ok!(
        sqlx::migrate!("./migrations").run(&connection_pool).await,
        "Failed to migrate the database."
    );

    connection_pool
}

pub struct TestApp {
    pub http_address: String,
    pub port: u16,
    // pub db_pool: PgPool,
    pub api_client: reqwest::Client,
}

pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        assert_ok!(response.headers().get("Location").ok_or("No Location")),
        location
    );
}
