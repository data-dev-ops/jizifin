---
name: jizifin-backend-developer
description: Builds and changes the jizifin Rust/DuckDB backend (crates jizifin-domain, jizifin-auditor, jizifin-store, jizifin-api, jizifin-wasm) the way this repo requires — small clippy-gated methods, integer cents, DuckDB-only SQL, zero-knowledge ciphertext columns, Docker-only builds and a real-run check before calling anything done. Use it for any backend work in /opt/jizifin: a new endpoint, repository, view, Auditor calculation, schema migration, bug fix, refactor, deploy or a "why does the API return X" investigation, even when the user only names a screen or a number and not the crate.
---

# Jizifin backend developer

You are working on a household finance tracker whose backend was migrated from FastAPI/SQLite to object-oriented Rust on DuckDB 2.0. `AGENTS.md` is the contract and `docs/RUST_MIGRATION_PLAN.md` is the deep reference (API shapes §7, Auditor §4, schema §5, repositories §6, spike results §11.2, parity hazards §15). This skill does not repeat those tables; it tells you how to work so a change lands cleanly the first time. Read the relevant plan section before you touch an area you have not seen in this session.

The owner's standing wishes, from the migration sessions, are the reason behind most rules below:
- **Redundancy and legacy are a "capital sin".** No shims, no v1 paths, no dead code, no second copy of a helper. If two places need it, extract one.
- **Small methods inside their classes.** OOP-style Rust: a struct with private fields and an `impl`; stateless repositories are unit structs.
- **The Auditor lives inside the backend** and owns every contentious number. The browser does almost nothing.
- **DuckDB is the only database.** No fallback, no ORM.
- **Brief output.** Report results, not narration.

## 1. Decide where the change belongs

Before writing code, place it. A wrong placement is the most expensive mistake here because it breaks the "one owner per number" design.

| The change is… | It goes in | Exposed as |
| :--- | :--- | :--- |
| A number someone could dispute: settlement, deposit, budget status/proposal, income, ratio, split, cash flow, project equity, anything with allocation or rounding | `jizifin-auditor` calculator + `InvariantSet` | `AuditedResult` route in `routes/audit.rs` or `settlements.rs` |
| A plain aggregate: sum, count, average, share of a total, grouped by an id | `jizifin-store/src/repo/views.rs` | `/views/*` in `routes/views.rs` |
| Create/read/update/delete of a record | one repository struct per aggregate in `jizifin-store/src/repo/` + an input with `validate()` in `inputs.rs` | CRUD route; create `201 {"id"}`, update/delete `204` |
| A new type, id, enum or entity | `jizifin-domain` (pure, no IO, must stay wasm32-compatible) | — |
| Anything the browser would otherwise compute | server side, per the rows above | never in Svelte |

If a screen needs a number the server does not return, add it server side. Never "just sum it in the component".

## 2. Hard rules (each one has bitten this project)

1. **Clippy is the gate**: ≤ 20 lines per fn, ≤ 4 parameters including `self`, cognitive complexity ≤ 8, `-D warnings`. Techniques that keep you under it:
   - Split into private helpers named for what they compute; one helper per concept.
   - Beyond 4 parameters, a parameter struct (see `MonthSlice` in `repo/views.rs`, `Focus`).
   - `tracing::…!` macros inflate cognitive complexity; wrap them in tiny helpers (`log_failure` in `jizifin-api/src/error.rs`, `scheduler.rs`).
   - The codebase prefers `match cond { true => …, false => … }` and iterator chains over nested `if`/`for`.
   - A new crate adds `[lints] workspace = true`.
2. **Every `pub` item has a `///` line.** Errors are `{"detail": "..."}` via `ApiError`: 422 invalid, 401 no session, 404 missing, 409 conflict / sealed month / still referenced, 500 internal or failed invariant.
3. **Integer cents.** `Cents(i64)` in Rust, `BIGINT` in SQL. `SUM` returns `HUGEINT` in DuckDB, so write `CAST(SUM(amount_cents) AS BIGINT)`. Python parity: `round()` is `f64::round_ties_even`, `math.floor` is `.floor()` before the cast (never `as i64` on a negative float), integer division is `div_euclid`/`rem_euclid`. Never truncate a percentage float.
4. **Ciphertext columns (`*_ct`)**: only equality, `IN`, joins and `GROUP BY`. No range filter, `LIKE` or `ORDER BY` on them; the browser sorts names. `Ciphertext` deliberately has no `Ord`; do not add one.
5. **SARGable dates**: compare `entry_date` to bounds (`BETWEEN ? AND ?` from `Month::period()`); never wrap the column in a function in `WHERE`. `date_trunc` belongs in `SELECT`/`GROUP BY` only. Month ends come from `Month::last_day()`, never `'{month}-31'`.
6. **No foreign keys.** DuckDB blocks updating a UNIQUE column of a referenced parent, and `members.name_ct`/`categories.name_ct` are UNIQUE and renamable. Integrity lives in repositories: `require(conn, table, id)` before linking, `unreferenced(conn, REFERENCES, id, what)` before deleting (both in `jizifin-store/src/sql.rs`).
7. **Single writer.** Overlapping updates of one row abort ("Conflict on tuple deletion"). All writes go through `Database::write` (bumps `DataVersion`, invalidates Auditor snapshots) or `write_meta` (sessions only, no bump); both take `Database::exclusive()`. In the API use `AppState::read / write(topic, …) / write_meta`; they run in `spawn_blocking` and `write` publishes `{"event":"<table>.<action>"}` on `/ws/finance`. Never write through a raw pooled connection.
8. **Sealed months**: any entry create/update/delete must call `Seals::guard` with every affected date (old and new date on update). The result is 409.
9. **Bound lists need a cast**: `list_contains(CAST(? AS BIGINT[]), id)`. An empty uncast list fails to bind ("Cannot deduce template type") and broke the empty-month snapshot once.
10. **Schema changes**: new numbered `migrations/000N_*.sql`, bump `LATEST_VERSION` in `migrate.rs`, keep `schema::TABLES` (parent-first; used by backup restore and auth reset) in step, then update `AGENTS.md` §3, `README.md` and the plan §5 together.
11. **Secrets**: never write the household passphrase anywhere (file, log, env file, doc, commit message, skill). If it appears in a transcript, do not copy it. Read it from the environment at run time only.
12. **No tests unless asked.** The Rust test suite is Phase B (plan §12). Do not create tests or claim a red/green suite as evidence. Lint and real runs are the evidence.

## 3. Recipes

### Add an Auditor calculation
1. Calculator struct in its own file under `jizifin-auditor/src/`, constructed from `&Snapshot` (+ `&AuditScope` / ids), with a `calculate()`/`propose()` that is pure. Apply member and account filters in the calculator; the loader does not filter them.
2. If it needs data the snapshot lacks, extend `Snapshot` and `DuckSnapshotLoader::load` (`jizifin-store/src/loader.rs`) — keep the month load well under 20 ms (measured p95 8.9 ms on 100k entries).
3. An `InvariantSet` of small `Invariant` structs with stable snake_case ids that prove conservation (Σ parts = total, nets sum to zero, rounding only raises, …). A calculator without invariants needs a reason; ask the `jizifin-auditor` skill which identities must hold.
4. A public `Auditor` method that calls the private `audit(scope, invariants, |snap| …)`; never return a value outside that pipeline.
5. A route that builds the scope (`AuditScope::month / trailing / account_to_date / project`, `.with_members`, `.with_account`) and calls `run(&s, move |a| …)`.
6. Anything that changes the result must be in `AuditScope` (it is the cache key). Presentation flags such as `?explain=1` stay out of it.
7. Allocation goes through `LargestRemainderAllocator` with a `TieSeed` (`entry(id)` or `named("…")`); do not write a second rounding routine.

### Add a view (plain aggregate)
SQL in `Views` returning ids and cents only (no ciphertext), parameter struct when filters grow, route in `routes/views.rs`, shape documented in plan §7.4.

### Add a CRUD resource
Domain entity → input struct with `validate()` (422 rules) in `inputs.rs` → repository unit struct (`list/get/create/update/delete`, `require` on every foreign id, `unreferenced` on delete, `touch_one` for 404 on update/delete) → row mapper in `rows.rs` → route with `s.write("<table>.created", …)` → add the table to `schema::TABLES` if new.

## 4. Build, run, verify (Docker only)

Host `cargo`/`npm` may be missing; use the Makefile. Builds are slow (cold image ≈ 20 min, warm ≈ 10 min, clippy a few minutes): run them with `run_in_background` and keep working.

1. `touch backend/crates/*/src/lib.rs backend/crates/*/src/main.rs` then `make lint-backend`. Clean ends in `Finished`. Clippy only type-checks: it cannot catch the link problem `backend/.cargo/config.toml` fixes (`-Wl,-u,duckdb_register_static_extensions`) — never remove that file, and copy `.cargo` into every Dockerfile stage.
2. If the frontend changed: `make wasm`, then the production build from `AGENTS.md` §1, then `git checkout frontend/package-lock.json`.
3. **Real run before you say it works.** Follow [references/scratch-run.md](references/scratch-run.md): a scratch instance on `127.0.0.1:18000` initialised with a throwaway proof (optionally with the user's backup restored into it), curl the endpoint, compare with what you expected. Never experiment on `./data/finance.duckdb`, and never go looking for the household's passphrase or login proof.
4. Deploy only when asked: `make deploy-backend` (stop-then-start; DuckDB holds an exclusive per-process file lock so there is no blue/green; Caddy's `lb_try_duration` covers the gap). Then check `curl -s http://localhost/api/health` and one authenticated route.
5. Docs: when a schema, endpoint or workflow changes, update `AGENTS.md`, `README.md` and the plan in the same change.
6. Commit only when the user asks, on the current branch (`dev`), with the session's attribution line.

## 5. Known DuckDB 2.0 facts (from the spike gate, 2026-10-26)

- The crate `duckdb = "~1.20000.0"` bundles DuckDB 2.0.0-dev (`d54cbbe3df`). Its extensions are not published, so anything that autoloads one fails at run time with HTTP 500: `INSTALL ducklake` 404s (Phase D waits for a release build), and `current_date`, `now()` and time-zone functions need `icu`. Pass dates in as bound parameters from Rust (`today()` in `routes/query.rs`).
- The SQL console (`POST /query`) accepts one plain `SELECT` (CTEs and `information_schema` fine) verified with `json_serialize_sql`; it rejects writes, multiple statements and table functions (`read_csv`, `duckdb_tables()`), caps at 1000 rows.
- `INSERT OR REPLACE` and `ON CONFLICT DO NOTHING` both work; recurring materialization relies on the unique `(recurring_id, entry_date)` index for idempotency.
- Backup export/restore round-trips row counts and sequence positions.

## 6. Before you finish

- [ ] Placement matches §1; no money arithmetic moved to the browser.
- [ ] No duplicate helper, shim or dead code left behind.
- [ ] `make lint-backend` clean (and frontend build if touched).
- [ ] Real-run evidence: the request you made and what came back.
- [ ] Docs synced if schema/endpoints/workflow changed.
- [ ] If the change touches money maths, run the `jizifin-auditor` review checklist on it.
