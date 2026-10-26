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

/// Forbidden function/token signatures in console queries.
const FORBIDDEN: &[&str] = &[
    "\"TABLE_FUNCTION\"",
    "read_csv",
    "read_parquet",
    "read_json",
    "read_blob",
    "read_text",
    "parquet_scan",
    "scan_parquet",
    "scan_csv",
    "glob",
    "sniff_csv",
    "current_setting",
    "set_variable",
    "get_variable",
    "load_extension",
    "install_extension",
    "checkpoint",
    "write_csv",
];

/// Read-only SQL console. Accepts a single plain `SELECT` without table functions.
pub struct Console;

impl Console {
    /// Validates and runs `sql`.
    pub fn run(conn: &Connection, sql: &str) -> Result<QueryResult, StoreError> {
        Self::ensure_plain_select(conn, sql)?;
        let mut stmt = conn.prepare(sql).map_err(|e| StoreError::Invalid(e.to_string()))?;
        let mut rows = stmt.query([]).map_err(|e| StoreError::Invalid(e.to_string()))?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(|e| StoreError::Invalid(e.to_string()))? {
            out.push(Self::read_row(row)?);
            if out.len() > ROW_LIMIT {
                break;
            }
        }
        let truncated = out.len() > ROW_LIMIT;
        out.truncate(ROW_LIMIT);
        Ok(QueryResult { columns: stmt.column_names(), rows: out, truncated })
    }

    fn read_row(row: &duckdb::Row<'_>) -> Result<Vec<Json>, StoreError> {
        let cols = (0..row.as_ref().column_count())
            .map(|i| row.get::<_, Value>(i).map(Self::to_json))
            .collect::<duckdb::Result<Vec<_>>>()
            .map_err(|e| StoreError::Invalid(e.to_string()))?;
        Ok(cols)
    }

    fn ensure_plain_select(conn: &Connection, sql: &str) -> Result<(), StoreError> {
        let q = "SELECT CAST(json_serialize_sql(?) AS VARCHAR)";
        let parsed: String = conn.query_row(q, [sql], |r| r.get(0)).map_err(|e| StoreError::Invalid(e.to_string()))?;
        let tree: Json = serde_json::from_str(&parsed).map_err(|e| StoreError::Invalid(e.to_string()))?;
        let single = tree["error"] == json!(false) && tree["statements"].as_array().is_some_and(|s| s.len() == 1);
        let is_select = tree["statements"][0]["node"]["type"] == "SELECT_NODE";
        let safe = single && is_select && !FORBIDDEN.iter().any(|f| parsed.contains(f));
        match safe {
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
