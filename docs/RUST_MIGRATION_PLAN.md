# Jizifin Migration Plan: FastAPI/SQLite → Rust/DuckDB 2.0, Thin Client, In-Backend Auditor

**Branch:** `rust-migration` (not pushed).
**Status (2026-10-26):** Phase A is mostly implemented. Commits: `d30b5cd` (workspace + Auditor), `964bf5a` (CRUD, auth, WebSocket, v2 routes), `7d8b3e2` (backup, migrator, deploy wiring). The WASM CSV parser, the `/views/*` endpoints, the Auditor additions and the thin-frontend rewrite are in the working tree but **not committed yet**. The DuckDB spike gate (§11.2) is done and its findings are applied. The reference capture (§11.3) is done. Remaining Phase A work: the freeze. The Dockerfile fix (§11.1), `?explain=1`, `GET /entries/{id}/shares`, `Dockerfile.test.rust` and the frontend arithmetic check (§11.4) are done. **Nothing has been tested yet**; tests are Phase B.

This document is written so that an implementer who did not take part in the earlier work can finish the migration:
- §0 explains how to work on this branch.
- §1 lists what was asked for.
- §2 summarises what exists.
- §3–§10 describe how it works.
- §11 lists exactly what is left, step by step.

---

## 0. How to work on this branch (read first)

### 0.1 Rules that apply to every change

1. **DuckDB only.**
   - Use no other database and no fallback engine.
   - SQLite is never used at runtime. The only SQLite use is the migrator reading the legacy file once, read-only.
2. **No legacy, no redundancy.**
   - No v1 API, no compatibility shims, no "old way" code paths and no duplicated helpers.
   - When two places need the same logic, extract one helper.
   - Delete dead code instead of leaving it in place.
3. **Small methods.** Clippy enforces these limits (§3.2):
   - At most 20 lines per function.
   - At most 4 parameters, including `self`.
   - Cognitive complexity of at most 8.

   When a function grows, split it into private helpers. When the parameter list grows, introduce a parameter struct; `MonthSlice` in `jizifin-store/src/repo/views.rs` is an example.
4. **Contentious maths lives in the Auditor.**
   - Only `jizifin-auditor` computes settlements, deposit proposals, budget status and proposals, income, ratios, split resolution, cash flow and project equity.
   - Plain aggregates (sums, counts, shares of a total) live in store repositories, exposed as `/views/*`.
   - **The browser does no money arithmetic**: no sums, no percentages, no rounding of amounts.
   - The browser only encrypts input, decrypts labels, formats numbers for display, and does plaintext substring search and sorting on the current page.
5. **No tests until the freeze.**
   - Do not write or run tests in Phase A.
   - `cargo check`, `cargo clippy` and `vite build` are allowed and expected.
   - On this branch, this overrides the per-change test rule in `AGENTS.md` §7.
6. **Docker first.** Never rely on host `cargo` or `npm`; use the commands in §0.2.
7. **Secrets.**
   - Never write the household passphrase to a file, a log, an env file or this document.
   - The migrator reads it from `JIZIFIN_PASSPHRASE` at run time only.
8. **Commits** happen only when the user asks. End commit messages with the attribution line the session requires.

### 0.2 Commands

Run these from the repo root (`/opt/jizifin`) unless stated otherwise.

**Lint the whole Rust workspace.** This also type-checks. Touch the crate roots first: the Docker volume sometimes keeps stale fingerprints, and touching forces a recheck.

```bash
touch backend/crates/*/src/lib.rs backend/crates/*/src/main.rs 2>/dev/null
docker run --rm -v "$PWD/backend:/src" -w /src \
  -v jizifin-cargo-registry:/usr/local/cargo/registry -v jizifin-cargo-target:/target \
  -e CARGO_TARGET_DIR=/target rust:1-bookworm \
  sh -c 'rustup component add clippy >/dev/null 2>&1; cargo clippy --workspace --all-targets -- -D warnings'
```

**Build the WASM CSV package** into `frontend/src/lib/wasm/` (git-ignored). This is needed before any frontend build or dev run.

```bash
make wasm
```

**Build the frontend for production.** This catches Svelte compile errors and missing imports.

```bash
cd frontend && docker run --rm -v "$PWD:/app" -v jizifin-node-modules:/app/node_modules -w /app node:22-alpine \
  sh -c 'npm install --include=optional --no-audit --no-fund >/dev/null 2>&1; npx vite build'
```

`npm install` may rewrite `package-lock.json`. Revert that change afterwards:

```bash
git checkout frontend/package-lock.json
```

**Deploy and migrate:**
- `make deploy-backend` builds the backend image and swaps the container only if the build succeeded.
- `make migrate` runs the one-shot legacy SQLite → DuckDB migration. It needs `JIZIFIN_PASSPHRASE` in the environment.

A clean clippy run ends with `Finished`. A clean `vite build` ends with `✓ built`; the chunk-size warning is expected.

---

## 1. What was requested

These are the user's requirements in the order given. Each has an ID that is used later in this document.

| ID | Requirement |
| :--- | :--- |
| R1 | Migrate the backend to **object-oriented Rust**. |
| R2 | Replace the database with one that is highly compatible with Rust, runs natively on Apple Silicon and supports file storage for a future **data lakehouse**. The choice is **DuckDB 2.0.0** (Rust crate `duckdb = "~1.20000.0"`, see <https://duckdb.org/docs/2.0/clients/rust/overview>). |
| R3 | The **frontend does the absolute minimum of data processing**, because JavaScript is slower than Rust. |
| R4 | Everything stays on the **same public port** (Caddy `:80/:443`). The web server picks up where it left off once a build succeeds; a failed build changes nothing. |
| R5 | Backend methods are **small and live inside their classes** (structs with impl blocks). |
| R6 | An **Auditor** owns the contentious calculations: settlements, proposed deposit amounts, proposed budgets and similar. It is **not a separate process**. It lives inside the backend; when the frontend asks, it pulls data from DuckDB, calculates in memory and returns the value. |
| R7 | **No Postgres fallback.** DuckDB is the only option. The current tables may be redesigned freely. |
| R8 | **Tests are written and run only once the framework is finalized** (the freeze). |
| R9 | **Reseal all data** with the new deterministic tie-break seed, which is based on the member id. |
| R10 | **No legacy and no redundancy** (called a "capital sin"). |
| R11 | Follow `AGENTS.md` for the current setup: Docker-first execution, the zero-knowledge crypto contract, cents stored as integers, and the deterministic AES-GCM rules. |
| R12 | Brief, efficient communication. |

---

## 2. Status overview

| Area | State | Where |
| :--- | :--- | :--- |
| Workspace and lint gates | ✅ done | `backend/Cargo.toml`, `backend/clippy.toml` |
| Domain types and entities | ✅ done | `crates/jizifin-domain` |
| Auditor (all calculators, invariants, cache) | ✅ done, untested | `crates/jizifin-auditor` |
| Store: schema, migrations, repositories, loader, backup | ✅ done, untested | `crates/jizifin-store` |
| API: routes, auth, sessions, WebSocket, scheduler, console | ✅ done, untested | `crates/jizifin-api` |
| Legacy migrator | ✅ done, never run | `crates/jizifin-migrate` |
| WASM CSV parser | ✅ done (uncommitted) | `crates/jizifin-wasm` |
| Thin-frontend rewrite | ✅ done (uncommitted), builds clean | `frontend/src` |
| Compose, Caddy, Makefile, Dockerfiles | ✅ done (`Cargo.lock` fix applied) | repo root, `backend/Dockerfile.rust`, `frontend/Dockerfile.prod` |
| Spike gate (DuckDB behaviour checks) | ✅ done, results applied | §11.2 |
| Reference capture for Phase B | ✅ done | §11.3 |
| Open endpoints and decisions | ✅ done (§11.4 items 1–4); item 5 waits for cutover | §11.4 |
| Framework freeze | ❌ to do | §11.5 |
| Phase B tests | ❌ to do | §12 |
| Phase C cutover | ❌ to do | §13 |
| Phase D lakehouse | ❌ future | §14 |

---

## 3. Architecture

```
              :80 / :443  (unchanged, Caddy never restarts)
                     │
             ┌───────┴────────┐
             │     caddy      │  lb_try_duration 30s retry during backend swap
             └───┬────────┬───┘
 /api/* (prefix  │        │ everything else
 stripped), /ws/*▼        ▼
  ┌───────────────────────────┐  ┌──────────────────────────┐
  │ backend:8000  (Rust)      │  │ frontend:5173            │
  │  axum router              │  │ Svelte: encrypt, decrypt │
  │   ├─ repositories (CRUD,  │  │ labels, render, WASM CSV │
  │   │   /views aggregates)  │  └──────────────────────────┘
  │   └─ Auditor (in-process) │
  │        │ snapshot loads   │
  │  ┌─────▼───────────────┐  │
  │  │ DuckDB 2.0 embedded │──┼─▶ /data/finance.duckdb  (+ /data/lake/ in Phase D)
  │  └─────────────────────┘  │
  └───────────────────────────┘
```

There is one backend process in one container. DuckDB takes an exclusive file lock per process, so deploys stop the old container and then start the new one; Caddy's retry covers the gap (§10).

### 3.1 Rust stack

| Concern | Crate or choice |
| :--- | :--- |
| Runtime, HTTP, WebSocket | `tokio`, `axum` 0.8 (`ws`), `tower-http` 0.7 (`TraceLayer`) |
| Database | `duckdb = { version = "~1.20000.0", features = ["bundled", "parquet", "json", "r2d2", "chrono", "serde_json"] }`. This is DuckDB 2.0.0: since 1.5.0 the crate is versioned `1.<major><minor><patch>.x`. |
| Pool | `r2d2` with `DuckdbConnectionManager`. Every DB call runs in `spawn_blocking` (`AppState::read/write/write_meta`). |
| Validation | serde into domain value types (`Cents`, `Month`, `Ciphertext`, typed ids) plus `validate()` on input structs. There is no `garde`/`validator` crate. |
| Scheduler | A plain `tokio` loop (`jizifin-api/src/scheduler.rs`); no cron crate |
| Hashing and tokens | `sha2`, `hex`, `getrandom` |
| Migrator only | `rusqlite` (bundled, read-only), `aes-gcm`, `pbkdf2`, `base64` |
| WASM only | `wasm-bindgen`, `serde-wasm-bindgen`, `regex-lite` |
| Errors and logs | `thiserror`, `tracing`, `tracing-subscriber` |

### 3.2 Class design and lint gates

A "class" is a `struct` with private fields and an `impl` block.
- Stateless repositories are unit structs (`pub struct Entries;`) with associated functions that take `&Connection`.
- Dependency injection is an `Arc<dyn SnapshotLoader>` inside the `Auditor`, which sits in `AppState`.

Lints are configured once for every crate:
- `backend/Cargo.toml` sets `too_many_lines`, `cognitive_complexity` and `too_many_arguments` to `deny` under `[workspace.lints.clippy]`.
- `backend/clippy.toml` sets the thresholds: `too-many-lines-threshold = 20`, `too-many-arguments-threshold = 4`, `cognitive-complexity-threshold = 8`.
- Every crate's `Cargo.toml` has `[lints] workspace = true`. A new crate **must** add this.
- Clippy runs with `-D warnings`, so every warning fails the run.
- Every `pub` item has a `///` doc line. This is a convention, not a lint.

Tracing macros inflate cognitive complexity. Put `tracing::…!` calls in tiny helper functions; see `log_failure` in `error.rs` and `scheduler.rs`.

### 3.3 Workspace map

```
backend/
├── Cargo.toml          workspace, shared deps, lint levels
├── Cargo.lock
├── clippy.toml         thresholds
├── Dockerfile.rust     cargo-chef build → distroless (replaces Dockerfile at cutover)
└── crates/
    ├── jizifin-domain/    pure types, no IO, wasm32-compatible
    │   ├── money.rs        Cents(i64): new/value/abs/is_*; Add, Sub, Neg, Sum, Ord; ZERO
    │   ├── calendar.rs     Month (first-day NaiveDate; serde "YYYY-MM"; parse, of, first_day, last_day,
    │   │                   next, previous, period, clamp_day), Period { from, to } (contains, overlaps)
    │   ├── cipher.rs       Ciphertext: validated Base64URL, Eq/Hash, deliberately no Ord
    │   ├── ids.rs          MemberId, CategoryId, AgreementId, AccountId, EntryId, RecurringId, JobId,
    │   │                   ProjectId, TagId, OffsetRuleId, BudgetId (serde transparent i64)
    │   ├── error.rs        DomainError
    │   └── entities/       Member, Category (+CategoryKind, PayMode, Share, Agreement), Entry (+EntryKind),
    │                       Account (+DepositMode, AccountMember, AccountCategory), Job (+Frequency),
    │                       IncomeOverride, Budget (+BudgetSource), Project, Tag, RecurringTemplate
    │                       (+Recurrence, occurrences(month)), OffsetRule, MonthSeal
    ├── jizifin-auditor/   §4
    ├── jizifin-store/     §5 (schema), §6 (repositories)
    │   ├── migrations/0001_init.sql
    │   ├── database.rs     Database { pool, version, writer }: open, in_memory, read, write (bumps DataVersion),
    │   │                   write_meta (no bump; sessions only), exclusive() (single-writer lock, S2)
    │   ├── migrate.rs      Migrator: numbered SQL files, schema_version, LATEST_VERSION = 1
    │   ├── schema.rs       TABLES (parent-first order) + clear(); used by backup restore and auth reset
    │   ├── backup.rs       Backup::export / Backup::restore (§9)
    │   ├── loader.rs       DuckSnapshotLoader (implements the Auditor's SnapshotLoader)
    │   ├── inputs.rs       all write inputs with validate()
    │   ├── rows.rs         column lists + row mappers
    │   ├── sql.rs          query, insert_id, touch_one, exists, require, unreferenced, opt_ct, DynParams
    │   ├── error.rs        StoreError: Duck, Pool, NotFound, Conflict, Invalid ("Constraint Error" → Conflict)
    │   └── repo/           one class per aggregate (§6)
    ├── jizifin-api/       §7
    ├── jizifin-migrate/   §9
    └── jizifin-wasm/      §8.4
```

---

## 4. The Auditor (`crates/jizifin-auditor`)

### 4.1 Role and request flow

`main.rs` creates one `Auditor` and stores it in `AppState` as `Arc<Auditor>`. Every public method runs the same private pipeline, `Auditor::audit`:

1. **Snapshot.** `snapshot(scope)` returns a cached `Arc<Snapshot>` for `(scope, data_version)`, or loads one through `SnapshotLoader::load(scope)`.
2. **Calculate.** A pure calculator runs on the snapshot.
3. **Check.** The calculator's `InvariantSet` runs; the first failing rule aborts with `AuditError::InvariantFailed(id)`.
4. **Return** `AuditedResult<T> { value, inputs_hash, data_version, checks }`, where `checks` lists the ids of the invariants that held.

An invariant failure becomes **HTTP 500 `{"detail": "audit invariant <id> failed"}`**. The Auditor never returns an unverified number.

**Caching:**
- `DataVersion` is an `AtomicU64` in `cache.rs`.
- `Database::write` bumps it on every committed ledger write, so a cached snapshot is never stale.
- Session writes use `write_meta`, which does not bump it.
- `SnapshotCache` is an LRU with 64 entries (`CACHE_CAPACITY` in `auditor.rs`).
- `Snapshot::inputs_hash()` is the hex SHA-256 of the snapshot's JSON.

### 4.2 Scope and snapshot

`AuditScope` (`scope.rs`) holds `{ month, period, members: Option<BTreeSet<MemberId>>, account, project, as_of }`.

| Constructor or builder | Scope it produces |
| :--- | :--- |
| `month(m, as_of)` | One month |
| `trailing(m, n, as_of)` | The `n` months before `m` |
| `account_to_date(m, account, as_of)` | From `LEDGER_START` (0001-01-01) up to `m` |
| `project(id, as_of)` | Up to `PROJECT_HORIZON` (9999-12-31) |
| `with_members(set)` | Member filter; an empty set means no filter |
| `with_account(Option)` | Account filter |

`DuckSnapshotLoader` (`jizifin-store/src/loader.rs`) loads:
- all members, categories, agreements, jobs, accounts, active offset rules, budgets and projects;
- the income overrides inside the period;
- the entries inside the period; a project scope loads `project_id = ? OR kind = 'deposit'`;
- the share overrides of those entries.

The calculators apply the member and account filters; the loader does not.

### 4.3 Calculators, endpoints and invariants

| Auditor method | Calculator (file) | Endpoint | Invariant ids |
| :--- | :--- | :--- | :--- |
| `settlement(scope)` | `SettlementCalculator` (`settlement/`) | `GET /settlements/{month}` (live) | `bucket_zero_sum`, `net_zero_sum`, `debts_settle_net`, `debts_stay_in_component`, `joint_excluded` |
| `sealed_settlement(scope, seal)` | Recomputes and compares the JSON | `GET /settlements/{month}` (sealed) | none; reports `drift` |
| `deposit_proposal(scope, account, round_to)` | `DepositProposer` (`deposit.rs`) | `GET /accounts/{id}/deposit-proposal` | `deposit_sums_to_target`, `deposit_non_negative`, `rounding_only_raises` |
| `joint_dashboard(scope, account)` | `JointBalanceCalculator` (`joint.rs`) | `GET /accounts/{id}/dashboard` | `categories_within_actual`, `target_covers_expected`, `top_up_closes_gap` |
| `budget_status(scope)` | `BudgetEvaluator` (`budget.rs`) | `GET /budgets/status` | `remaining_balances`, `summary_matches_rows` |
| `budget_proposals(scope)` | `BudgetProposer` (`budget.rs`) | `GET /budgets/proposals` | `whole_unit_proposals` |
| `salary_ratios(scope)` | `SalaryRatioCalculator` (`ratios.rs`) | `GET /splits/salary-ratios` | `ratios_sum_to_100` |
| `split_overview(scope)` | `SplitOverviewCalculator` (`splits.rs`) | `GET /splits/effective` | `shared_splits_total_100` |
| `split_proposal(scope, strategy)` | `SplitProposer` (`splits.rs`) | `GET /splits/proposal` | `proposal_totals_100` |
| `income(scope)` | `IncomeResolver` (`income.rs`) | `GET /income/effective` | none |
| `cash_flow(scope)` | `CashFlowCalculator` (`cashflow.rs`) | `GET /cashflow` | `net_balances` |
| `project_settlement(scope, id)` | `ProjectSettlementCalculator` (`project.rs`) | `GET /projects/{id}/settlement` | `project_funding_liability_balance`, `project_debts_settle` |

`POST /settlements/{month}/seal` runs `settlement` and stores the JSON in `month_seals`.

### 4.4 Core rules

Do not change these without updating the Phase B parity tests.

**Allocation** (`allocation.rs`, `LargestRemainderAllocator`) is a signed Hare-Niemeyer:
- Work on the absolute total and floor each share.
- Hand out the leftover units in descending remainder order.
- Break ties by the SHA-256 of `"{seed}:{member_id}"`. This is the R9 reseal seed.
- Entries use `TieSeed::entry(id)`. Named calculations use `TieSeed::named(...)` with `"deposit:{account}:{month}"`, `"ratio:{month}"` or `"even:{month}"`.

**Share precedence** (`shares.rs`, `ShareResolver`). The first rule that applies wins:
1. Personal pay: the payer bears 100 %, in a bucket keyed by the payer.
2. The entry's share override.
3. Income ratio, when the category has `pay_mode = income_ratio` and someone earns.
4. The highest-precedence agreement that covers the entry date. A bounded agreement beats an open-ended one; after that the later `valid_from` wins, then the higher id.
5. An equal split over the active members.

**Settlement** (`settlement/`):
1. Book every non-joint expense into buckets (`ledger.rs`). Each row has `total`, `paid`, `expected`, `share_pct`, `paid_pct` and `net`.
2. Net per member = paid − expected.
3. Apply offset rules (`offsets.rs`). The smaller of two amounts moves from the debit member to the credit member: the credit member's spend in the credit category, and the debit member's spend in the debit category.
4. Partition members into connected components (`components.rs`, union-find).
5. Simplify debts within each component (`debts.rs`).
- `transfer_total` is the sum of all debts.
- Joint-account expenses (`entry.account` set) never enter the settlement.

**Income** (`income.rs`):
- `contract` is the sum of active jobs, each normalized to monthly with `round_ties_even(amount × periods_per_year / 12)`.
- An income override for the month replaces the contract as `base`.
- `one_off` is the sum of income entries in the month.
- `total = base + one_off`.

**Deposits** (`deposit.rs`):
- `target = round_ties_even(expected_total × (1 + margin/100))`.
- The target is divided equally, by income weights (equally when nobody earns), or as fixed amounts per member.
- With `round_to`, the result also reports `rounded`: each amount rounded up to a multiple of the step.

**Joint dashboard** (`joint.rs`):
- `balance` = all-time deposits + corrections − expenses on the account, up to month end.
- It also reports:
  - month spend and deposits, `expected_total`, `safety_margin_pct` and `target`;
  - `top_up = max(0, target − balance)` and `month_surplus = deposited − spent`;
  - usage per category and deposits per member.
- `status` is `target_met`, `pending {due_day}` or `below_target`.

**Budgets** (`budget.rs`):
- Each category gets its effective budget for the month by precedence.
- Spend is filtered by the scope's members and account.
- The summary reports totals, usage % and a health index: the share of configured categories that are under 90 % usage.
- Proposals are the mean of the 3 previous months, rounded up to a whole currency unit.

**Split overview** (`splits.rs`). For each expense category it reports:
- the source: `personal`, `income_ratio`, `agreement` or `equal`;
- the agreement in force for the month. Overlapping agreements compete; a bounded one beats an open-ended one;
- the percentage per member.

Proposals use `even` (largest remainder over 100 points) or `salary` (equal to the salary ratios).

**Cash flow** (`cashflow.rs`):
- `income` is the sum of the scoped members' effective income.
- `spent` is the sum of expenses paid by the scoped members.
- It also reports `net` and the savings rate.

### 4.5 Sealed months

- `month_seals` stores `result_json` (the settlement), `inputs_hash` and `sealed_at`.
- For a sealed month, `GET /settlements/{month}` returns `{"state":"sealed","seal":{…},"drift":bool}`. `drift` is true when today's recomputation differs from the stored JSON.
- For any other month it returns `{"state":"live", …AuditedResult}`.
- `Seals::guard` makes every entry create, update or delete in a sealed month fail with **409**.

---

## 5. Schema (`crates/jizifin-store/migrations/0001_init.sql`)

**Principles:**
- Surrogate `BIGINT` keys from sequences.
- No foreign keys, by decision (spike S1): DuckDB blocks updating a UNIQUE column of a referenced parent row, which `members` and `categories` need. Repositories enforce integrity through `require`/`unreferenced`.
- One unified `entries` ledger.
- SCD2 agreements and budgets.
- No stored contentious values, except month seals.

Columns ending in `*_ct` hold deterministic AES-GCM ciphertext in Base64URL.

| Table | Key columns |
| :--- | :--- |
| `app_config` | `key` PK, `value` (holds `auth_proof_sha256`) |
| `sessions` | `token_sha256` PK, `last_active` (7-day idle TTL) |
| `members` | `id`, `name_ct` UNIQUE, `color`, `is_active` |
| `categories` | `id`, `kind` ('expense','income'), `name_ct`, `pay_mode` ('shared','personal','income_ratio'), UNIQUE(kind, name_ct) |
| `agreements` | `id`, `category_id`, `valid_from`, `valid_to` NULL (open-ended = baseline), `note_ct` |
| `agreement_shares` | (`agreement_id`, `member_id`) PK, `pct` DOUBLE |
| `accounts` | `id`, `name_ct` UNIQUE, `safety_margin_pct`, `expected_total_cents` NULL (gross override), `deposit_mode` ('equal','salary_ratio','fixed') |
| `account_members` | `account_id`, `member_id`, `fixed_deposit_cents` NULL, `due_day` |
| `account_categories` | `account_id`, `category_id`, `expected_cents` |
| `projects` | `id`, `name_ct`, `target_cents`, `target_date`, `color` |
| `project_members` | `project_id`, `member_id` |
| `tags` | `id`, `name_ct`, `description_ct`, `color`, `valid_from` NULL, `valid_to` NULL |
| `recurring_templates` | `id`, `kind`, `amount_cents`, `member_id`, `category_id`, `account_id` NULL, `day_of_month`, `recurrence`, `valid_from`, `valid_to`, `name_ct`, `is_active` |
| `entries` | `id`, `kind` ('expense','income','deposit','correction'), `entry_date`, `amount_cents` (signed), `member_id`, `category_id`, `account_id`, `project_id`, `tag_id`, `recurring_id`, `name_ct`, `note_ct`, `created_at`. CHECKs: expense and income need a member and a category; deposit and correction need an account. |
| `entry_share_overrides` | (`entry_id`, `member_id`) PK, `pct` |
| `jobs` | `id`, `member_id`, `name_ct`, `amount_cents`, `frequency` ('monthly','weekly','biweekly','annual'), `valid_from`, `valid_to`, `notes_ct` |
| `income_overrides` | (`member_id`, `month`) PK, `amount_cents`, `note_ct` |
| `budgets` | `id`, `category_id`, `valid_from` (month start), `valid_to` NULL (= standing), `limit_cents`, `source` ('manual','accepted_proposal') |
| `offset_rules` | `id`, `credit_member_id`, `credit_category_id`, `debit_member_id`, `debit_category_id`, `is_active` |
| `month_seals` | `month` PK (first day), `result_json` JSON, `inputs_hash`, `sealed_at` |
| `schema_version` | Created by the migrator code, not by the SQL file |

**Indexes:**
- `entries(entry_date, id)`, `(member_id, entry_date)`, `(category_id, entry_date)`, `(account_id, entry_date)`, `(project_id)`.
- UNIQUE `(recurring_id, entry_date)`, which makes the recurring catch-up idempotent.

The encryption rules from `AGENTS.md` carry over. On `*_ct` columns, use only equality, `IN`, joins and `GROUP BY`; never range filters, `LIKE` or collation sorting.

---

## 6. Store repositories (`crates/jizifin-store/src/repo/`)

| Class | File | Notes |
| :--- | :--- | :--- |
| `Members` | members.rs | List, create, update, delete. Delete is refused while the member is referenced; it also removes project memberships and income overrides. |
| `Categories` | categories.rs | `list`, `map`, `kind_of`, create, update, delete. Delete is refused while referenced (the `REFERENCES` list). |
| `Agreements` | agreements.rs | `all`, `for_category`, create, update, delete. `set_baseline(category, shares)` updates the latest open-ended agreement, or creates one from `LEDGER_START`. |
| `Accounts` | accounts.rs | CRUD, including members and categories |
| `Entries` | entries.rs | See below |
| `Jobs`, `IncomeOverrides` | income.rs | Jobs: CRUD, `get`, `adjust(id, JobAdjustment)`, which ends the job the day before `effective` and creates a copy with the new amount. Overrides: list, upsert, delete. |
| `Budgets` | budgets.rs | CRUD |
| `Projects` | projects.rs | CRUD, including members |
| `Tags` | tags.rs | CRUD. Update refuses a window that would orphan tagged entries; delete untags the entries. |
| `Recurring` | recurring.rs | CRUD and `materialize(today, grace_days)`. The insert uses `ON CONFLICT DO NOTHING` on the unique index. |
| `OffsetRules` | offsets.rs | `list`, `active`, create, delete |
| `Seals` | seals.rs | `list`, `get`, `insert`, `guard` |
| `Auth`, `Sessions` | auth.rs | Proof hash: init, read, reset. Sessions: create, touch, delete, sweep. |
| `Console` | console.rs | Read-only SQL, see below |
| `Views` | views.rs | Read-only aggregates (§7.4) |
| progress | progress.rs | `project_progress(today)`, used by `Views::projects` |

**`Entries` details:**
- Keyset pagination, newest first, through `EntryFilter`: `from`, `to`, `kind`, `member`, `category`, `account`, `project`, `tag`, `cursor` (`"YYYY-MM-DD:id"`) and `limit` (at most 200).
- `get`, `create`, `create_batch` (one transaction), `update`, `delete`.
- `duplicates` finds entries with the same category and amount in the same month.
- Writes check the category kind, the tag window and sealed months.

**`Console` details:**
- Accepts a single statement only when `json_serialize_sql` reports it as a plain `SELECT` with no table functions.
- Caps results at 1000 rows and returns `{columns, rows, truncated}`.

---

## 7. HTTP API (`crates/jizifin-api`)

Paths are listed as the backend sees them. The browser calls them under `/api`; Caddy and the Vite dev proxy strip the prefix. `/ws/finance` has no prefix.

**Conventions:**
- JSON in and out.
- Amounts are integer cents; `Cents` serializes as a plain number.
- Months are `"YYYY-MM"` and dates are `"YYYY-MM-DD"`. Ids are numbers.
- Fields backed by `*_ct` columns (`name`, `note`, `notes`, `description`) carry ciphertext strings.
- Errors use `{"detail": "..."}`:

  | Status | Meaning |
  | :--- | :--- |
  | 422 | Invalid input or a domain error |
  | 401 | No session, or the session expired |
  | 404 | Not found |
  | 409 | Conflict, sealed month, or still referenced |
  | 500 | Internal error or a failed audit invariant |

- Create returns `201 {"id": n}`; update and delete return `204`.
- Every route except `/health` and `/auth/*` needs `Authorization: Bearer <token>`. The WebSocket takes `?token=` instead.
- Every successful write publishes `{"event": "<table>.<action>"}` on `/ws/finance`, for example `entries.created`, `settlements.sealed` or `backup.restored`.
- Common query parameters: `?month=YYYY-MM` (defaults to the current month) and `?members=1,2` (empty means everyone).

### 7.1 Auth and system

| Method + path | Body / query | Response |
| :--- | :--- | :--- |
| GET `/health` | — | `ok` |
| GET `/auth/status` | — | `{initialized, authenticated}` |
| POST `/auth/init` | `{proof}` | `{token}`; stores only SHA-256(proof) |
| POST `/auth/login` | `{proof}` | `{token}` or 401 |
| POST `/auth/logout` | — | `{status}` |
| POST `/auth/reset` | `{proof}` | Wipes all tables |
| GET `/backup` | — | The backup file |
| POST `/backup` | Raw `.duckdb` bytes (≤ 512 MB) | Restores the file |
| POST `/query` | `{sql}` | `{columns, rows, truncated}` |
| GET `/ws/finance` | `?token=` | Event stream |

The client proof is `encryptText("FinanceTrackerAuth", key)`, using the PBKDF2/AES-GCM parameters from `AGENTS.md` §2. It is deterministic, so it is the same on every login. `GET /auth/salt` no longer exists.

### 7.2 Catalog and ledger (CRUD)

| Resource | Routes | Response shape |
| :--- | :--- | :--- |
| Members | `/members`, `/members/{id}` | `{id, name, color, is_active}` |
| Categories | `/categories`, `/categories/{id}` | `{id, kind, name, pay_mode}` |
| Agreements | GET/POST `/categories/{id}/agreements`; PUT/DELETE `/agreements/{id}`; PUT `/categories/{id}/baseline` with `{shares}` | `{id, category, valid_from, valid_to, note, shares:[{member, pct}]}` |
| Budgets | `/budgets`, `/budgets/{id}` | `{id, category, valid_from, valid_to, limit, source}` |
| Tags | `/tags`, `/tags/{id}` | `{id, name, description, color, valid_from, valid_to}` |
| Recurring | `/recurring`, `/recurring/{id}` | `{id, kind, amount, member, category, account, day_of_month, recurrence, valid_from, valid_to, name, is_active}` |
| Offset rules | GET/POST `/offset-rules`; DELETE `/offset-rules/{id}` | `{id, is_active, credit_member, credit_category, debit_member, debit_category}` |
| Jobs | `/jobs`, `/jobs/{id}`; POST `/jobs/{id}/adjust` with `{effective, amount, notes}` | `{id, member, amount, frequency, valid_from, valid_to, name, notes}` |
| Income overrides | GET `/income-overrides?month=`; PUT `/income-overrides` (upsert); DELETE `/income-overrides/{member}/{month}` | `{member, month, amount, note}` |
| Accounts | `/accounts`, `/accounts/{id}` | `{id, name, safety_margin_pct, expected_override, deposit_mode, members:[{member, fixed_deposit, due_day}], categories:[{category, expected}]}` |
| Projects | `/projects`, `/projects/{id}` | `{id, name, color, target, target_date, members:[id]}` |
| Entries | See below | See below |
| Settlements | GET `/settlements` (seals); GET `/settlements/{month}?members=&explain=1`; POST `/settlements/{month}/seal` | §4.5. `explain=1` (live months only) adds `trail: ["share(entry <id>, member <id>) = <cents> via <source>"]` to the result. |

**Entry routes:**

| Route | Purpose |
| :--- | :--- |
| GET `/entries?…filter` | Page `{entries:[{id, kind, date, amount, member, category, account, project, tag, recurring, name, note, overrides}], next_cursor}` |
| POST `/entries` | Create one entry |
| POST `/entries/batch` | Create an array of entries; returns their ids |
| POST `/entries/duplicates` | Array of `{category, amount, date, exclude?}` → `[{index, matches:[entry]}]` |
| GET/PUT/DELETE `/entries/{id}` | Read, replace, delete one entry |
| GET `/entries/{id}/shares` | Resolved split of one expense: `{entry, source, total, shares:{member:cents}}`; 404 when it is not a payable expense |

Create and update bodies are the entity without `id`; entries also omit `recurring`. PUT replaces the whole record.

### 7.3 Auditor endpoints

| Path | Query | `value` shape |
| :--- | :--- | :--- |
| `/accounts/{id}/deposit-proposal` | month, round_to (cents) | `{account, month, mode, expected_total, safety_margin_pct, target, per_member:{id:cents}, rounded}` |
| `/accounts/{id}/dashboard` | month | `{account, month, balance, actual_total, deposited_total, month_surplus, expected_total, safety_margin_pct, target, top_up, categories:[{category, actual, expected, pct_used}], members:[{member, deposited, due_day}], status:{status, due_day?}}` |
| `/budgets/status` | month, members, account | `{rows:[{category, limit, spent, remaining, pct_used}], summary:{total_limit, total_spent, total_remaining, usage_pct, configured, safe, health_pct}}` |
| `/budgets/proposals` | month | `[{category, history:[cents], proposed}]` |
| `/splits/salary-ratios` | month, members | `{month, pct:{id:int}}` |
| `/splits/effective` | month | `{month, categories:[{category, source, agreement, is_override, pct:{id:float}}]}` |
| `/splits/proposal` | strategy (`even`/`salary`), month, members | `{strategy, pct:{id:int}}` |
| `/income/effective` | month | `{id:{member, contract, base, one_off, total, has_override}}` |
| `/cashflow` | month, members | `{month, income, spent, net, savings_rate_pct}` |
| `/projects/{id}/settlement` | — | `{project, target, total_spent, participants:[{member, funding, liability, net}], debts}` |

All of these return an `AuditedResult`: `{value, inputs_hash, data_version, checks}`. Where a JSON object is keyed by member id, the keys are strings (`"3"`), as usual for JSON maps.

### 7.4 Views (plain aggregates, ids only, no ciphertext)

| Path | Query | Response |
| :--- | :--- | :--- |
| `/views/month` | month, kind (`expense` by default), members, and at most one of category / member / project / account | `{month, total, count, average, by_category:[bucket], by_member:[bucket]}`, where bucket = `{key, total, count, share_pct}` |
| `/views/series` | kind, from, to (months; by default the 12 months up to `to`) | `[{month, category, total}]` |
| `/views/tags` | — | `[{tag, total, count, first, last, monthly_avg}]` |
| `/views/tags/{id}` | — | `{summary, by_month:[bucket], by_category:[bucket]}` |
| `/views/projects` | — | `[{project, target, spent, count, progress_pct, monthly_avg, last_payment, completion:{status:"completed"\|"indefinite"\|"projected", date?, months?}}]` |
| `/views/recurring` | month | `{month, total, active, due:[{recurring, dates, total}], by_category, by_member}` |

### 7.5 Background jobs (`scheduler.rs`)

At start-up, and daily at 00:00:05 UTC, the scheduler:
- materializes recurring templates, with a 7-day grace period for missed days;
- sweeps sessions that have been idle for more than 7 days.

---

## 8. Frontend (`frontend/src`)

### 8.1 What the browser does

1. Derives the AES-GCM key from the passphrase (`crypto.js`, contract unchanged) and sends the auth proof.
2. Encrypts text fields before writes (`seal`) and decrypts labels after reads (`open`, memoized per ciphertext).
3. Renders server values, formatting cents and dates for display (`format.js`).
4. Does plaintext-only work: substring search and sorting on the loaded page, and CSV parsing in WASM.

It does **not** sum, average, round, split or compare money. If a screen needs a number the server does not provide:
- add it to the Auditor when it is contentious, or to `/views/*` when it is a plain aggregate;
- do not compute it in Svelte.

### 8.2 Client modules

| File | Purpose |
| :--- | :--- |
| `lib/api.js` | HTTP client and every resource API; see below |
| `lib/stores.js` | Catalog and session stores, lookups and UI preferences; see below |
| `lib/live.js` | Opens `/ws/finance?token=`. On each event it bumps `ledgerVersion` and refreshes the matching catalog store. Reconnects every 3 s. |
| `lib/format.js` | `money` store (`$money(cents, summary?)`), `fmtDate`, `fmtMonth`, `fmtPct`, `toCents` (string → cents, no floats), `centsInput`, `today`, `shiftMonth`, `monthBounds` |
| `lib/csv.js` | Async wrappers over the WASM exports |
| `lib/entryInput.js` | Turns an entry row into a full `EntryInput`; PUT replaces the whole entry |
| `lib/chartTheme.js` | Chart.js colours per theme, `hexToRgba`, `PALETTE` |
| `lib/SplitEditor.svelte` | Shares editor: a slider for 2 members, or one input per member. Exports `toShares`/`fromShares`. |
| `lib/EntryEditModal.svelte` | Edits any expense or income entry |

`lib/api.js` contains:
- `request()`, which throws `ApiError` with the server's `detail` and clears the session on 401;
- `seal`, `open`, `openRows` and `auth`;
- a generic `resource(path, cipheredFields, store)` used by `memberApi`, `categoryApi`, `tagApi`, `jobApi` (plus `adjust`), `recurringApi`, `budgetApi`, `accountApi`, `projectApi` and `offsetRuleApi`;
- `agreementApi` (plus `setBaseline`), `incomeOverrideApi`, and `entryApi` (page, get, create, batch, update, remove, duplicates);
- `audit` (every §7.3 call plus `seal`/`seals`), `views`, `runQuery`, `backup` and `loadCatalog()`.

`lib/stores.js` contains:
- catalog stores: members, categories, accounts, projects, tags, jobs, recurring, budgets, offsetRules, seals;
- `ledgerVersion`;
- id lookups (`memberById`, `categoryName`, …) and filters (`activeMembers`, `expenseCategories`, `incomeCategories`);
- `selectedMonth` and the dashboard scope: `MEMBER:<id>` / `ACCOUNT:<id>` tokens resolve to `scopedMembers`;
- `activeAccountId`, the session and key stores, and all UI preference stores and device profiles (unchanged).

Views refetch through reactive statements that depend on `$selectedMonth`, `$scopedMembers` and `$ledgerVersion`, for example `$: load($selectedMonth, $ledgerVersion);`.

### 8.3 Screens and their data sources

| Component | Server data |
| :--- | :--- |
| `App.svelte` | Gates on `cryptoKey && sessionToken`; runs `loadCatalog()`, then `live.connect()` |
| `Login.svelte` | `auth.status/init/login`. First boot seeds categories (LEISURE, GIFT and PERSONAL COST as `personal`) or restores a `.duckdb` backup. |
| `AnalyticsSummary.svelte` | `views.month`, `audit.cashFlow`, `audit.income`, `audit.budgetStatus`, `views.tags`, `views.projects`, `audit.jointDashboard` |
| `PaybackVisual.svelte` | `audit.settlement`, `audit.seal` |
| `DrilldownModal.svelte` | `views.month` focused on the clicked item, `entryApi.page`, `audit.income`, `views.projects` |
| `ExpenseForm.svelte` | `entryApi.duplicates`, `entryApi.create` |
| `ExpenseList.svelte` | `entryApi.page` (category and tag filters run server-side; "Load more" uses the cursor), `views.month` for totals |
| `IncomeTab.svelte` | `audit.income`, `audit.salaryRatios`, `audit.cashFlow`, `incomeOverrideApi`, `jobApi` (plus adjust), income entries |
| `SplitManager.svelte` | `audit.splits`, `audit.salaryRatios`, `audit.splitProposal`, `agreementApi`, `categoryApi` |
| `BudgetManager.svelte` | `audit.budgetStatus` (scoped), `audit.budgetProposals`, `budgetApi` |
| `ProjectsTab.svelte` | `views.projects`, `audit.projectSettlement`, `projectApi` |
| `TagsTab.svelte` | `views.tags`, `views.tag`, `entryApi.page({tag})`, `tagApi` |
| `RecurringManager.svelte` | `views.recurring`, `recurringApi` |
| `JointAccountTab.svelte` | `audit.jointDashboard`, `audit.depositProposal` (rounding), deposit and correction entries, `accountApi` |
| `BankCsvImportModal.svelte` | WASM (§8.4), `entryApi.duplicates`, `entryApi.batch` |
| `QueryConsole.svelte` | `runQuery`; decrypts the cells of columns whose names end in `_ct` |
| `SettingsTab.svelte`, `UserManager.svelte` | `backup.download`, `auth.reset`, `memberApi` |
| `RealtimeChart.svelte` | `views.series` |
| `IncomeChart.svelte` | `audit.income` |

**Deleted:**
- `csvParser.js` and `duplicateDetector.js`;
- every legacy `fetch*`/`create*` helper;
- the derived stores `analytics`, `paybacks`, `incomeAnalytics`, `joint*`, `expenses`, `users`, `splits` and `authSalt`.

**Behaviour changes** compared with the Python version. These are accepted; revisit them only if the user asks.
- CSV review rows no longer show a live "duplicate" badge. Duplicates are checked against the server when you press Import.
- Job cards no longer show a "≈ monthly" equivalent per job. The Auditor's contract total per member is shown instead.
- Tags have no active/inactive toggle; their date window decides.
- The joint "adjust deposits" settle mode is gone. The dashboard shows `top_up` and `month_surplus`, and a proposal can be saved as fixed deposits.
- A new joint account starts at a balance of 0. An opening balance is entered as a correction entry.
- The "Joint Account" default payer is stored as `JOINT`. The default payer, category and project are stored as id strings.

### 8.4 WASM (`crates/jizifin-wasm`)

**Modules:**

| Module | Purpose |
| :--- | :--- |
| `tokenizer.rs` | RFC 4180 parsing, BOM handling |
| `amount.rs` | Belgian amounts such as `-2.105,68` → cents, using integer maths |
| `date.rs` | `DD/MM/YY[YY]` → ISO dates |
| `beneficiary.rs` | ING counterparty extraction, with the same regexes as the old JS |
| `statement.rs` | Header detection (EN/NL), rows, stats |
| `import.rs` | `ImportPlan`, `ImportSummary`, `cascade_targets`, `micro_debits` |

**Exports** (camelCase in JS):

| Export | Returns |
| :--- | :--- |
| `parseStatement(text, bank)` | `{transactions:[{rowIndex, rawDescription, date, displayDate, amountCents, isCredit, beneficiary, include, groupToOther}], stats}` |
| `summarizeImport(items)` | Import totals |
| `cascadeTargets(items, index)` | Row indices to receive the same category |
| `microDebits(items, maxCents)` | Indices of small debits |
| `prepareImport(items, bundleCategoryId?)` | Drafts `{kind, date, amount, member, category, tag, label}` |

`prepareImport` turns grouped debits into one "Other expenses (n items bundled)" draft in the `OTHER` category. The modal adds `member`, `category` and `tag` to each row, and the JS sends `label` as the encrypted `name`.

**Build:**
- `frontend/Dockerfile.prod` has a `wasm` stage: a Rust image with the `wasm32-unknown-unknown` target and `wasm-bindgen-cli` pinned to the version in `Cargo.lock`.
- A `wasm-pkg` stage (from scratch) makes the output exportable.
- Compose passes `backend` as an additional build context.
- `make wasm` exports the package to `frontend/src/lib/wasm/` for the dev server.

---

## 9. Legacy migration and backup

### 9.1 `jizifin-migrate`

Usage: `jizifin-migrate --sqlite <legacy finance.db> --duckdb <new file>`.
- It refuses to overwrite an existing target.
- It requires a non-empty `JIZIFIN_PASSPHRASE`.
- The compose service `migrate` (profile `migrate`) mounts the legacy file read-only; `make migrate` runs it.

The steps (`steps/`) run in one transaction and roll back on any failure:
1. **people:**
   - members and jobs;
   - members without jobs get monthly jobs from their legacy `SALARY` rows, one job per amount change;
   - salary overrides.
2. **catalog:**
   - categories: `PERSONAL COST`, `LEISURE` and `GIFT` become `personal`;
   - active SCD2 agreements, plus baseline allocations as open-ended agreements from `0001-01-01`;
   - tags, and budgets (`'ALL'` becomes standing);
   - the Jane/John offset rule (`Combined Fixed` vs `Apartment`).
3. **accounts:**
   - the roster plus everyone with a deposit plan;
   - deposit mode mapped even/salary/manual → equal/salary_ratio/fixed;
   - the gross expected total kept as an override.
4. **ledger:**
   - expenses, non-SALARY income, paid monthly deposits and corrections;
   - one opening correction per account, bridging the stored legacy balance and the derived one;
   - recurring templates, with links to the entries they already generated.
5. **verify:**
   - the auth proof hash: SHA-256 of the legacy magic-word ciphertext, which equals the new login proof;
   - cent totals per kind match the legacy totals.
6. **seal** (after commit): reseal every month listed in legacy `settlements` with the new Auditor (R9).

The passphrase is used only to compute the ciphertext of the legacy names that were hard-coded (`cipher.rs`, `Markers`).

### 9.2 Backup

**`Backup::export`:**
1. `ATTACH` a temp file.
2. `COPY FROM DATABASE "<current>" TO jizifin_backup`.
3. Delete `sessions` from the copy.
4. Stream it. The file contains only ciphertext and numbers.

**`Backup::restore`:**
1. Attach the file read-only and check `schema_version`.
2. Clear all tables (`schema::TABLES`).
3. Copy table by table in one transaction.
4. Advance every sequence past its table's maximum id.

---

## 10. Deployment (same public port)

**`docker-compose.yml`:**
- `backend` builds `backend/Dockerfile.rust` as `jizifin-backend:latest`. It mounts `./data:/data`, sets `JIZIFIN_BIND`, `JIZIFIN_DB_PATH` and `RUST_LOG`, and uses the healthcheck `jizifin-api --healthcheck`.
- `frontend` builds `frontend/Dockerfile.prod` with `additional_contexts: backend: ./backend`.
- `migrate` is described in §9.1.

**`Caddyfile`:**
- `handle_path /api/*` → `backend:8000`, with `lb_try_duration 30s`, `lb_try_interval 250ms` and `health_uri /health`.
- `handle /ws/*` → backend, with the same retry.
- Everything else → frontend.

**`Makefile`:**
- `deploy-backend` runs `docker compose build backend && docker compose up -d --no-deps --wait backend`. A failed build leaves the running container untouched.
- `wasm`, `build-dev` (depends on `wasm`) and `migrate`.

**`.gitignore`** adds `data/`, `*.duckdb`, `*.duckdb.wal` and `frontend/src/lib/wasm/`.

---

## 11. Remaining Phase A work (do these in order)

### 11.1 `Cargo.lock` in the backend image build — done

`backend/Dockerfile.rust` now copies `Cargo.lock` in both the `planner` and `builder` stages and builds with `--locked`. Still to confirm once: `docker compose build backend` succeeds.

### 11.2 Spike gate (DuckDB 2.0 behaviour, about 1 day)

Write throwaway probes as a scratch binary under `backend/crates/jizifin-store/examples/spike.rs`. They are not tests; **delete them afterwards**. Run them in the Rust container from §0.2 with `cargo run -p jizifin-store --example spike`.

Record each result in a "Spike results" subsection at the end of this document, then apply the decision.

**S1 — Foreign keys.** Does DuckDB 2.0 still block an `UPDATE` or `DELETE` of a row that a declared foreign key references, even when the key column is unchanged?
- **Check:** create two tables with an FK and insert rows. Update a non-key column of the parent. Delete a child, then the parent.
- **Decision:**
  - If updates are blocked, keep the current design: no FKs, checks in the repositories.
  - If not, add FKs for child tables only in a new `0002_foreign_keys.sql`, and bump `LATEST_VERSION`.

**S2 — Concurrent writers.**
- **Check:** two threads from the r2d2 pool each run 100 write transactions on `entries`.
- **Decision:** if any "Transaction conflict" error appears, serialize writes with a `tokio::sync::Mutex` around `AppState::write` (one writer). Reads stay pooled.

**S3 — SQL constructs the code relies on.**
- **Check:** run each of these once:
  - `list_contains(CAST(string_split(?, ',') AS BIGINT[]), member_id)`, with NULL and with `'1,2'`;
  - `list_contains(?, entry_id)` with a bound `Value::List`;
  - `INSERT OR REPLACE INTO income_overrides …`;
  - `INSERT … ON CONFLICT DO NOTHING` on `uq_entries_recurring_date`;
  - `CHECK (day(month) = 1)`;
  - `CAST(SUM(...) AS BIGINT)`, `date_trunc('month', d)::DATE` and `datediff('month', a, b)`;
  - `json_serialize_sql` on a SELECT, a CTE, an `information_schema.tables` query and an `INSERT`.
- **Decision:**
  - Fix any construct that fails in the repository that uses it.
  - If the console guard rejects `information_schema`, either change `DB_OBJECTS_SQL` in `QueryConsole.svelte` to a supported source, or allow it explicitly in `Console::ensure_plain_select`.

**S4 — Backup round-trip.**
- **Check:**
  1. Create a DB through the repositories.
  2. Run `Backup::export`, then `Backup::restore` into an empty DB.
  3. Compare row counts per table.
  4. Insert a new member and confirm its id is above the restored maximum.
- **Decision:** fix `backup.rs` until all of this holds.

**S5 — Snapshot load speed.**
- **Check:** insert 100k entries spread over 5 years, then time `DuckSnapshotLoader::load` for one month (p95 of 50 runs).
- **Decision:** the target is under 20 ms. If it is slower, load entries with `query_arrow` instead of row mapping, or add the missing index.

**S6 — `ducklake` offline in the arm64 image.**
- **Check:** in the runtime image, run `INSTALL ducklake; LOAD ducklake;` with networking disabled after a build-time install.
- **Decision:** if it fails, note it for Phase D. It does not block the freeze.

**S7 — Build time.**
- **Check:** `time docker compose build backend`, cold and warm (cargo-chef cache).
- **Decision:** record the times only.

#### Spike results (2026-10-26, DuckDB 2.0.0-dev build `d54cbbe3df` bundled by `duckdb 1.20000.0`, linux/arm64)

The probes were throwaway and are deleted. Two defects were found *outside* the seven gates and fixed first.

**Defects found while running the probes (fixed):**
- **Release binaries did not link.** `libduckdb-sys` links its static-extension autoregister archive after the rlib that bundles `libduckdb.a`, so `duckdb_register_static_extensions` stayed undefined. Clippy only type-checks, so it never showed. Fix: `backend/.cargo/config.toml` passes `-Wl,-u,duckdb_register_static_extensions` for the two Linux targets, and `Dockerfile.rust` copies `.cargo` into every stage so `cook` and `build` use identical flags. Verified by `docker compose build backend`.
- **Empty month broke the snapshot load.** `list_contains(?, entry_id)` with an empty bound list fails to bind ("Cannot deduce template type"). The same query existed twice (loader and `Entries::with_overrides`). Both now use one helper, `Entries::overrides_of`, with `CAST(? AS BIGINT[])`; the empty list is verified.

| Gate | Result | Decision applied |
| :--- | :--- | :--- |
| S1 foreign keys | Updating a non-key column of a referenced parent works. Updating a **UNIQUE column** of a referenced parent is blocked ("Violates foreign key constraint"), and `members.name_ct` / `categories.name_ct` are UNIQUE and renamable. Deleting a referenced parent is blocked, as expected. | Keep no FKs; integrity stays in the repositories. No `0002_foreign_keys.sql`; `LATEST_VERSION` stays 1. |
| S2 concurrent writers | 2 threads × 100 inserts: 0 errors. 2 threads × 100 updates of the **same row**: 101 of 200 failed with "Conflict on tuple deletion". | Writes are serialized: `Database::exclusive()` (a mutex) is taken by `write_meta` (and so `write`) and by `Backup::restore`. Re-run: 0 errors on both probes. Reads stay pooled. |
| S3 SQL constructs | OK: `string_split` filter with NULL and `'1,99'`, bound `Value::List`, `INSERT OR REPLACE` (1 row after two writes), `ON CONFLICT DO NOTHING` (second `materialize` inserts 0), `CHECK (day(month) = 1)` rejects the 15th, `CAST(SUM)`, `date_trunc`, `datediff`, `Views::tags/projects`. Console accepts a SELECT, a CTE and `information_schema.tables`; it rejects `INSERT`, two statements, `read_csv` and `duckdb_tables()`. | Empty-list bind fixed (above). `QueryConsole.svelte` already uses `information_schema`, so no change. |
| S4 backup round trip | Export, restore into an empty DB: row counts equal for every table. A new member after restore gets id 2 (restored max 1). A second restore into a populated DB works. | None needed. |
| S5 snapshot load | 100k entries over 5 years, 1705 in the month: median 8.5 ms, p95 8.9 ms, max 13.7 ms (in-memory DB, release build). | Under the 20 ms target; row mapping stays. Re-measure on a file DB at Phase B. |
| S6 `ducklake` | `INSTALL ducklake` fails with HTTP 404 for `extensions.duckdb.org/d54cbbe3df/linux_arm64/ducklake.duckdb_extension.gz`; the offline `LOAD` therefore also fails. The crate bundles a 2.0.0 development build with no published extensions. | Does not block the freeze. Phase D must wait for a crate release whose build hash has published extensions, or build `ducklake` in. |
| S7 build time | `docker compose build backend` on this machine: cold 19 min 41 s; warm after a one-line source change 9 min 39 s (dependency layer cached, workspace crates rebuilt and linked). | Recorded only. The cause of the slow warm build was not investigated. |

### 11.3 Reference capture for Phase B — done (2026-10-26)

Run with `JIZIFIN_PASSPHRASE=… scripts/capture-legacy-reference.sh [finance.db]` (optionally `OUT_DIR=…`). The script copies the database to a temp directory, drops the copy's `magic_word` row so no login is needed, starts the legacy Python app (`jizifin-backend:python-last`, built from the unmodified `backend/Dockerfile`) on the copy, and calls the two endpoints for every month that has data. The passphrase is read from the environment at run time and is never written. Output goes to `backend/tests/reference/legacy/` (git-ignored; it holds ciphertext of household data):

| File | Content |
| :--- | :--- |
| `<month>.json` | `{paybacks, monthly_total}` as the legacy API returned them (ciphertext category and user names) |
| `totals.json` | cents and counts per month for `expense`, `income` (without `SALARY`) and `deposit` (paid), read **before** the legacy app starts, which is exactly what the migrator's `verify` step sums |
| `startup_materialized.json` | recurring expenses the legacy app inserted into the copy while starting |

What the capture contains (`backend/finance.db`, file dated 2026-10-22, confirmed by the user as the production database):
- 114 expenses: 2026-08 (16, 134 210 ¢), 2026-09 (74, 341 817 ¢), 2026-10 (23, 40 702 ¢), plus one expense dated `0026-10-08` (89 ¢, evidently a typo). That month is captured too; decide at migration whether to fix the date.
- 0 income rows, 0 settlements, 0 joint accounts, 2 users, 2 jobs.
- The legacy app's start-up generated 7 recurring expenses in 2026-10 (34 050 ¢). They are in `2026-10.json` (paybacks) but not in `totals.json`. For the parity test of 2026-10, run the Rust recurring materialization first, or compare 2026-08 and 2026-09 only.
- Personal-pay renames appear in the responses of all three months, which confirms the passphrase matched the data.

Limits of this reference: with no income rows, no settlements and no joint accounts it cannot exercise the income-ratio split, salary-derived weights, sealed-month reseal or joint-account exclusion. Those need hand-built fixtures in Phase B. `docker compose build backend` retags `jizifin-backend:latest` to the Rust image; the Python image lives on as `jizifin-backend:python-last` (the §13 rollback tag).

### 11.4 Open items to finish before the freeze

**1. Explanation trail with `?explain=1` — done.** `AuditedResult.trail` (skipped when empty); `Auditor::settlement_explained` records one line per allocated share via `EntryShares::trail`. The flag is not part of `AuditScope`, so the cache key is unchanged.

**2. `GET /entries/{id}/shares` — done.** `Auditor::entry_shares` resolves one expense with `ShareResolver` and `Resolution::allocate` (the same helper the settlement ledger uses) and checks that the shares sum to the amount (`entry_shares_sum_to_total`). `EntryEditModal.svelte` shows the result read-only as "Who bears what".

**3. `backend/Dockerfile.test.rust` — done.** No tests added.

**4. Frontend clean-up check — done.** Every hit of the grep is presentation only (chart `/ 100` conversions and tooltip `* 100` back to cents, timing, row counts).

Original instructions, kept for the freeze re-check:
- Run `grep -rnE "reduce\(|Math\.round|toFixed|\* 100|/ 100" frontend/src/lib/*.svelte frontend/src/App.svelte`.
- Every hit must be presentation only: formatting, or turning server cents into chart values (`/ 100`).
- Any money arithmetic is a defect. Move it server-side as described in §8.1.

**5. In-app docs.** `lib/docs/*.svelte` still describe FastAPI/SQLite. Update them at cutover (§13), not now.

### 11.5 Framework freeze checklist

When every item is true, tag the freeze:
- [ ] §11.1–§11.4 are done, and the spike results are recorded and applied.
- [ ] The clippy command from §0.2 passes after `cargo clean -p` of all workspace crates.
- [ ] `make wasm` and `vite build` pass.
- [ ] `docker compose build` passes for the backend and the frontend.
- [ ] The schema is final (`LATEST_VERSION` decided), and every route in §7 exists.
- [ ] This document matches the code; update §7 if any shape changed.
- [ ] Commit, then run `git tag framework-freeze`.

After the tag, production code changes only to fix defects that tests find.

---

## 12. Phase B: Tests (after the freeze, about 2–3 weeks)

Run Rust tests with the `Dockerfile.test.rust` image and `cargo nextest run`. Run frontend tests with Vitest in the frontend test container.

**1. Auditor unit tests** (`crates/jizifin-auditor/tests/`). Build `Snapshot` values by hand.
- **Allocation:** positive, zero and negative totals; equal remainders resolved by the seed; the sum always equals the total.
- **Share precedence:** one test per rule in §4.4.
- **Settlement:** two couples never owe across components; joint expenses are excluded; the offset rule applies; every invariant id fails when fed a deliberately broken value (call the invariant directly).
- **Income:** weekly, biweekly and annual normalization with ties-to-even; an override replaces the contract; one-off income adds on top.
- **Deposits:** equal, salary and fixed modes; zero income falls back to equal; rounding.
- **Budgets:** standing vs month budget precedence; proposal rounding; the summary.
- **Also:** split overview, proposals, cash flow, project settlement, and joint dashboard status transitions.
- **Port the numeric cases** from `backend/tests/test_spec_financial_cases.py`, `test_scd2_category_splits.py`, and the Vitest `numerical_precision` and `splits_allocations` suites.

**2. Store and API integration** (`crates/jizifin-api/tests/`). Use `Database::in_memory()`, build the router and drive it with `tower::ServiceExt::oneshot`. Cover:
- the auth flow, and 401 without a token;
- CRUD for every resource;
- 409 for a sealed month, 422 for a tag window, and 409 for deleting a category that is still referenced;
- pagination cursors and duplicates;
- backup export → restore;
- recurring materialize idempotency;
- the console rejecting anything other than a SELECT.

**3. Migrator.** Run it on the production copy.
- The totals must equal `totals.json`.
- The Auditor settlement for each month must equal the reference payback output (§11.3), apart from the documented R9 tie-break change. That change may differ by at most 1 cent per member per entry, and only where remainders tie.

**4. Fault injection.** A test-only allocator that drops a cent must make `bucket_zero_sum` fail and the route return 500.

**5. WASM.** Run `cargo test -p jizifin-wasm` with fixtures copied from `frontend/src/test/fixtures` (ING exports). Cover beneficiary extraction, amounts, dates and bundling.

**6. Frontend Vitest.**
- **Delete** the suites that test removed modules: `csvParser`, `duplicateDetector`, `budgeting_engine`, `splits_allocations`, `numerical_precision`, `networth_reporting`, `reconciliation_locking`, `recurrence_scheduling`, `ledger_transfers`, `multicurrency`, `mutations_reclassification`, `import_export_analytics`, `concurrency_security`, `categories_tags`, and the old `api.test.js`.
- **Keep or rewrite:**
  - `crypto`, `colorUtils`, `device_profiles`, `customization_personas`;
  - component rendering with a mocked `api.js`;
  - `format.js` (`toCents`);
  - the `csv.js` boundary with the built WASM.

**7. Exit criteria:**
- all suites are green;
- every historical month reseals without drift;
- Auditor p95 is under 50 ms for a one-month scope.

---

## 13. Phase C: Cutover (about 1 day)

1. Back up the legacy `backend/finance.db` by copying it off the machine. Tag the current Python image as `jizifin-backend:python-last`.
2. Stop the Python backend: `docker compose stop backend`.
3. Run `make migrate`, with `JIZIFIN_PASSPHRASE` exported in the shell for this command only; never store it. Check the printed report: the totals match and the months are resealed.
4. Run `make deploy-backend`, then `docker compose up -d --build frontend`.
5. Log in and compare the dashboard and settlements of the last three months with the reference.
6. Delete the legacy code in one commit:
   - `backend/app/`, the Python `backend/tests/`, `backend/pyproject.toml`, `backend/uv.lock` and `backend/pytest.ini`;
   - the Python `backend/Dockerfile` and `backend/Dockerfile.test`. Rename `Dockerfile.rust` → `Dockerfile` and `Dockerfile.test.rust` → `Dockerfile.test`, and update compose and the Makefile;
   - `crates/jizifin-migrate`, the `migrate` compose service, `make migrate`, and the migrator `COPY` in the Dockerfile;
   - the Python test targets in the `Makefile` (`test-backend`). Replace them with the nextest command.
7. Update the docs:
   - `AGENTS.md`: the stack, the schema and encryption matrix from §5, the Auditor, the API and the test commands;
   - `README.md` and `.agents/rules/*.md`;
   - `frontend/src/lib/docs/*.svelte`: `BackendDocs`, `SystemDocs`, `FrontendDocs`, `GettingStartedDocs`.

**Rollback:** redeploy `jizifin-backend:python-last`. The migrator never modifies the legacy file.

---

## 14. Phase D: Lakehouse (future)

- At image build time, `INSTALL ducklake` into a baked extension directory, so nothing is downloaded at runtime.
- `ATTACH 'ducklake:/data/lake/catalog.ducklake' AS lake (DATA_PATH '/data/lake/files')`, with a nightly scheduler job that copies `entries` and `month_seals`.
- Optionally, let `SnapshotLoader` read old history from the lake.

---

## 15. Parity hazards (Python → Rust)

| Hazard | Rule used |
| :--- | :--- |
| Python `round()` rounds ties to even | `f64::round_ties_even` |
| `math.floor` vs truncation | `.floor()` before casting; `div_euclid`/`rem_euclid` for integer division |
| Tie-break seed | `"{seed}:{member_id}"`. All history is resealed by the migrator (R9); there is no old-seed path. |
| Base64URL | `URL_SAFE_NO_PAD` |
| A failed decryption returns the input unchanged | Mirrored in the migrator and in `crypto.js` |
| `'{month}-31'` string dates | `Month::last_day()` on the server, `monthBounds()` on the client |
| Error envelope `{"detail": …}` | `ApiError` |

---

## 16. Decisions (resolved)

1. **Tie-break:** the member-id seed everywhere, with a full reseal (R9).
2. **No legacy** (R10): no v1 API, no legacy SQLite import endpoint, no `joint_account` singleton and no shims. The migrator is deleted after cutover.
3. **Personal-pay categories:** a `pay_mode` column on `categories`.
4. **Special deduction:** stored as `OffsetRule` data, seeded with the Jane/John rule.
5. **Explanation trail:** only with `?explain=1` (still to implement, §11.4). The snapshot cache holds 64 entries.
6. **Dependencies:** no `garde`, no cron crate, no separate app crate.
7. **Split of responsibilities:** plain aggregates are store views; contentious values are Auditor results (§0.1 rule 4).

## 17. Effort (remaining)

| Work | Estimate |
| :--- | :--- |
| Remaining Phase A (§11) | 2–4 days |
| Phase B tests | 2–3 weeks |
| Phase C cutover | 1 day |
