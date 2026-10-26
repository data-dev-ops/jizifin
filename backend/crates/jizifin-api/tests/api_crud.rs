mod common;

use axum::http::StatusCode;
use common::{json_body, TestClient};
use serde_json::Value;

async fn create_member(client: &TestClient, name: &str, color: &str) -> i64 {
    let body = serde_json::json!({ "name": name, "color": color, "is_active": true });
    let resp = client.post_json("/members", &body).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let val: Value = json_body(resp).await;
    val["id"].as_i64().unwrap()
}

async fn create_category(client: &TestClient, name: &str, kind: &str, pay_mode: &str) -> i64 {
    let body = serde_json::json!({ "kind": kind, "name": name, "pay_mode": pay_mode });
    let resp = client.post_json("/categories", &body).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let val: Value = json_body(resp).await;
    val["id"].as_i64().unwrap()
}

#[tokio::test]
async fn crud_members_and_categories_lifecycle() {
    let client = TestClient::authenticated().await;
    let m_id = create_member(&client, "Alice", "#111111").await;
    let c_id = create_category(&client, "Groceries", "expense", "shared").await;

    let resp = client.get("/members").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let members: Vec<Value> = json_body(resp).await;
    assert_eq!(members.len(), 1);
    assert_eq!(members[0]["id"], m_id);

    let update = serde_json::json!({ "name": "Alice_Smith", "color": "#222222", "is_active": true });
    let resp = client.put_json(&format!("/members/{m_id}"), &update).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = client.get("/categories").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let cats: Vec<Value> = json_body(resp).await;
    assert_eq!(cats.len(), 1);
    assert_eq!(cats[0]["id"], c_id);
}

#[tokio::test]
async fn crud_entries_single_and_duplicate_probe() {
    let client = TestClient::authenticated().await;
    let m_id = create_member(&client, "Bob", "#333333").await;
    let c_id = create_category(&client, "Transport", "expense", "shared").await;

    let entry = serde_json::json!({
        "kind": "expense", "date": "2026-06-15", "amount": 4500,
        "member": m_id, "category": c_id, "name": "Train_Ticket"
    });
    let resp = client.post_json("/entries", &entry).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let e_val: Value = json_body(resp).await;
    let e_id = e_val["id"].as_i64().unwrap();

    let resp = client.get(&format!("/entries/{e_id}")).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let probe = serde_json::json!([{
        "category": c_id, "amount": 4500, "date": "2026-06-20", "exclude": null
    }]);
    let resp = client.post_json("/entries/duplicates", &probe).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let dupes: Vec<Value> = json_body(resp).await;
    assert_eq!(dupes.len(), 1);
}

#[tokio::test]
async fn crud_entries_batch_insert() {
    let client = TestClient::authenticated().await;
    let m_id = create_member(&client, "Bob2", "#333333").await;
    let c_id = create_category(&client, "Transport2", "expense", "shared").await;

    let batch = serde_json::json!([
        { "kind": "expense", "date": "2026-06-16", "amount": 1200, "member": m_id, "category": c_id },
        { "kind": "expense", "date": "2026-06-17", "amount": 1500, "member": m_id, "category": c_id }
    ]);
    let resp = client.post_json("/entries/batch", &batch).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn crud_tags_window_integrity_rejection() {
    let client = TestClient::authenticated().await;
    let m_id = create_member(&client, "Charlie", "#444444").await;
    let c_id = create_category(&client, "Books", "expense", "shared").await;

    let tag_body = serde_json::json!({ "name": "Summer_Reading", "valid_from": "2026-06-01", "valid_to": "2026-06-30" });
    let resp = client.post_json("/tags", &tag_body).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let t_val: Value = json_body(resp).await;
    let tag_id = t_val["id"].as_i64().unwrap();

    let inside = serde_json::json!({ "kind": "expense", "date": "2026-06-10", "amount": 2000, "member": m_id, "category": c_id, "tag": tag_id });
    let resp = client.post_json("/entries", &inside).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let outside = serde_json::json!({ "kind": "expense", "date": "2026-07-05", "amount": 2000, "member": m_id, "category": c_id, "tag": tag_id });
    let resp = client.post_json("/entries", &outside).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn crud_sealed_month_blocks_mutations_and_referenced_deletes() {
    let client = TestClient::authenticated().await;
    let m_id = create_member(&client, "Dana", "#555555").await;
    let c_id = create_category(&client, "Coffee", "expense", "shared").await;

    let entry = serde_json::json!({ "kind": "expense", "date": "2026-05-10", "amount": 500, "member": m_id, "category": c_id });
    let resp = client.post_json("/entries", &entry).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let seal_resp = client.post_json("/settlements/2026-05/seal", &serde_json::json!({})).await;
    assert_eq!(seal_resp.status(), StatusCode::CREATED);

    let late = serde_json::json!({ "kind": "expense", "date": "2026-05-20", "amount": 700, "member": m_id, "category": c_id });
    let resp = client.post_json("/entries", &late).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    let del_resp = client.delete(&format!("/categories/{c_id}")).await;
    assert_eq!(del_resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn unseal_reopens_a_sealed_month() {
    let client = TestClient::authenticated().await;
    let m_id = create_member(&client, "Eve", "#666666").await;
    let c_id = create_category(&client, "Tea", "expense", "shared").await;
    let late = serde_json::json!({ "kind": "expense", "date": "2026-05-20", "amount": 700, "member": m_id, "category": c_id });
    assert_eq!(client.post_json("/settlements/2026-05/seal", &serde_json::json!({})).await.status(), StatusCode::CREATED);
    assert_eq!(client.post_json("/entries", &late).await.status(), StatusCode::CONFLICT);

    assert_eq!(client.delete("/settlements/2026-05/seal").await.status(), StatusCode::NO_CONTENT);
    assert_eq!(client.post_json("/entries", &late).await.status(), StatusCode::CREATED);
    assert_eq!(client.delete("/settlements/2026-05/seal").await.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn member_filter_does_not_flag_drift_on_sealed_month() {
    let client = TestClient::authenticated().await;
    let m1 = create_member(&client, "Fay", "#777777").await;
    let _m2 = create_member(&client, "Gus", "#888888").await;
    let c_id = create_category(&client, "Bread", "expense", "shared").await;
    let entry = serde_json::json!({ "kind": "expense", "date": "2026-05-10", "amount": 900, "member": m1, "category": c_id });
    assert_eq!(client.post_json("/entries", &entry).await.status(), StatusCode::CREATED);
    client.post_json("/settlements/2026-05/seal", &serde_json::json!({})).await;

    let resp = client.get(&format!("/settlements/2026-05?members={m1}")).await;
    let body: serde_json::Value = common::json_body(resp).await;
    assert_eq!(body["state"], "sealed");
    assert_eq!(body["drift"], false);
}
