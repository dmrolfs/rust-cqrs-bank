use crate::model::{
    AccountId, AtmId, BankAccountAggregate, CheckNumber, EmailAddress, MailingAddress,
};
use crate::queries::BankAccountView;
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{routing, Extension};
use axum::{Json, Router};
use money2::Money;

pub fn api() -> Router {
    Router::new()
        .route("/", routing::post(create_bank_account))
        .route("/:account_id", routing::get(bank_account))
        .route("/email/:account_id", routing::post(update_email))
        .route(
            "/address/:account_id",
            routing::post(update_mailing_address),
        )
        .route("/deposit/::account_id", routing::post(deposit_amount))
        .route(
            "/atm/withdrawal/:account_id",
            routing::post(withdrawal_by_atm),
        )
        .route(
            "/check/withdrawl/:account_id",
            routing::post(withdrawal_by_check),
        )
    // .route("/balance", routing::get(all_by_balance))
}

#[tracing::instrument(level = "trace", skip(agg))]
async fn create_bank_account(Extension(agg): Extension<BankAccountAggregate>) -> impl IntoResponse {
    todo!()
}

#[tracing::instrument(level = "trace", skip(agg))]
async fn bank_account(
    Path(account_id): Path<AccountId>, Extension(agg): Extension<BankAccountAggregate>,
) -> impl IntoResponse {
    todo!()
}

#[tracing::instrument(level = "trace", skip(agg))]
async fn update_email(
    Path(account_id): Path<AccountId>, Json(new_email): Json<EmailAddress>,
    Extension(agg): Extension<BankAccountAggregate>,
) -> impl IntoResponse {
    todo!()
}

#[tracing::instrument(level = "trace", skip(agg))]
async fn update_mailing_address(
    Path(account_id): Path<AccountId>, Json(new_mailing_address): Json<MailingAddress>,
    Extension(agg): Extension<BankAccountAggregate>,
) -> impl IntoResponse {
    todo!()
}

#[tracing::instrument(level = "trace", skip(agg))]
async fn deposit_amount(
    Path(account_id): Path<AccountId>, Json(amount): Json<Money>,
    Extension(agg): Extension<BankAccountAggregate>,
) -> impl IntoResponse {
    todo!()
}

#[tracing::instrument(level = "trace", skip(agg))]
async fn withdrawal_by_atm(
    Path(account_id): Path<AccountId>, Json(atm_id): Json<AtmId>,
    Extension(agg): Extension<BankAccountAggregate>,
) -> impl IntoResponse {
    todo!()
}

#[tracing::instrument(level = "trace", skip(agg))]
async fn withdrawal_by_check(
    Path(account_id): Path<AccountId>, Json(check_nr): Json<CheckNumber>,
    Extension(agg): Extension<BankAccountAggregate>,
) -> impl IntoResponse {
    todo!()
}

#[tracing::instrument(level = "trace", skip(agg, view))]
async fn all_by_balance(
    Extension(agg): Extension<BankAccountAggregate>, Extension(view): Extension<BankAccountView>,
) -> impl IntoResponse {
    todo!()
}
