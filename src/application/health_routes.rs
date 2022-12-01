use crate::application::app_state::AppState;
use crate::model::BankAccountAggregate;
use crate::queries::BankAccountViewProjection;
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

pub fn api() -> Router<AppState> {
    Router::new()
        .route("/", routing::get(serve_health))
        .route("/ready", routing::get(serve_readiness))
        .route("/live", routing::get(serve_liveness))
}

#[derive(
    Debug, Display, Copy, Clone, PartialEq, Eq, Hash, EnumString, EnumVariantNames, Serialize,
)]
// #[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum HealthStatus {
    Up,
    NotReady,
    Error,
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HealthStatusResponse {
    status: HealthStatus,
}

impl From<HealthStatus> for HealthStatusResponse {
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

#[tracing::instrument(level = "trace", skip(agg, view))]
async fn serve_health(
    State(agg): State<BankAccountAggregate>, State(view): State<BankAccountViewProjection>,
) -> impl IntoResponse {
    let (system_health, _health_report) = check_health(agg, view).await;
    serde_json::to_value::<HealthStatusResponse>(system_health.into())
        .map(|resp| (system_health.into(), Json(resp)))
        .unwrap_or_else(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": error.to_string() })),
            )
        })
}

#[tracing::instrument(level = "trace", skip(agg, view))]
async fn serve_readiness(
    State(agg): State<BankAccountAggregate>, State(view): State<BankAccountViewProjection>,
) -> impl IntoResponse {
    let (system_health, _) = check_health(agg, view).await;
    let status_code: StatusCode = system_health.into();
    status_code
}

#[tracing::instrument(level = "trace", skip(agg, view))]
async fn serve_liveness(
    State(agg): State<BankAccountAggregate>, State(view): State<BankAccountViewProjection>,
) -> impl IntoResponse {
    let (system_health, _) = check_health(agg, view).await;
    let status_code: StatusCode = system_health.into();
    status_code
}

#[tracing::instrument(level = "trace", skip(_bankaccount_agg, _bankaccount_view))]
async fn check_health(
    _bankaccount_agg: BankAccountAggregate, _bankaccount_view: BankAccountViewProjection,
) -> (HealthStatus, HashMap<HealthStatus, Vec<&'static str>>) {
    let service_statuses = vec![
        ("bank_account_aggregate", Result::<_, anyhow::Error>::Ok(())),
        ("bank_account_view", Result::<_, anyhow::Error>::Ok(())),
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
