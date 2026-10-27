use crate::StoreError;
use duckdb::Connection;
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
    "app_config",
    "sessions",
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
            .map(|i| {
                let r = row.get_ref(i);
                Self::val_to_json(r)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| StoreError::Invalid(e.to_string()))?;
        Ok(cols)
    }

    fn val_to_json(val: Result<duckdb::types::ValueRef<'_>, duckdb::Error>) -> Result<Json, duckdb::Error> {
        let v = val?;
        Ok(Self::ref_to_json(v))
    }

    fn ref_to_json(v: duckdb::types::ValueRef<'_>) -> Json {
        use duckdb::types::ValueRef;
        match v {
            ValueRef::Null => Json::Null,
            ValueRef::Boolean(b) => json!(b),
            ValueRef::TinyInt(n) => json!(n),
            ValueRef::SmallInt(n) => json!(n),
            ValueRef::Int(n) => json!(n),
            ValueRef::BigInt(n) => json!(n),
            ValueRef::Float(n) => json!(n),
            ValueRef::Double(n) => json!(n),
            ValueRef::Text(s) => json!(String::from_utf8_lossy(s)),
            ValueRef::Blob(b) => json!(format!("<blob {}B>", b.len())),
            ValueRef::Date32(d) => Self::date_to_json(d),
            ValueRef::Timestamp(unit, val) => Self::timestamp_to_json(unit, val),
            ValueRef::Time64(unit, t) => json!(format!("{t} {unit:?}")),
            _ => json!(format!("{v:?}")),
        }
    }

    fn date_to_json(d: i32) -> Json {
        chrono::NaiveDate::from_num_days_from_ce_opt(d + 719_163)
            .map(|dt| json!(dt.format("%Y-%m-%d").to_string()))
            .unwrap_or_else(|| json!(d))
    }

    fn timestamp_to_json(unit: duckdb::types::TimeUnit, val: i64) -> Json {
        let (s, ns) = Self::split_timestamp(unit, val);
        chrono::DateTime::from_timestamp(s, ns)
            .map(|dt| json!(dt.to_rfc3339()))
            .unwrap_or_else(|| json!(val))
    }

    fn split_timestamp(unit: duckdb::types::TimeUnit, val: i64) -> (i64, u32) {
        use duckdb::types::TimeUnit::*;
        match unit {
            Second => (val, 0),
            Millisecond => (val / 1_000, ((val % 1_000) * 1_000_000) as u32),
            Microsecond => (val / 1_000_000, ((val % 1_000_000) * 1_000) as u32),
            Nanosecond => (val / 1_000_000_000, (val % 1_000_000_000) as u32),
        }
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
}
