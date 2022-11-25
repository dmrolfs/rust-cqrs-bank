use crate::queries::BankAccountProjection;
use crate::settings::HttpApiSettings;
use axum::error_handling::HandleErrorLayer;
use axum::handler::Handler;
use axum::http::{StatusCode, Uri};
use axum::response::IntoResponse;
use axum::{BoxError, Router};
use serde::Deserialize;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tower::ServiceBuilder;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tower_http::ServiceBuilderExt;

mod bank_routes;
mod errors;
mod health_routes;

use crate::model::BankAccountAggregate;
pub use errors::ApiError;

pub type TxHttpGracefulShutdown = oneshot::Sender<()>;
pub type HttpJoinHandle = JoinHandle<Result<(), ApiError>>;

#[derive(Debug, PartialEq, Eq)]
enum Version {
    V1,
}

#[tracing::instrument(level = "trace", skip(bankaccount_agg, bankaccount_view))]
pub async fn run_http_server(
    bankaccount_agg: BankAccountAggregate, bankaccount_view: BankAccountProjection,
    settings: &HttpApiSettings,
) -> Result<(HttpJoinHandle, TxHttpGracefulShutdown), ApiError> {
    let middleware_stack = ServiceBuilder::new()
        // .rate_limit(settings.rate_limit.nr_requests, settings.rate_limit.per_duration)
        .layer(HandleErrorLayer::new(handle_api_error))
        .timeout(settings.timeout)
        .compression()
        .add_extension(bankaccount_agg)
        .add_extension(bankaccount_view)
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
        .propagate_x_request_id()
        .into_inner();

    let api_routes = Router::new()
        .nest("/health", health_routes::api())
        .nest("/bank", bank_routes::api());

    let app = Router::new()
        .nest("/api/:version", api_routes)
        .fallback(fallback.into_service())
        .layer(middleware_stack);

    let host = settings.http.host.clone();
    let port = settings.http.port;

    let (tx_shutdown, rx_shutdown) = oneshot::channel();
    let handle = tokio::spawn(async move {
        let address = format!("{host}:{port}");
        let listener = tokio::net::TcpListener::bind(&address).await?;
        tracing::info!(
            "{:?} API listening on {address}: {listener:?}",
            std::env::current_exe()
        );
        let std_listener = listener.into_std()?;
        let builder = axum::Server::from_tcp(std_listener)?;
        let server = builder.serve(app.into_make_service());
        let graceful = server.with_graceful_shutdown(async {
            rx_shutdown.await.ok();
        });
        graceful.await?;
        tracing::info!("{:?} API shutting down", std::env::current_exe());
        Ok(())
    });

    Ok((handle, tx_shutdown))
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
