//! Small SQL helpers shared by repositories and the loader.

use crate::StoreError;
use duckdb::{Connection, Params, Row, ToSql};

/// Collects every row of `sql` through `map`.
pub(crate) fn query<T, P: Params>(
    conn: &Connection,
    sql: &str,
    params: P,
    map: impl FnMut(&Row<'_>) -> duckdb::Result<T>,
) -> Result<Vec<T>, StoreError> {
    let mut stmt = conn.prepare_cached(sql)?;
    let rows = stmt.query_map(params, map)?;
    Ok(rows.collect::<duckdb::Result<Vec<T>>>()?)
}

/// Runs an `INSERT … RETURNING id`.
pub(crate) fn insert_id<P: Params>(conn: &Connection, sql: &str, params: P) -> Result<i64, StoreError> {
    Ok(conn.query_row(sql, params, |r| r.get(0))?)
}

/// Runs a statement that must touch exactly one row, else `NotFound(what)`.
pub(crate) fn touch_one<P: Params>(conn: &Connection, sql: &str, params: P, what: &str) -> Result<(), StoreError> {
    match conn.execute(sql, params)? {
        0 => Err(StoreError::NotFound(what.to_owned())),
        _ => Ok(()),
    }
}

/// True when `sql` (a `SELECT 1 … LIMIT 1`) returns a row.
pub(crate) fn exists<P: Params>(conn: &Connection, sql: &str, params: P) -> Result<bool, StoreError> {
    let mut stmt = conn.prepare_cached(sql)?;
    Ok(stmt.exists(params)?)
}

/// Fails with `Invalid` unless row `id` exists in `table` (a trusted, static name).
pub(crate) fn require(conn: &Connection, table: &'static str, id: i64) -> Result<(), StoreError> {
    let sql = format!("SELECT 1 FROM {table} WHERE id = ? LIMIT 1");
    match exists(conn, &sql, [id])? {
        true => Ok(()),
        false => Err(StoreError::Invalid(format!("{table} {id} does not exist"))),
    }
}

/// Fails with `Conflict` when any `(table, column)` still references `id`.
pub(crate) fn unreferenced(conn: &Connection, refs: &[(&'static str, &'static str)], id: i64, what: &str) -> Result<(), StoreError> {
    for (table, column) in refs {
        let sql = format!("SELECT 1 FROM {table} WHERE {column} = ? LIMIT 1");
        if exists(conn, &sql, [id])? {
            return Err(StoreError::Conflict(format!("{what} is still referenced by {table}")));
        }
    }
    Ok(())
}

/// Optional ciphertext as a bindable string.
pub(crate) fn opt_ct(value: &Option<jizifin_domain::Ciphertext>) -> Option<&str> {
    value.as_ref().map(jizifin_domain::Ciphertext::as_str)
}

/// Boxed parameter list for dynamically built queries.
pub(crate) type DynParams = Vec<Box<dyn ToSql>>;
