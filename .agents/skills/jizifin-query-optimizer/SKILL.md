---
name: jizifin-query-optimizer
description: Inspects and optimizes DuckDB queries, indexes and views in the jizifin Rust backend while preserving the deterministic AES-GCM column rules.
---

# Jizifin Query & Plan Optimizer (DuckDB)

Use this when you write or change SQL in `backend/crates/jizifin-store` (repositories, `loader.rs`, `migrations/*.sql`).

## Goals

1. No full scans of `entries` for month, member, category, account or project lookups. The indexes are listed in `AGENTS.md` §3.
2. Keep date filters SARGable: compare `entry_date` to bounds; never wrap it in a function (`date_trunc` belongs only in `GROUP BY` / `SELECT` of aggregates, not in `WHERE`).
3. On `*_ct` (ciphertext) columns use only equality, `IN`, joins and `GROUP BY`. Never range filters, `LIKE` or collation `ORDER BY`.
4. Bound lists must be cast: `list_contains(CAST(? AS BIGINT[]), id)` (an empty uncast list fails to bind).
5. The snapshot loader for one month must stay well under 20 ms (measured p95 8.9 ms on 100k entries).

## Inspecting a plan

Through the authenticated SQL console (`POST /query`, plain `SELECT` only) or the Query Console tab:

```sql
EXPLAIN SELECT id FROM entries WHERE entry_date >= DATE '2026-09-01' AND entry_date <= DATE '2026-09-30'
```

Only single `SELECT` statements without table functions are accepted; for `EXPLAIN ANALYZE` or `INSERT` plans, run the statement from a scratch Rust example in the backend container, never against the live file.

## Schema changes

Add a new numbered file under `jizifin-store/migrations/`, bump `LATEST_VERSION` in `migrate.rs`, and keep `schema::TABLES` (backup/restore order) in step. Declare no foreign keys (DuckDB blocks updating a UNIQUE column of a referenced parent).

## Verification

`make lint-backend`, then check the change on a migrated scratch database or the running API; update `AGENTS.md` and the plan when an index or table changes.
