mod common;

use axum::http::StatusCode;
use common::{json_body, TestClient, TEST_PROOF};
use serde_json::Value;

#[tokio::test]
async fn auth_status_reports_uninitialized_initially() {
    let client = TestClient::uninitialized().await;
    let resp = client.get("/auth/status").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let val: Value = json_body(resp).await;
    assert_eq!(val["initialized"], false);
    assert_eq!(val["authenticated"], false);
}

#[tokio::test]
async fn auth_init_rejects_invalid_proofs() {
    let client = TestClient::uninitialized().await;
    let short = serde_json::json!({ "proof": "too-short" });
    let resp = client.post_json("/auth/init", &short).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let space = serde_json::json!({ "proof": "invalid with spaces and length" });
    let resp = client.post_json("/auth/init", &space).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn auth_login_and_logout_lifecycle() {
    let client = TestClient::authenticated().await;
    let login_body = serde_json::json!({ "proof": TEST_PROOF });
    let login_resp = client.post_json("/auth/login", &login_body).await;
    assert_eq!(login_resp.status(), StatusCode::OK);

    let wrong_body = serde_json::json!({ "proof": "wrong-proof-must-be-long-enough" });
    let wrong_resp = client.post_json("/auth/login", &wrong_body).await;
    assert_eq!(wrong_resp.status(), StatusCode::UNAUTHORIZED);

    let member_resp = client.get("/members").await;
    assert_eq!(member_resp.status(), StatusCode::OK);

    let logout_resp = client.post_json("/auth/logout", &serde_json::json!({})).await;
    assert_eq!(logout_resp.status(), StatusCode::OK);

    let after_logout = client.get("/members").await;
    assert_eq!(after_logout.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn auth_reset_resets_instance() {
    let client = TestClient::authenticated().await;
    let reset_body = serde_json::json!({ "proof": TEST_PROOF });
    let reset_resp = client.post_json("/auth/reset", &reset_body).await;
    assert_eq!(reset_resp.status(), StatusCode::OK);

    let status_resp = client.get("/auth/status").await;
    assert_eq!(status_resp.status(), StatusCode::OK);
    let val: Value = json_body(status_resp).await;
    assert_eq!(val["initialized"], false);
}
