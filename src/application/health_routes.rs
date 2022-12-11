use crate::application::app_state::AppState;
use crate::application::ACCOUNT_QUERY_VIEW;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Router;
use axum::{routing, Json};
use itertools::Itertools;
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::string::ToString;
use strum_macros::{Display, EnumString, EnumVariantNames};
use utoipa::{OpenApi, ToSchema};

#[derive(OpenApi)]
#[openapi(
    paths(serve_health, serve_readiness, serve_liveness),
    components(
        schemas(HealthStatus, HealthStatusReport)
    ),
    tags(
        (name = "health", description = "Bank Account Health API")
    )
)]
pub struct HealthApiDoc;

pub fn api() -> Router<AppState> {
    Router::new()
        // .merge(
        //     SwaggerUi::new("/swagger-ui")
        //         .url("/api-doc/health/openapi.json", HealthApiDoc::openapi()),
        // )
        .route("/", routing::get(serve_health))
        .route("/ready", routing::get(serve_readiness))
        .route("/live", routing::get(serve_liveness))
}

#[derive(
    Debug,
    Display,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    EnumString,
    EnumVariantNames,
    ToSchema,
    Serialize,
)]
// #[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum HealthStatus {
    Up,
    NotReady,
    Error,
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize)]
pub struct HealthStatusReport {
    status: HealthStatus,
}

impl From<HealthStatus> for HealthStatusReport {
    fn from(status: HealthStatus) -> Self {
        Self { status }
    }
}

impl From<HealthStatus> for StatusCode {
    fn from(health: HealthStatus) -> Self {
        match health {
            HealthStatus::Up => Self::OK,
            HealthStatus::Error => Self::INTERNAL_SERVER_ERROR,
            HealthStatus::Down | HealthStatus::NotReady => Self::SERVICE_UNAVAILABLE,
        }
    }
}

#[utoipa::path(
    get,
    path = "/",
    context_path = "/api/v1/health",
    tag = "health",
    responses(
        (status = 200, description = "system health"),
        (status = 5XX, description = "system down")
    )
)]
#[axum::debug_handler]
#[tracing::instrument(level = "trace", skip(app))]
async fn serve_health(State(app): State<AppState>) -> impl IntoResponse {
    let (system_health, _health_report) = check_health(app).await;
    serde_json::to_value::<HealthStatusReport>(system_health.into())
        .map(|resp| (system_health.into(), Json(resp)))
        .unwrap_or_else(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": error.to_string() })),
            )
        })
}

#[utoipa::path(
    get,
    path = "/ready",
    context_path = "/api/v1/health",
    tag = "health",
    responses(
        (status = 200, description = "system ready"),
        (status = 5XX, description = "system not ready")
    )
)]
#[axum::debug_handler]
#[tracing::instrument(level = "trace", skip(app))]
async fn serve_readiness(State(app): State<AppState>) -> impl IntoResponse {
    let (system_health, _) = check_health(app).await;
    let status_code: StatusCode = system_health.into();
    status_code
}

#[utoipa::path(
    get,
    path = "/live",
    context_path = "/api/v1/health",
    tag = "health",
    responses(
        (status = 200, description = "system live"),
        (status = 5XX, description = "system not live")
    )
)]
#[axum::debug_handler]
#[tracing::instrument(level = "trace", skip(app))]
async fn serve_liveness(State(app): State<AppState>) -> impl IntoResponse {
    let (system_health, _) = check_health(app).await;
    let status_code: StatusCode = system_health.into();
    status_code
}

#[tracing::instrument(level = "trace", skip(app))]
async fn check_health(app: AppState) -> (HealthStatus, HashMap<HealthStatus, Vec<&'static str>>) {
    let view_select_sql = format!("SELECT version FROM {ACCOUNT_QUERY_VIEW}");
    let view_status: Result<(), anyhow::Error> = sqlx::query(&view_select_sql)
        .fetch_optional(&app.db_pool)
        .await
        .map_err(|err| err.into())
        .map(|_| ());

    let agg_select_sql = "SELECT event_version FROM events";
    let agg_status: Result<(), anyhow::Error> = sqlx::query(agg_select_sql)
        .fetch_optional(&app.db_pool)
        .await
        .map_err(|err| err.into())
        .map(|_| ());

    let service_statuses = vec![
        ("bank_account_aggregate", agg_status),
        ("bank_account_view", view_status),
    ];

    let service_by_status = service_statuses
        .into_iter()
        .map(|(service, status)| {
            let health = match status {
                Ok(()) => HealthStatus::Up,
                Err(error) => {
                    tracing::error!("{service} service is down with error: {error:?}");
                    HealthStatus::Error
                },
            };
            (service, health)
        })
        .into_group_map_by(|(_, health)| *health);

    let health_report: HashMap<_, _> = service_by_status
        .into_iter()
        .map(|(status, service_status)| {
            let services: Vec<_> = service_status.into_iter().map(|s| s.0).collect();
            (status, services)
        })
        .collect();

    let all_services_are_up =
        health_report.iter().all(|(health, _services)| *health == HealthStatus::Up);
    let system_health = if all_services_are_up { HealthStatus::Up } else { HealthStatus::Down };

    (system_health, health_report)
}
