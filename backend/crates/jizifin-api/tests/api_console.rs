mod common;

use axum::http::StatusCode;
use common::{json_body, TestClient};
use serde_json::Value;

#[tokio::test]
async fn console_executes_plain_select() {
    let client = TestClient::authenticated().await;
    let query = serde_json::json!({ "sql": "SELECT 42 AS answer, 'hello' AS greeting" });
    let resp = client.post_json("/query", &query).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let val: Value = json_body(resp).await;
    assert_eq!(val["columns"], serde_json::json!(["answer", "greeting"]));
    assert_eq!(val["rows"][0][0], 42);
    assert_eq!(val["rows"][0][1], "hello");
    assert_eq!(val["truncated"], false);
}

#[tokio::test]
async fn console_rejects_mutations_and_table_functions() {
    let client = TestClient::authenticated().await;
    let bad = [
        "INSERT INTO members (name_ct, color, is_active) VALUES ('test', '#000', true)",
        "DROP TABLE members",
        "SELECT 1; SELECT 2",
        "SELECT * FROM read_csv('/tmp/nonexistent.csv')",
        "COPY members TO '/tmp/out.csv'",
        "ATTACH '/tmp/x.db' AS x",
    ];
    for sql in bad {
        let resp = client.post_json("/query", &serde_json::json!({ "sql": sql })).await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[tokio::test]
async fn console_rejects_external_file_probing() {
    let client = TestClient::authenticated().await;
    let probes = [
        "SELECT * FROM read_parquet('/tmp/test.parquet')",
        "SELECT * FROM glob('*.csv')",
        "SELECT read_blob('/etc/passwd')",
        "SELECT READ_BLOB('/etc/passwd')",
        "SELECT READ_TEXT('/tmp/secret.txt')",
        "SELECT * FROM 'file.csv'",
    ];
    for sql in probes {
        let resp = client.post_json("/query", &serde_json::json!({ "sql": sql })).await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY, "expected rejection for {sql}");
    }
}

#[tokio::test]
async fn console_rejects_system_and_catalog_functions() {
    let client = TestClient::authenticated().await;
    let queries = [
        "PRAGMA version",
        "SELECT * FROM duckdb_settings()",
        "SELECT * FROM duckdb_tables()",
        "DETACH jizifin_backup",
        "CALL current_setting('threads')",
        "SELECT current_setting('threads')",
    ];
    for sql in queries {
        let resp = client.post_json("/query", &serde_json::json!({ "sql": sql })).await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY, "expected rejection for {sql}");
    }
}

#[tokio::test]
async fn console_runtime_errors_return_unprocessable_entity() {
    let client = TestClient::authenticated().await;
    let resp = client.post_json("/query", &serde_json::json!({ "sql": "SELECT CAST('abc' AS INTEGER)" })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

