use crate::application::ApiError;
use crate::model::{BankAccount, BankAccountAggregate};
use crate::queries::{AccountQuery, BankAccountViewProjection, EventTracingQuery};
use crate::services::{BankAccountServices, HappyPathBankAccountServices};
use axum::extract::FromRef;
use cqrs_es::Query;
use postgres_es::PostgresViewRepository;
use sqlx::PgPool;
use std::fmt;
use std::sync::Arc;

pub const ACCOUNT_QUERY_VIEW: &str = "account_query";
pub const ACCOUNT_QUERY_VIEW_PAYLOAD: &str = "payload";

#[tracing::instrument(level = "debug")]
pub async fn initialize_app_state(pool: PgPool) -> Result<AppState, ApiError> {
    let tracing_query = EventTracingQuery;
    let account_view_projection = Arc::new(PostgresViewRepository::new(
        ACCOUNT_QUERY_VIEW,
        pool.clone(),
    ));
    let mut account_query = AccountQuery::new(account_view_projection.clone());
    account_query.use_error_handler(Box::new(
        |err| tracing::error!(error=?err, "account query failed"),
    ));

    let queries: Vec<Box<dyn Query<BankAccount>>> =
        vec![Box::new(tracing_query), Box::new(account_query)];
    let services = BankAccountServices::HappyPath(HappyPathBankAccountServices);

    Ok(AppState {
        bank_account_agg: Arc::new(postgres_es::postgres_cqrs(pool.clone(), queries, services)),
        bank_account_view: account_view_projection,
        db_pool: pool,
    })
}

#[derive(Clone)]
pub struct AppState {
    pub bank_account_agg: BankAccountAggregate,
    pub bank_account_view: BankAccountViewProjection,
    pub db_pool: PgPool,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState").finish()
    }
}

impl FromRef<AppState> for BankAccountAggregate {
    fn from_ref(state: &AppState) -> Self {
        state.bank_account_agg.clone()
    }
}

impl FromRef<AppState> for BankAccountViewProjection {
    fn from_ref(state: &AppState) -> Self {
        state.bank_account_view.clone()
    }
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.db_pool.clone()
    }
}
