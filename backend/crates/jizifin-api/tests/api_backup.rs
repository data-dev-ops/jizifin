mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::TestClient;

#[tokio::test]
async fn backup_restore_rejects_corrupted_payload() {
    let client = TestClient::authenticated().await;
    let req = Request::post("/backup")
        .header("authorization", format!("Bearer {}", client.token))
        .header("content-type", "application/octet-stream")
        .body(Body::from(b"corrupted binary payload".to_vec()))
        .unwrap();
    let resp = client.request(req).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn backup_restore_rejects_version_mismatch() {
    let client = TestClient::authenticated().await;
    let path = std::env::temp_dir().join(format!("api_mismatch_{}.duckdb", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let conn = jizifin_store::Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TABLE schema_version (version BIGINT); INSERT INTO schema_version VALUES (999);").unwrap();
    drop(conn);
    let bytes = std::fs::read(&path).unwrap();
    let _ = std::fs::remove_file(&path);

    let req = Request::post("/backup")
        .header("authorization", format!("Bearer {}", client.token))
        .header("content-type", "application/octet-stream")
        .body(Body::from(bytes))
        .unwrap();
    let resp = client.request(req).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn backup_export_and_restore_lifecycle() {
    let client = TestClient::authenticated().await;
    let resp = client.get("/backup").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = http_body_util::BodyExt::collect(resp.into_body()).await.unwrap().to_bytes();
    assert!(!bytes.is_empty());

    let req = Request::post("/backup")
        .header("authorization", format!("Bearer {}", client.token))
        .header("content-type", "application/octet-stream")
        .body(Body::from(bytes))
        .unwrap();
    let restore_resp = client.request(req).await;
    assert_eq!(restore_resp.status(), StatusCode::NO_CONTENT);
}
