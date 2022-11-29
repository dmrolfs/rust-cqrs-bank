use crate::Settings;
use axum::error_handling::HandleErrorLayer;
use axum::http::{StatusCode, Uri};
use axum::response::IntoResponse;
use axum::{BoxError, Router};
use serde::Deserialize;
use tokio::signal;
use tokio::task::JoinHandle;
use tower::ServiceBuilder;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tower_http::ServiceBuilderExt;

mod app_state;
mod bank_routes;
mod errors;
mod health_routes;

pub use errors::ApiError;

pub type HttpJoinHandle = JoinHandle<Result<(), ApiError>>;

#[derive(Debug, PartialEq, Eq)]
enum Version {
    V1,
}

#[tracing::instrument(level = "trace")]
pub async fn run_http_server(settings: &Settings) -> Result<HttpJoinHandle, ApiError> {
    let state = app_state::initialize_app_state(settings).await?;

    let middleware_stack = ServiceBuilder::new()
        // .rate_limit(settings.rate_limit.nr_requests, settings.rate_limit.per_duration)
        .layer(HandleErrorLayer::new(handle_api_error))
        .timeout(settings.http_api.timeout)
        .compression()
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().include_headers(true))
                .on_response(DefaultOnResponse::new().include_headers(true))
        )
        // .layer(tower::limit::RateLimitLayer::new(
        //     settings.rate_limit.nr_requests,
        //     settings.rate_limit.per_duration,
        // ))
        // .set_x_request_id(unimplemented!())
        .propagate_x_request_id();

    let api_routes = Router::new()
        .nest("/health", health_routes::api())
        .nest("/bank", bank_routes::api())
        .with_state(state);

    let app = Router::new()
        .nest("/api/:version", api_routes)
        .fallback(fallback)
        .layer(middleware_stack);

    let address = settings.http_api.server.address();

    let handle = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(&address).await?;
        tracing::info!(
            "{:?} API listening on {address}: {listener:?}",
            std::env::current_exe()
        );
        let std_listener = listener.into_std()?;
        let builder = axum::Server::from_tcp(std_listener)?;
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
