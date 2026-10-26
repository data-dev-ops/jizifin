# Backend Technical Documentation

The backend is a Rust workspace (`backend/`): an axum/tokio HTTP and WebSocket server over an embedded DuckDB 2.0 file, with an in-process **Auditor** that owns every contentious calculation. The design history, complete API tables and spike results are in [docs/RUST_MIGRATION_PLAN.md](../../docs/RUST_MIGRATION_PLAN.md); working rules are in [AGENTS.md](../../AGENTS.md).

## 1. Crates

| Crate | Role |
|---|---|
| `jizifin-domain` | Pure types, no IO: `Cents`, `Month`, `Ciphertext` (validated Base64URL), typed ids, entities |
| `jizifin-auditor` | Calculators, invariants, snapshot cache, `AuditedResult` |
| `jizifin-store` | Numbered SQL migrations, one repository struct per aggregate, backup/restore, `DuckSnapshotLoader` |
| `jizifin-api` | Routes, auth and sessions, WebSocket hub, scheduler, SQL console |
| `jizifin-wasm` | CSV bank-statement parser built for the browser |

Clippy enforces small methods workspace-wide: at most 20 lines, 4 parameters and cognitive complexity 8, with all warnings denied.

## 2. Database

- DuckDB 2.0 via the `duckdb` crate, pooled with r2d2. Every call runs in `spawn_blocking`.
- **Single writer:** `Database::exclusive()` serialises writes, because DuckDB aborts overlapping updates of the same row. Reads are not locked. `Database::write` bumps the data version that invalidates Auditor snapshots; `write_meta` (sessions) does not.
- **No foreign keys.** DuckDB blocks updating a UNIQUE column of a referenced parent row. Repositories enforce integrity (`require`, `unreferenced`) and return 409 for deletes of referenced rows.
- Cents are `BIGINT`; dates are `DATE`; ciphertext columns end in `_ct`.
- Schema: `crates/jizifin-store/migrations/0001_init.sql` (20 tables, sequences, indexes); `LATEST_VERSION` in `migrate.rs`. `schema::TABLES` lists tables in dependency order for backup, restore and reset.
- The Query Console accepts one plain `SELECT` (checked with `json_serialize_sql`), no table functions, at most 1000 rows.
- Backups (`GET /backup`, `POST /backup`) are DuckDB files containing ciphertext and numbers; sessions are excluded; restore advances every sequence past the restored maximum.

## 3. Cryptographic split

The browser derives an AES-256-GCM key (PBKDF2, 100,000 iterations, SHA-256, salt `"jizifin-salt-pbkdf2"`) with the static IV `"jizifin-cryp"` and encrypts text fields; ciphertext is Base64URL without padding. Login sends `encryptText("FinanceTrackerAuth")` as proof, and the server stores only its SHA-256. On `*_ct` columns use only equality, `IN`, joins and `GROUP BY`; never range filters, `LIKE` or collation `ORDER BY`.

## 4. The Auditor

Every method: load a `Snapshot` (cached per `(scope, data_version)`), run a pure calculator, verify its invariants, return `AuditedResult { value, inputs_hash, data_version, checks }`. A failed invariant is HTTP 500 and the value is discarded.

| Calculation | Endpoint |
|---|---|
| Peer settlement (live, sealed with drift, `?explain=1` trail) | `GET /settlements/{month}` |
| Deposit proposal / joint dashboard | `GET /accounts/{id}/deposit-proposal`, `/dashboard` |
| Budget status / proposals | `GET /budgets/status`, `/budgets/proposals` |
| Salary ratios, split overview, split proposal | `GET /splits/salary-ratios`, `/splits/effective`, `/splits/proposal` |
| Effective income, cash flow | `GET /income/effective`, `/cashflow` |
| Project equity | `GET /projects/{id}/settlement` |
| Resolved split of one expense | `GET /entries/{id}/shares` |

- **Allocation:** signed Hare-Niemeyer (largest remainder) with `.floor()`; remainder ties broken by the SHA-256 of `"{seed}:{member_id}"`; entries are seeded with their id.
- **Share precedence:** personal pay, entry override, income ratio, agreement in force (bounded beats open-ended, then later `valid_from`, then higher id), equal split.
- **Settlement:** per-category buckets, net = paid − expected, offset rules, connected-component isolation, greedy debt simplification; joint-account expenses are excluded.
- **Sealed months:** `POST /settlements/{month}/seal` stores the result; later entry writes in that month fail with 409.

## 5. HTTP and real time

Routes are listed in plan §7. Errors are `{"detail": "..."}` with 401, 404, 409, 422 or 500. Every route except `/health` and `/auth/*` needs `Authorization: Bearer <token>`. Plain aggregates for charts are under `/views/*` (ids only, no ciphertext). `GET /ws/finance?token=` streams `{"event": "<table>.<action>"}` after each successful write.

A tokio scheduler runs at start-up and daily at 00:00:05 UTC: it materialises recurring templates (7-day grace for missed days; idempotent through the unique `(recurring_id, entry_date)` index) and sweeps sessions idle for more than 7 days.

## 6. Build, run, verify

```bash
make lint-backend      # clippy -D warnings over the workspace (also type-checks)
make deploy-backend    # build the image; swap the running container only if the build succeeds
```

`backend/.cargo/config.toml` forces a linker symbol needed by `libduckdb-sys`; without it the release binary does not link, which clippy cannot detect. The Rust test suite (`make test-backend`, cargo-nextest) is planned but not yet written.
