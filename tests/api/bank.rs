use crate::helpers::{spawn_latest_app, TestApp};
use axum::http::StatusCode;
use bankaccount::{AccountId, AtmId, BankAccount, BankAccountView, CheckNumber, LedgerEntry};
use claim::{assert_ok, assert_some};
use money2::{Currency, Money};
use pretty_assertions::{assert_eq, assert_ne};
use pretty_snowflake::Id;
use reqwest::Response;
use serde_json::json;

fn create_account_body(
    user_name: Option<&str>, mailing_address: Option<&str>, email: Option<&str>,
) -> serde_json::Value {
    let user_name = user_name.unwrap_or("neo");
    let email = email
        .map(|e| e.to_string())
        .unwrap_or_else(|| format!("{user_name}@example.com"));
    json!({
        "user_name": user_name,
        "mailing_address": mailing_address.unwrap_or("12 Seahawks Way, Renton, WA 98056, USA"),
        "email": email,
    })
}

fn create_money_body(amount: Money) -> serde_json::Value {
    // Money is serializable, so we could ser directly, but this way get to see JSON structure more clearly.
    json!({
        "amount": amount.amount.to_string(),
        "currency": amount.currency.to_string(),
    })
}

fn create_atm_withdrawal_body(atm_id: impl Into<AtmId>, amount: Money) -> serde_json::Value {
    json!({
        "atm_id": atm_id.into().to_string(),
        "amount": create_money_body(amount),
    })
}

fn create_check_withdrawal_body(
    check_nr: impl Into<CheckNumber>, amount: Money,
) -> serde_json::Value {
    json!({
        "check_nr": check_nr.into(),
        "amount": create_money_body(amount),
    })
}

#[tokio::test]
async fn create_bank_account_returns_a_200_and_account_id() {
    let app = spawn_latest_app().await;
    let body = create_account_body(Some("neo"), None, None);

    let response: Response = app.post_create_bank_account(body).await;

    let response_status = response.status();
    assert_eq!(assert_some!(response.content_length()), 19);
    assert_eq!(response_status, StatusCode::OK);
}

#[tokio::test]
async fn create_bank_account_persists_event_and_view() {
    let app = spawn_latest_app().await;
    let body = create_account_body(
        Some("otis"),
        Some("123 Main St., Springfield, IL, 61890"),
        None,
    );

    let response = app.post_create_bank_account(body).await;
    let account_id: AccountId = assert_ok!(response.json().await);
    let aggregate_id: Id<BankAccount> = account_id.into();

    let saved_event = assert_ok!(
        sqlx::query!(
            "SELECT aggregate_type, aggregate_id, sequence, event_type, event_version, payload, metadata FROM events WHERE aggregate_id = $1",
            aggregate_id.pretty()
        )
        .fetch_one(&app.db_pool)
        .await
    );

    assert_eq!(saved_event.aggregate_type, "account");
    assert_eq!(saved_event.aggregate_id, aggregate_id.pretty());
    assert_eq!(saved_event.sequence, 1);
    assert_eq!(saved_event.event_type, "account_opened");
    assert_eq!(saved_event.event_version, "1.0");
    assert_eq!(
        saved_event.payload,
        json!({
            "AccountOpened": {
                "account_id": account_id,
                "email": "otis@example.com",
                "mailing_address": "123 Main St., Springfield, IL, 61890",
                "user_name": "otis"
            }
        })
    );
    assert_ne!(
        saved_event.metadata["correlation_pretty_id"],
        serde_json::Value::Null
    );
    assert_ne!(
        saved_event.metadata["correlation_snowflake_id"],
        serde_json::Value::Null
    );
    assert_ne!(
        saved_event.metadata["recv_timestamp"],
        serde_json::Value::Null
    );

    let saved_view = assert_ok!(
        sqlx::query!(
            "SELECT view_id, version, payload FROM account_query WHERE view_id = $1",
            aggregate_id.pretty()
        )
        .fetch_one(&app.db_pool)
        .await
    );
    assert_eq!(saved_view.view_id, saved_event.aggregate_id);
    assert_eq!(saved_view.version, 1);
    assert_eq!(
        saved_view.payload,
        json!({
            "account_id": account_id,
            "balance": {
                "amount": "0",
                "currency": "USD"
            },
            "ledger": [],
            "written_checks": []
        })
    );
}

#[tokio::test]
async fn create_account_fails_if_there_is_a_fatal_database_error() {
    let app = spawn_latest_app().await;
    let body = create_account_body(None, None, None);

    assert_ok!(
        sqlx::query!("ALTER TABLE events DROP COLUMN aggregate_type;")
            .execute(&app.db_pool)
            .await
    );

    let response = app.post_create_bank_account(body).await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn deposit_amount_returns_a_200() {
    let app = spawn_latest_app().await;
    let body = create_money_body(Money::new(123456, 2, Currency::Usd));

    let response = app.post_create_bank_account(create_account_body(None, None, None)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let account_id: AccountId = assert_ok!(response.json().await);
    let response = app.post_deposit_amount(account_id, body).await;
    assert_eq!(response.status(), StatusCode::OK);
    let _ = assert_bank_account_detail(
        &app,
        account_id,
        BankAccountView {
            account_id: Some(account_id),
            balance: Money::new(123456, 2, Currency::Usd),
            ledger: vec![LedgerEntry::new(
                "deposit",
                Money::new(123456, 2, Currency::Usd),
            )],
            ..Default::default()
        },
    )
    .await;
}

#[tokio::test]
async fn atm_withdrawal_returns_a_200() {
    let app = spawn_latest_app().await;
    let response = app.post_create_bank_account(create_account_body(None, None, None)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let account_id: AccountId = assert_ok!(response.json().await);
    let response = app
        .post_deposit_amount(
            account_id,
            create_money_body(Money::new(1000, 2, Currency::Usd)),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = create_atm_withdrawal_body("abc_123", Money::new(923, 2, Currency::Usd));
    let response = app.post_atm_withdrawal(account_id, body).await;
    assert_eq!(response.status(), StatusCode::OK);

    let _ = assert_bank_account_detail(
        &app,
        account_id,
        BankAccountView {
            account_id: Some(account_id),
            balance: Money::new(77, 2, Currency::Usd),
            ledger: vec![
                LedgerEntry::new("deposit", Money::new(1000, 2, Currency::Usd)),
                LedgerEntry::new("ATM withdrawal", Money::new(-923, 2, Currency::Usd)),
            ],
            ..Default::default()
        },
    )
    .await;
}

#[tokio::test]
async fn check_withdrawal_returns_a_200() {
    let app = spawn_latest_app().await;
    let response = app.post_create_bank_account(create_account_body(None, None, None)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let account_id: AccountId = assert_ok!(response.json().await);
    let response = app
        .post_deposit_amount(
            account_id,
            create_money_body(Money::new(1000, 2, Currency::Usd)),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let check_nr = CheckNumber::new(873487_u32);
    let body = create_check_withdrawal_body(check_nr, Money::new(923, 2, Currency::Usd));
    let response = app.post_check_withdrawal(account_id, body).await;
    assert_eq!(response.status(), StatusCode::OK);

    let _ = assert_bank_account_detail(
        &app,
        account_id,
        BankAccountView {
            account_id: Some(account_id),
            balance: Money::new(77, 2, Currency::Usd),
            ledger: vec![
                LedgerEntry::new("deposit", Money::new(1000, 2, Currency::Usd)),
                LedgerEntry::new(
                    format!("Check {check_nr}"),
                    Money::new(-923, 2, Currency::Usd),
                ),
            ],
            written_checks: vec![CheckNumber::new(873487_u32)],
            ..Default::default()
        },
    )
    .await;
}

// redundant given other tests in this module
#[tokio::test]
async fn account_view_updates_with_commands() {
    let app = spawn_latest_app().await;

    // -- create bank account
    let body = create_account_body(Some("stella"), None, None);
    let response = app.post_create_bank_account(body).await;
    assert_eq!(response.status(), StatusCode::OK);
    let account_id: AccountId = assert_ok!(response.json().await);
    let expected = BankAccountView { account_id: Some(account_id), ..Default::default() };
    let e_created = assert_bank_account_detail(&app, account_id, expected).await;

    // -- deposit money
    let deposit = Money::new(435987, 2, Currency::Usd);
    let response = app.post_deposit_amount(account_id, create_money_body(deposit)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let _e_deposited = assert_bank_account_detail(
        &app,
        account_id,
        BankAccountView {
            balance: deposit,
            ledger: vec![LedgerEntry::new("deposit", deposit)],
            ..e_created
        },
    )
    .await;
}

async fn assert_bank_account_detail(
    app: &TestApp, account_id: AccountId, expected: BankAccountView,
) -> BankAccountView {
    let response = app.get_serve_bank_account(account_id).await;
    assert_eq!(response.status(), StatusCode::OK);
    let actual: BankAccountView = assert_ok!(response.json().await);
    assert_eq!(actual, expected);
    actual
}
