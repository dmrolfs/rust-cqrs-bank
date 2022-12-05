use crate::application::app_state::AppState;
use crate::application::result::OptionalResult;
use crate::errors::BankError;
use crate::model::{bank_account, BankAccount};
use crate::model::{
    AccountId, AtmId, BankAccountAggregate, BankAccountCommand, CheckNumber, EmailAddress,
    MailingAddress,
};
use crate::queries::BankAccountViewProjection;
use axum::extract::{rejection::PathRejection, Path, State};
use axum::response::IntoResponse;
use axum::routing;
use axum::{Json, Router};
use cqrs_es::persist::ViewRepository;
use money2::Money;
use pretty_snowflake::envelope::MetaData;
use pretty_snowflake::Id;
use serde::Deserialize;

pub fn api() -> Router<AppState> {
    Router::new()
        .route("/", routing::post(create_bank_account))
        .route("/:account_id", routing::get(serve_bank_account))
        .route("/email/:account_id", routing::post(update_email))
        .route(
            "/address/:account_id",
            routing::post(update_mailing_address),
        )
        .route("/deposit/:account_id", routing::post(deposit_amount))
        .route(
            "/atm/withdrawal/:account_id",
            routing::post(withdrawal_by_atm),
        )
        .route(
            "/check/withdrawl/:account_id",
            routing::post(withdrawal_by_check),
        )
        .route("/balance", routing::get(serve_all_by_balance))
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AccountApplication {
    user_name: String,
    mailing_address: MailingAddress,
    email: EmailAddress,
}

#[tracing::instrument(level = "debug", skip(agg))]
async fn create_bank_account(
    State(agg): State<BankAccountAggregate>, Json(account_application): Json<AccountApplication>,
) -> impl IntoResponse {
    let aggregate_id = bank_account::generate_id();
    let account_id: AccountId = aggregate_id.clone().into();
    let command = BankAccountCommand::OpenAccount {
        account_id,
        user_name: account_application.user_name,
        mailing_address: account_application.mailing_address,
        email: account_application.email,
    };
    let meta: MetaData<BankAccount> = MetaData::default();

    agg.execute_with_metadata(aggregate_id.pretty(), command, meta.into())
        .await
        .map_err::<BankError, _>(|err| err.into())
        .map(|_| Json(account_id))
}

#[tracing::instrument(level = "debug", skip(view_repo))]
async fn serve_bank_account(
    account_id: Result<Path<AccountId>, PathRejection>,
    State(view_repo): State<BankAccountViewProjection>,
) -> impl IntoResponse {
    let Path(account_id) = account_id.map_err(|err| BankError::User(err.into()))?;
    let aggregate_id: Id<BankAccount> = account_id.into();
    tracing::debug!("loading account view for aggregate: {aggregate_id}");
    let view = view_repo
        .load(aggregate_id.pretty())
        .await
        .map_err::<BankError, _>(|err| BankError::DatabaseConnection { source: err.into() })
        .map(|v| OptionalResult(v.map(Json)));

    tracing::debug!("view response: {view:?}");
    view
}

#[tracing::instrument(level = "trace", skip(agg))]
async fn update_email(
    Path(account_id): Path<AccountId>, State(agg): State<BankAccountAggregate>,
    Json(new_email): Json<EmailAddress>,
) -> impl IntoResponse {
    todo!()
}

#[tracing::instrument(level = "trace", skip(agg))]
async fn update_mailing_address(
    Path(account_id): Path<AccountId>, State(agg): State<BankAccountAggregate>,
    Json(new_mailing_address): Json<MailingAddress>,
) -> impl IntoResponse {
    todo!()
}

#[tracing::instrument(level = "trace", skip(agg))]
async fn deposit_amount(
    Path(account_id): Path<AccountId>, State(agg): State<BankAccountAggregate>,
    Json(amount): Json<Money>,
) -> impl IntoResponse {
    todo!()
}

#[tracing::instrument(level = "trace", skip(agg))]
async fn withdrawal_by_atm(
    Path(account_id): Path<AccountId>, State(agg): State<BankAccountAggregate>,
    Json(atm_id): Json<AtmId>,
) -> impl IntoResponse {
    todo!()
}

#[tracing::instrument(level = "trace", skip(agg))]
async fn withdrawal_by_check(
    Path(account_id): Path<AccountId>, State(agg): State<BankAccountAggregate>,
    Json(check_nr): Json<CheckNumber>,
) -> impl IntoResponse {
    todo!()
}

#[tracing::instrument(level = "trace", skip(agg, view))]
async fn serve_all_by_balance(
    State(agg): State<BankAccountAggregate>, State(view): State<BankAccountViewProjection>,
) -> impl IntoResponse {
    todo!()
}
