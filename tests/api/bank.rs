use crate::helpers::spawn_latest_app;
use axum::http::StatusCode;
use bankaccount::{AccountId, BankAccount, BankAccountView};
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
    let body = json!({
        "amount": "1234.56",
        "currency": "USD"
    });

    let response = app.post_create_bank_account(create_account_body(None, None, None)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let account_id: AccountId = assert_ok!(response.json().await);
    let response = app.post_deposit_amount(account_id, body).await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn account_view_updates_with_commands() {
    let app = spawn_latest_app().await;
    let body = create_account_body(Some("stella"), None, None);

    let response = app.post_create_bank_account(body).await;
    assert_eq!(response.status(), StatusCode::OK);
    let account_id: AccountId = assert_ok!(response.json().await);

    let response = app.get_serve_bank_account(account_id).await;
    assert_eq!(response.status(), StatusCode::OK);
    let account_view: BankAccountView = assert_ok!(response.json().await);
    assert_eq!(
        account_view,
        BankAccountView {
            account_id: Some(account_id),
            balance: Money::new(0, 2, Currency::Usd),
            written_checks: vec![],
            ledger: vec![],
        }
    )
}
