use crate::sql::{exists, query};
use crate::StoreError;
use chrono::{Duration, NaiveDateTime, Utc};
use duckdb::{Connection, params};

/// `app_config` key holding the SHA-256 of the client login proof.
const PROOF_KEY: &str = "auth_proof_sha256";

/// Instance initialization and proof storage.
pub struct Auth;

impl Auth {
    /// Stored proof hash, `None` before first boot.
    pub fn proof_hash(conn: &Connection) -> Result<Option<String>, StoreError> {
        let found = query(conn, "SELECT value FROM app_config WHERE key = ?", [PROOF_KEY], |r| r.get(0))?;
        Ok(found.into_iter().next())
    }

    /// Stores the proof hash on first boot.
    pub fn initialize(conn: &Connection, proof_hash: &str) -> Result<(), StoreError> {
        if Self::proof_hash(conn)?.is_some() {
            return Err(StoreError::Conflict("already initialized".into()));
        }
        conn.execute("INSERT INTO app_config (key, value) VALUES (?, ?)", params![PROOF_KEY, proof_hash])?;
        Ok(())
    }

    /// Wipes every table, returning the instance to first boot.
    pub fn reset(conn: &Connection) -> Result<(), StoreError> {
        crate::schema::clear(conn)?;
        conn.execute("DELETE FROM sessions", [])?;
        Ok(())
    }
}

/// Server-side sessions keyed by the SHA-256 of the bearer token.
pub struct Sessions;

impl Sessions {
    /// Records a new session.
    pub fn create(conn: &Connection, token_hash: &str) -> Result<(), StoreError> {
        conn.execute("INSERT INTO sessions (token_sha256, last_active) VALUES (?, ?)", params![token_hash, Self::now()])?;
        Ok(())
    }

    /// True when the session exists and was active within `ttl`; refreshes it.
    pub fn touch(conn: &Connection, token_hash: &str, ttl: Duration) -> Result<bool, StoreError> {
        let cutoff = Self::now() - ttl;
        let sql = "SELECT 1 FROM sessions WHERE token_sha256 = ? AND last_active >= ? LIMIT 1";
        let alive = exists(conn, sql, params![token_hash, cutoff])?;
        if alive {
            conn.execute("UPDATE sessions SET last_active = ? WHERE token_sha256 = ?", params![Self::now(), token_hash])?;
        }
        Ok(alive)
    }

    /// Ends a session.
    pub fn delete(conn: &Connection, token_hash: &str) -> Result<(), StoreError> {
        conn.execute("DELETE FROM sessions WHERE token_sha256 = ?", [token_hash])?;
        Ok(())
    }

    /// Drops sessions idle longer than `ttl`.
    pub fn sweep(conn: &Connection, ttl: Duration) -> Result<usize, StoreError> {
        Ok(conn.execute("DELETE FROM sessions WHERE last_active < ?", [Self::now() - ttl])?)
    }

    fn now() -> NaiveDateTime {
        Utc::now().naive_utc()
    }
}
