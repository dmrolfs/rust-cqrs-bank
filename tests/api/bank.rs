use crate::helpers::{spawn_app, spawn_latest_app};
use axum::http::StatusCode;
use reqwest::Response;
use serde_json::json;
// use tokio_test::assert_ok;
use claim::{assert_ok, assert_some};
use trim_margin::MarginTrimmable;

#[tokio::test]
async fn create_bank_account_returns_a_200_and_account_id() {
    let app = spawn_latest_app().await;
    let body = json!({
        "user_name": "neo",
        "mailing_address": "12 Seahawks Way, Renton, WA 98056, USA",
        "email": "neocat@example.com"
    });

    let response: Response = app.post_create_bank_account(body).await;

    let response_status = response.status();
    assert_eq!(assert_some!(response.content_length()), 19);
    assert_eq!(response_status, StatusCode::OK);
}
