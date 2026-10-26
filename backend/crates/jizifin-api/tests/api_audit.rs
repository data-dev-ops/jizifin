mod common;

use axum::http::StatusCode;
use common::{json_body, TestClient};
use serde_json::Value;

async fn setup_household(client: &TestClient) -> (i64, i64, i64) {
    let m1_resp = client.post_json("/members", &serde_json::json!({
        "name": "Member_One", "color": "#111111", "is_active": true
    })).await;
    let m1: Value = json_body(m1_resp).await;
    let m1_id = m1["id"].as_i64().unwrap();

    let m2_resp = client.post_json("/members", &serde_json::json!({
        "name": "Member_Two", "color": "#222222", "is_active": true
    })).await;
    let m2: Value = json_body(m2_resp).await;
    let m2_id = m2["id"].as_i64().unwrap();

    let c_resp = client.post_json("/categories", &serde_json::json!({
        "kind": "expense", "name": "Shared_Groceries", "pay_mode": "shared"
    })).await;
    let c: Value = json_body(c_resp).await;
    let c_id = c["id"].as_i64().unwrap();

    (m1_id, m2_id, c_id)
}

#[tokio::test]
async fn audit_settlements_live_and_explain() {
    let client = TestClient::authenticated().await;
    let (m1, _m2, cat) = setup_household(&client).await;

    // Member 1 pays 10000 cents for shared groceries in July 2026
    let entry = serde_json::json!({
        "kind": "expense", "date": "2026-07-10", "amount": 10000,
        "member": m1, "category": cat
    });
    let resp = client.post_json("/entries", &entry).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Fetch live settlement
    let resp = client.get("/settlements/2026-07").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let val: Value = json_body(resp).await;
    assert_eq!(val["state"], "live");
    assert!(val["value"]["net"].is_object());
    assert_eq!(val["value"]["net"]["1"], 5000);
    assert_eq!(val["value"]["net"]["2"], -5000);

    // Fetch with explain=1
    let resp = client.get("/settlements/2026-07?explain=1").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let explain_val: Value = json_body(resp).await;
    assert_eq!(explain_val["state"], "live");
}

#[tokio::test]
async fn audit_budgets_cashflow_and_splits_endpoints() {
    let client = TestClient::authenticated().await;
    let (_m1, _m2, _cat) = setup_household(&client).await;

    // Budgets status
    let resp = client.get("/budgets/status?month=2026-07").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let b_val: Value = json_body(resp).await;
    assert!(b_val["value"]["rows"].is_array());
    assert!(b_val["value"]["summary"].is_object());

    // Cashflow
    let resp = client.get("/cashflow?month=2026-07").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let cf_val: Value = json_body(resp).await;
    assert!(cf_val["value"]["income"].is_number());
    assert!(cf_val["value"]["spent"].is_number());

    // Splits effective
    let resp = client.get("/splits/effective?month=2026-07").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let sp_val: Value = json_body(resp).await;
    assert!(sp_val["value"]["categories"].is_array());
}
