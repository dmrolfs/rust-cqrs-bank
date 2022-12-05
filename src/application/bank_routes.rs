use crate::application::app_state::AppState;
use crate::errors::BankError;
use crate::model::{bank_account, BankAccount};
use crate::model::{
    AccountId, AtmId, BankAccountAggregate, BankAccountCommand, CheckNumber, EmailAddress,
    MailingAddress,
};
use crate::queries::BankAccountViewProjection;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing;
use axum::{Json, Router};
use money2::Money;
use pretty_snowflake::envelope::MetaData;
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

#[tracing::instrument(level = "trace", skip(agg))]
async fn create_bank_account(
    State(agg): State<BankAccountAggregate>, Json(account_application): Json<AccountApplication>,
) -> impl IntoResponse {
    let aggregate_id = bank_account::generate_id();
    let account_id: AccountId = aggregate_id.clone().into();
    let command = BankAccountCommand::OpenAccount {
        account_id: account_id.clone(),
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

#[tracing::instrument(level = "trace", skip(agg))]
async fn serve_bank_account(
    Path(account_id): Path<AccountId>, State(agg): State<BankAccountAggregate>,
) -> impl IntoResponse {
    todo!()
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
