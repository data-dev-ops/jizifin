use crate::StoreError;
use duckdb::Connection;
use duckdb::types::Value;
use serde::Serialize;
use serde_json::{Value as Json, json};

/// Maximum rows a console query returns.
const ROW_LIMIT: usize = 1000;

/// Tabular console result.
#[derive(Debug, Serialize)]
pub struct QueryResult {
    /// Column names.
    pub columns: Vec<String>,
    /// Rows as JSON values.
    pub rows: Vec<Vec<Json>>,
    /// True when more rows existed than were returned.
    pub truncated: bool,
}

/// Read-only SQL console. Accepts a single plain `SELECT` without table functions.
pub struct Console;

impl Console {
    /// Validates and runs `sql`.
    pub fn run(conn: &Connection, sql: &str) -> Result<QueryResult, StoreError> {
        Self::ensure_plain_select(conn, sql)?;
        let mut stmt = conn.prepare(sql)?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push((0..row.as_ref().column_count()).map(|i| row.get::<_, Value>(i).map(Self::to_json)).collect::<duckdb::Result<Vec<_>>>()?);
        }
        let truncated = out.len() > ROW_LIMIT;
        out.truncate(ROW_LIMIT);
        Ok(QueryResult { columns: stmt.column_names(), rows: out, truncated })
    }

    fn ensure_plain_select(conn: &Connection, sql: &str) -> Result<(), StoreError> {
        let parsed: String = conn.query_row("SELECT CAST(json_serialize_sql(?) AS VARCHAR)", [sql], |r| r.get(0))?;
        let tree: Json = serde_json::from_str(&parsed).map_err(|e| StoreError::Invalid(e.to_string()))?;
        let single = tree["error"] == json!(false) && tree["statements"].as_array().is_some_and(|s| s.len() == 1);
        match single && !parsed.contains("\"TABLE_FUNCTION\"") {
            true => Ok(()),
            false => Err(StoreError::Invalid("only a single SELECT over tables is allowed".into())),
        }
    }

    fn to_json(value: Value) -> Json {
        match value {
            Value::Null => Json::Null,
            Value::Boolean(b) => json!(b),
            Value::TinyInt(n) => json!(n),
            Value::SmallInt(n) => json!(n),
            Value::Int(n) => json!(n),
            Value::BigInt(n) => json!(n),
            Value::Float(n) => json!(n),
            Value::Double(n) => json!(n),
            Value::Text(s) => json!(s),
            other => json!(format!("{other:?}")),
        }
    }
}
