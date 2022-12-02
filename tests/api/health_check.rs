use crate::helpers::{spawn_app, spawn_latest_app};
use axum::http::StatusCode;
use pretty_assertions::assert_eq;
use tokio_test::assert_ok;

#[tokio::test]
async fn health_check_works() {
    let app = spawn_latest_app().await;
    let client = app.api_client;
    let version = app.version;

    let url = format!("{}/api/{version}/health", app.http_address);
    assert_eq!(url, format!("http://0.0.0.0:{}/api/v1/health", app.port));
    let response = assert_ok!(client.get(url).send().await);

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        assert_ok!(response.text().await),
        assert_ok!(serde_json::to_string(
            &maplit::hashmap! { "status" => "Up", }
        ))
    );
}
