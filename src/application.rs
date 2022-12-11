mod app_state;
mod bank_routes;
pub mod errors;
mod health_routes;
mod result;

use crate::settings::HttpApiSettings;
pub use app_state::{AppState, ACCOUNT_QUERY_VIEW, ACCOUNT_QUERY_VIEW_PAYLOAD};
pub use errors::ApiError;
pub use result::HttpResult;

use crate::Settings;
use axum::error_handling::HandleErrorLayer;
use axum::http::{StatusCode, Uri};
use axum::response::IntoResponse;
use axum::{BoxError, Router};
use serde::Deserialize;
use settings_loader::common::database::DatabaseSettings;
use sqlx::PgPool;
use std::net::TcpListener;
use strum::Display;
use tokio::signal;
use tokio::task::JoinHandle;
use tower::ServiceBuilder;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tower_http::ServiceBuilderExt;
use utoipa::OpenApi;
use utoipa_swagger_ui::{SwaggerUi, Url as SwaggerUrl};

pub type HttpJoinHandle = JoinHandle<Result<(), ApiError>>;

#[derive(Debug, Display, PartialEq, Eq)]
#[strum(serialize_all = "snake_case")]
pub enum Version {
    V1,
}

impl Version {
    #[inline]
    pub const fn latest() -> Self {
        Self::V1
    }
}

pub struct Application {
    port: u16,
    server: HttpJoinHandle,
}

impl Application {
    #[tracing::instrument(level = "debug", skip(settings))]
    pub async fn build(settings: &Settings) -> Result<Self, ApiError> {
        let connection_pool = get_connection_pool(&settings.database);
        let address = settings.http_api.server.address();
        let listener = tokio::net::TcpListener::bind(&address).await?;
        tracing::info!(
            "{:?} API listening on {address}: {listener:?}",
            std::env::current_exe()
        );
        let std_listener = listener.into_std()?;
        let port = std_listener.local_addr()?.port();

        let server = run_http_server(
            std_listener,
            connection_pool,
            &RunParameters::from_settings(settings),
        )
        .await?;

        Ok(Self { port, server })
    }

    pub const fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), ApiError> {
        self.server.await?
    }
}

pub fn get_connection_pool(settings: &DatabaseSettings) -> PgPool {
    let connection_options = settings.pg_connect_options_with_db();
    settings.pg_pool_options().connect_lazy_with(connection_options)
}

#[derive(Debug, Clone)]
pub struct RunParameters {
    pub http_api: HttpApiSettings,
}

impl RunParameters {
    pub fn from_settings(settings: &Settings) -> Self {
        Self { http_api: settings.http_api.clone() }
    }
}

#[tracing::instrument(level = "trace")]
pub async fn run_http_server(
    listener: TcpListener, db_pool: PgPool, params: &RunParameters,
) -> Result<HttpJoinHandle, ApiError> {
    let state = app_state::initialize_app_state(db_pool).await?;

    let middleware_stack = ServiceBuilder::new()
        // .rate_limit(params.rate_limit.nr_requests, params.rate_limit.per_duration)
        .layer(HandleErrorLayer::new(handle_api_error))
        .timeout(params.http_api.timeout)
        .compression()
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().include_headers(true))
                .on_response(DefaultOnResponse::new().include_headers(true))
        )
        // .layer(tower::limit::RateLimitLayer::new(
        //     params.rate_limit.nr_requests,
        //     params.rate_limit.per_duration,
        // ))
        // .set_x_request_id(unimplemented!())
        .propagate_x_request_id();

    let api_routes = Router::new()
        .nest("/health", health_routes::api())
        .nest("/bank", bank_routes::api())
        .with_state(state);

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").urls(vec![
            (
                SwaggerUrl::with_primary("bank_api", "/api-doc/bank-openapi.json", true),
                bank_routes::BankApiDoc::openapi(),
            ),
            (
                SwaggerUrl::new("health_api", "/api-doc/health-openapi.json"),
                health_routes::HealthApiDoc::openapi(),
            ),
        ]))
        .nest("/api/v1", api_routes)
        .fallback(fallback)
        .layer(middleware_stack);

    let handle = tokio::spawn(async move {
        tracing::debug!(app_routes=?app, "starting API server...");
        let builder = axum::Server::from_tcp(listener)?;
        let server = builder.serve(app.into_make_service());
        let graceful = server.with_graceful_shutdown(shutdown_signal());
        graceful.await?;
        tracing::info!("{:?} API shutting down", std::env::current_exe());
        Ok(())
    });

    Ok(handle)
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Pagination {
    page: usize,
    per_page: usize,
}

async fn fallback(uri: Uri) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, format!("No route found for {uri}"))
}

async fn handle_api_error(error: BoxError) -> impl IntoResponse {
    if error.is::<tower::timeout::error::Elapsed>() {
        (
            StatusCode::REQUEST_TIMEOUT,
            format!("request took too long: {error}"),
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {error}"),
        )
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("signal received, starting graceful shutdown");
}
