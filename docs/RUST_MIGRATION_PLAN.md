# Jizifin Migration Plan: FastAPI/SQLite → Rust/DuckDB 2.0, Thin Client, In-Backend Auditor

**Status:** Planning. Nothing in this document is implemented yet.
**Scope:** `backend/` (rewrite), database (DuckDB 2.0.0 only, schema redesigned), `frontend/src/lib/*` (logic removal), `docker-compose.yml`, `Caddyfile`, `Makefile`.
**Non-goals:** UI redesign, changing the zero-knowledge crypto contract, moving off Docker/Caddy.
**Hard constraints:**
- DuckDB is the only database. There is no fallback engine.
- Public ports stay `:80/:443`, served by Caddy.
- Tests are written and run **only after the framework freeze** (Phase B). For the migration branch, this overrides the per-change verification in AGENTS.md §7.

---

## 1. Starting point (as of `3a98d48`)

| Area | Today |
| :--- | :--- |
| HTTP API | FastAPI: 90 routes plus `/ws/finance`, all in `backend/app/main.py` (4,095 LOC) |
| Schema | 24 tables and 9 views. Ciphertext natural keys, about 20 `CASCADE` foreign keys, and a legacy `joint_account` singleton. |
| Contentious math | Spread across `main.py` handlers, SQL views and the frontend: `calculateSalaryRatios` in `SplitManager.svelte`, 2× `toMonthlyEquivalent`, and deposit targets in `JointAccountTab.svelte` |
| Public edge | Caddy `:80/:443`: `/api/*` and `/ws/*` go to `backend:8000`; everything else goes to `frontend:5173` |
| Privacy | The server only sees ciphertext for names and categories |

---

## 2. Target architecture

```
              :80 / :443  (unchanged, Caddy never restarts)
                     │
             ┌───────┴────────┐
             │     caddy      │  lb_try_duration retry during swap
             └───┬────────┬───┘
   /api/*, /ws/* │        │ everything else
                 ▼        ▼
  ┌───────────────────────────┐  ┌──────────────┐
  │ backend:8000  (Rust)      │  │frontend:5173 │  render + decrypt only
  │  http → services          │  └──────────────┘
  │         ├── CRUD repos    │
  │         └── Auditor ◀─────┼── sole owner of contentious calculations
  │               │ read snapshot (Arrow)
  │  ┌────────────▼────────┐  │
  │  │ DuckDB 2.0 embedded │──┼─▶ /data/finance.duckdb
  │  └─────────────────────┘  │   /data/lake/ (DuckLake, Phase D)
  └───────────────────────────┘
```

There is one process and one container. The auditor is a component inside the backend, not a separate service.

### 2.1 Rust stack

| Concern | Crate |
| :--- | :--- |
| Runtime, HTTP, WebSocket | `tokio`, `axum` 0.8, `tower-http` |
| DTOs, validation | `serde` into domain value types (`Ciphertext`, `Month`, `Cents`) plus `validate()` on input structs |
| **Database** | **`duckdb = { version = "~1.20000.0", features = ["bundled", "parquet", "json", "r2d2", "chrono", "serde_json"] }`**. This is DuckDB **2.0.0**: since DuckDB 1.5.0 the crate is versioned `1.<major><minor><patch>.x` ([DuckDB 2.0 Rust docs](https://duckdb.org/docs/2.0/clients/rust/overview)). |
| Pool | `r2d2` (duckdb feature). Every call runs inside `spawn_blocking`, because the driver is synchronous. |
| Scheduler | `tokio` loop: recurring catch-up at start and daily after 00:00 UTC |
| Crypto | `aes-gcm`, `pbkdf2`, `sha2`, `base64` (`URL_SAFE_NO_PAD`) |
| Dates, errors, logs | `chrono`, `thiserror`, `tracing` |

`parquet` and `json` are statically linked. `ducklake` (Phase D) will be `INSTALL`ed at image build time into a baked extension directory, so nothing is downloaded at runtime.

---

## 3. Class design and the small-method rule

A "class" is a `struct` with private fields plus an `impl` block. Polymorphism comes from traits, and dependency injection from `Arc<dyn Trait>` held in the axum `AppState`.

| Rule | Limit | Enforcement |
| :--- | :--- | :--- |
| Lines per function | ≤ 20 | `clippy::too_many_lines`, `too-many-lines-threshold = 20` |
| Parameters | ≤ 4; above that, pass a parameter struct | `too-many-arguments-threshold = 4` |
| Cognitive complexity | ≤ 8 | `cognitive-complexity-threshold = 8` |
| Responsibility | One verb per method, one level of abstraction | Review checklist |
| Branching | No service `match` with more than 5 arms; dispatch through a trait instead | Review checklist |
| Encapsulation | Private fields; constructors validate | `#![deny(missing_docs)]` on `pub` items in the domain crate |

Every crate root sets these limits in `clippy.toml` and adds `#![deny(clippy::too_many_lines, clippy::cognitive_complexity, clippy::too_many_arguments)]`.

### 3.1 Workspace layout

```
backend/
├── Cargo.toml, clippy.toml
├── crates/
│   ├── jizifin-domain/          # entities + value objects, no IO, wasm32-compatible
│   │   ├── money.rs             # Cents(i64)
│   │   ├── calendar.rs          # Month, IsoDate, Period { from, to }
│   │   ├── cipher.rs            # Ciphertext: Eq/Hash, no Ord
│   │   └── entities/            # Member, Category, Account, Entry, Agreement, Job …
│   ├── jizifin-auditor/         # library crate: the Auditor (§4)
│   ├── jizifin-store/           # DuckDB adapters, migrations/*.sql, SnapshotLoader
│   ├── jizifin-api/             # bin: router, DTOs, auth, WS hub
│   ├── jizifin-migrate/         # bin: legacy SQLite → new DuckDB schema
│   └── jizifin-wasm/            # CSV parser for the browser
```

---

## 4. The Auditor: in-backend owner of contentious calculations

### 4.1 Role

The Auditor is a long-lived entity inside the backend, held in `AppState` as `Arc<Auditor>`.

- **Single source of truth.** It is the only code that computes contentious values. Nothing contentious is stored as a precomputed column, computed in a SQL view, or computed in the browser.
- **On each request it:**
  1. Pulls the raw rows it needs from DuckDB.
  2. Builds an in-memory snapshot.
  3. Calculates.
  4. Self-checks the result.
  5. Returns an `AuditedResult<T>`.

**Contentious items (Auditor-owned):**

| Item | Calculator class | Endpoint(s) |
| :--- | :--- | :--- |
| Monthly settlement, i.e. who owes whom | `SettlementCalculator` → `PaybackEngine` | `GET /settlements/{month}`, `GET /analytics/paybacks` |
| Locking a month | `SettlementSealer` | `POST /settlements/{month}/lock` |
| Proposed joint-account deposits | `DepositProposer` | `GET /accounts/{id}/deposit-proposal?month=` |
| Joint-account balance and shortfall | `JointBalanceCalculator` | `GET /accounts/{id}/dashboard?month=` |
| Proposed budgets | `BudgetProposer` | `GET /budgets/proposals?month=` |
| Budget usage vs limit | `BudgetEvaluator` | `GET /budgets?month=` |
| Salary-ratio split proposals | `SalaryRatioCalculator` | `GET /splits/salary-ratios?month=` |
| Effective income per member, including overrides | `IncomeResolver` → `SalaryNormalizer` | `GET /income/effective?month=` |
| Per-entry share allocation | `ShareResolver` → `LargestRemainderAllocator` | used internally; `GET /entries/{id}/shares` |
| Project equity and settlement | `ProjectSettlementCalculator` | `GET /projects/{id}/settlement` |

Non-contentious reads stay as plain SQL in repositories: lists, totals by category, chart series, tag totals.

### 4.2 Request flow

```
handler ──▶ Auditor::settlement(scope)
              ├─ self.snapshot(scope)        → SnapshotLoader (DuckDB, read-only connection)
              ├─ self.calculator().run(&snap) → pure in-memory math
              ├─ self.verify(&result, &snap)  → InvariantSet
              └─ self.wrap(result, &snap)     → AuditedResult<T>
```

```rust
pub struct Auditor {
    loader: Arc<dyn SnapshotLoader>,
    invariants: InvariantSet,
    cache: SnapshotCache,          // keyed by (AuditScope, data_version)
}

impl Auditor {
    pub fn deposit_proposal(&self, scope: AuditScope) -> AuditResult<DepositProposal> {
        let snap = self.snapshot(&scope)?;
        let proposal = DepositProposer::new(&snap).propose();
        self.invariants.check(&proposal, &snap)?;
        Ok(self.wrap(proposal, &snap))
    }
    fn snapshot(&self, scope: &AuditScope) -> AuditResult<Arc<Snapshot>> { /* cache or load */ }
    fn wrap<T>(&self, value: T, snap: &Snapshot) -> AuditedResult<T> { /* attach hash + trail */ }
}
```

### 4.3 Components

| Class | Responsibility |
| :--- | :--- |
| `AuditScope` | A value object: `{ period, members?, account?, project? }` |
| `SnapshotLoader` (trait) / `DuckSnapshotLoader` | One query per entity type, filtered by scope with SARGable date ranges, loaded with `query_arrow` into typed vectors. No contentious math happens in SQL. |
| `Snapshot` | Immutable in-memory set: members, categories, agreements, entries, jobs, overrides, accounts, locks. It exposes indexed lookups such as `agreement_for(category, date)`. |
| `SnapshotCache` | An LRU keyed by `(scope, data_version)`. Every write transaction bumps an `AtomicU64` `data_version`, so stale entries are never served. |
| Calculators (§4.1) | Pure classes, each one small, with methods under the §3 limits |
| `LargestRemainderAllocator` | Signed floor, leftover cents distributed in descending remainder order, SHA-256 salted tie-break |
| `PaybackEngine` | Personal-pay remap, joint exclusion, `ComponentPartitioner` (union-find), `DebtSimplifier` |
| `InvariantSet` | A list of `Invariant` trait objects, run before any value is returned |
| `AuditedResult<T>` | `{ value, inputs_hash, data_version, trail: Vec<TrailLine>, checks: Vec<CheckId> }` |

### 4.4 Invariants (self-checks on every result)

| Invariant | Rule |
| :--- | :--- |
| `ZeroSum` | Σ allocated shares = entry total, including negative refunds |
| `PercentTotal` | Agreement percentages sum to 100 within ±1e-4 |
| `NetZeroPerComponent` | Settlement net positions sum to 0 within each connected component, and no debt crosses components |
| `JointExcluded` | Joint-account entries never appear in peer paybacks |
| `DepositCoversExpected` | Σ proposed deposits ≥ expected costs × (1 + safety margin) − current surplus |
| `NonNegativeBudget` | Proposed limits are ≥ 0 and are whole cents |
| `LockedMonthStable` | For a sealed month, the recomputed result hash equals the stored seal hash. A mismatch is reported as data drift (see below). |

A violated invariant returns **HTTP 500 `{"detail": "audit invariant <id> failed"}`** and logs the scope plus `inputs_hash`. The Auditor never returns an unverified number.

### 4.5 Sealed months

`SettlementSealer` stores the final settlement in `month_seals`: the result JSON, `inputs_hash`, and `sealed_at`.

- Reads for a sealed month return the stored seal.
- The Auditor still recomputes the month and runs `LockedMonthStable`.
- If later edits changed the inputs, the API returns the seal together with a `drift: true` flag instead of silently changing history.
- Writes into a sealed period are rejected with 409 by `PeriodGuard`.

### 4.6 Trail

`trail` is a short list of explanation lines built from IDs and cents only, for example `share(entry 812, member 2) = 1,234 via agreement 17`. The frontend can show it after decrypting labels. It is returned only with `?explain=1`.

---

## 5. Schema redesign (DuckDB-native)

The old schema is not kept. The new schema is designed around these principles:

- Surrogate `BIGINT` keys everywhere.
- No reliance on `CASCADE`: deletes are explicit, ordered steps inside a `UnitOfWork`.
- One ledger table instead of four.
- No derived or contentious data stored, except month seals.
- A Parquet-friendly layout for the lakehouse.

### 5.1 Tables

| Table | Columns (ciphertext marked *ct*) | Replaces |
| :--- | :--- | :--- |
| `app_config` | `key PK`, `value` | same |
| `sessions` | `token_sha256 PK`, `last_active TIMESTAMP` | in-memory sessions |
| `members` | `id PK`, `name_ct UNIQUE`, `color`, `is_active BOOLEAN`, `created_at` | `users` |
| `categories` | `id PK`, `kind ENUM('expense','income')`, `name_ct`, `pay_mode ENUM('shared','personal')`, `UNIQUE(kind, name_ct)` | `splits`, `income_categories`, hard-coded personal-pay list |
| `agreements` | `id PK`, `category_id`, `valid_from DATE`, `valid_to DATE NULL`, `note_ct` | `split_agreements` (the baseline is an open-ended agreement) |
| `agreement_shares` | `agreement_id`, `member_id`, `pct DOUBLE`, `PK(agreement_id, member_id)` | `split_allocations` |
| `accounts` | `id PK`, `kind ENUM('joint')`, `name_ct`, `safety_margin_pct`, `deposit_mode ENUM('equal','salary_ratio','fixed')` | `joint_accounts`, `joint_account` singleton |
| `account_members` | `account_id`, `member_id`, `fixed_deposit_cents NULL` | `joint_account_members`, `joint_account_deposits` |
| `account_categories` | `account_id`, `category_id`, `expected_cents` | `joint_account_categories`, `joint_account_expected_costs` |
| `entries` | `id PK`, `kind ENUM('expense','income','deposit','correction')`, `entry_date DATE`, `amount_cents BIGINT` (signed), `member_id`, `category_id NULL`, `account_id NULL`, `project_id NULL`, `tag_id NULL`, `recurring_id NULL`, `name_ct`, `note_ct`, `created_at` | `expenses`, `income`, `joint_account_monthly_deposits`, `joint_account_corrections` |
| `entry_share_overrides` | `entry_id`, `member_id`, `pct`, `PK(entry_id, member_id)` | `expense_overrides` |
| `recurring_templates` | `id PK`, `kind`, `amount_cents`, `member_id`, `category_id`, `account_id NULL`, `day_of_month`, `frequency ENUM`, `valid_from`, `valid_to NULL`, `name_ct`, `is_active` | `recurring_expenses` |
| `jobs` | `id PK`, `member_id`, `name_ct`, `amount_cents`, `frequency ENUM('monthly','weekly','biweekly','annual')`, `valid_from`, `valid_to NULL`, `notes_ct` | same |
| `income_overrides` | `member_id`, `month DATE` (first day of month), `amount_cents`, `note_ct`, `PK(member_id, month)` | `salary_overrides` |
| `budgets` | `id PK`, `category_id`, `valid_from DATE`, `valid_to DATE NULL` (month starts; NULL = standing), `limit_cents`, `source ENUM('manual','accepted_proposal')` | `budgets` (incl. `'ALL'` rows) |
| `projects` | `id PK`, `name_ct`, `target_cents`, `color` | same |
| `project_members` | `project_id`, `member_id` | `project_users` |
| `tags` | `id PK`, `name_ct`, `description_ct`, `valid_from NULL`, `valid_to NULL` | same |
| `month_seals` | `month DATE PK`, `result_json JSON`, `inputs_hash BLOB`, `sealed_at` | `settlements` |
| `offset_rules` | `id PK`, `debtor_category_id`, `creditor_category_id`, `debtor_member_id`, `creditor_member_id`, `is_active` | hard-coded special deduction |

That is 20 tables, down from 24. The 9 old views are dropped: contentious results come from the Auditor, and the non-contentious aggregates are repository queries.

- **Indexes:** `entries(entry_date, id)`, `entries(member_id, entry_date)`, `entries(category_id, entry_date)`, `entries(account_id, entry_date)`, `agreements(category_id, valid_from)`. Each one is kept only if the spike shows a gain over DuckDB zone maps.
- **Integrity:** primary and unique keys are declared. Foreign keys are declared only where the spike confirms that DuckDB 2.0 does not block updates or deletes on referenced rows. Elsewhere, the repository enforces them inside the `UnitOfWork`.
- **Encryption matrix:** every `*_ct` column is deterministic AES-GCM Base64URL. Everything else is plaintext. The query rules (equality, `IN`, `GROUP BY` only on `*_ct`) carry over unchanged.
- **Migrations:** numbered `.sql` files plus `schema_version`, applied at startup in one transaction.

### 5.2 Legacy data

`jizifin-migrate` (one-shot, deleted after cutover) reads the legacy SQLite file **read-only** through `rusqlite` (no DuckDB `sqlite` extension, so nothing is downloaded) and writes a fresh DuckDB file through the store repositories, so every row passes the same validation as an API write. Everything runs in one transaction; any failure rolls it back.

1. Members, categories (personal-pay categories get `pay_mode = 'personal'`), active SCD2 agreements, baseline allocations (as open-ended agreements from `0001-01-01`), accounts (roster ∪ deposit plans, `deposit_split_mode` even/salary/manual → equal/salary_ratio/fixed, gross `expected_total_cents` kept as an override), projects, tags.
2. `expenses`, non-`SALARY` `income`, paid `joint_account_monthly_deposits` and `joint_account_corrections` become `entries`. Each account gets one opening correction bridging its stored legacy `balance_cents` and the derived balance.
3. Active jobs copy over; members without any job get monthly jobs from their legacy `SALARY` rows (one job per amount change). Salary overrides, recurring templates (and links to the entries the old scheduler generated, so nothing is regenerated), budgets (`'ALL'` → standing budget), the Jane/John offset rule, and the auth proof (`SHA-256` of the legacy magic word, which is exactly the client's login proof).
4. Verification before commit: per-kind cent totals (expense, income, deposit) must equal the legacy source.
5. After commit, the Auditor reseals every month listed in legacy `settlements`.

`JIZIFIN_PASSPHRASE` is used only to recompute the ciphertext of the names the legacy code hard-coded (`PERSONAL COST`, `LEISURE`, `GIFT`, `Combined Fixed`, `Apartment`, `Jane`, `John`, `SALARY`). It is never stored.

**Backup:** `GET /backup` returns the whole ledger (minus sessions) as one DuckDB file (`COPY FROM DATABASE`). `POST /backup` restores such a file if its `schema_version` matches. The file holds only ciphertext and numbers, so the server never needs the passphrase. Legacy SQLite backups are not accepted; convert them once with the migrator.

### 5.3 Spike gate (about 1 day; fix within DuckDB, no fallback)

- [ ] Foreign-key behaviour in 2.0 on update/delete of referenced rows. This decides which foreign keys are declared (§5.1).
- [ ] p95 of `SnapshotLoader` for a one-month scope on 100k entries, with `query_arrow` vs row mapping.
- [ ] Concurrent write conflicts. Decide between retry-on-conflict and a single writer connection behind a `tokio::Mutex`.
- [ ] The `ducklake` extension loads offline in the arm64 image.
- [ ] `COPY FROM DATABASE` / `ATTACH` round-trip for backup and restore, including the sequence advance after restore.
- [ ] Bundled build time with `cargo-chef`.

Every outcome has an in-DuckDB answer: app-level integrity, a single writer, or Arrow loading.

---

## 6. Thin frontend

The browser does four things only:

1. Encrypt input.
2. Decrypt labels once per distinct ciphertext, memoized.
3. Render server responses, including `AuditedResult` values and their optional trail.
4. Run the plaintext-only work: substring search and alphabetical sort on the current page, and CSV parsing through `jizifin-wasm`.

| Remove from frontend | File | Replacement |
| :--- | :--- | :--- |
| `calculateSalaryRatios` | `SplitManager.svelte` | Auditor `SalaryRatioCalculator` |
| `toMonthlyEquivalent` ×2 | `AnalyticsSummary.svelte`, `IncomeTab.svelte` | Auditor `IncomeResolver` |
| Deposit targets, expected sums | `JointAccountTab.svelte` | Auditor `DepositProposer`, `JointBalanceCalculator` |
| Budget progress | `BudgetManager.svelte` | Auditor `BudgetEvaluator` |
| Allocation totals and validation | `SplitManager.svelte` | `AgreementInput::validate` (shares total 100 ± 1e-4) |
| Drilldowns, filters, chart datasets | `DrilldownModal`, `ExpenseList`, charts | Repository SQL, `GET /views/*`, keyset pagination |
| Duplicate detection | `duplicateDetector.js` | `POST /entries/duplicates` (ciphertext + cents equality) |
| `csvParser.js` | — | `jizifin-wasm` |
| Derived store math | `stores.js` | Stores hold only responses and UI state |

The API moves to a v2 shape that matches the new schema: `/members`, `/categories`, `/entries`, `/accounts`. The frontend is rewritten against it in Phase A, so no v1 compatibility layer is needed.

---

## 7. Same public port, deploy only after a successful build

1. The backend keeps the compose service `backend`, port `8000`, and Caddy's `/api/*` and `/ws/*` routes.
2. `make deploy-backend` swaps containers only after the build succeeds; a failed build changes nothing:
   ```bash
   docker compose build backend && docker compose up -d --no-deps --wait backend
   ```
3. Caddy holds requests during the few-second swap. `RealtimeChart.svelte` already reconnects after 4 s.
   ```caddy
   reverse_proxy backend:8000 {
       lb_try_duration 30s
       lb_try_interval 250ms
       health_uri /health
   }
   ```
4. DuckDB holds an exclusive per-process file lock, so deploys are stop-then-start rather than blue/green.
5. The volume changes from the single file `./backend/finance.db` to the directory `./data:/data`, which holds the DB, its WAL and `lake/`.
6. The image is distroless, so the health check calls `jizifin-api --healthcheck`.

---

## 8. Phases

### Phase A: Framework (no tests written or run) — about 5–7 weeks

1. Spike gate (§5.3).
2. Capture reference material for Phase B: production-copy totals per month and the current settlement and payback outputs per month. These are artifacts, not tests.
3. Workspace, `clippy.toml` lint gates, port traits, domain classes. `cargo check` and clippy are allowed.
4. `jizifin-store`: schema, migrations, repositories, `DuckSnapshotLoader`, `UnitOfWork`, `data_version` bump.
5. `jizifin-auditor`: snapshot, cache, all calculators, `InvariantSet`, `SettlementSealer`, `PeriodGuard`.
6. `jizifin-api`: v2 routes, WS hub, auth/sessions, backup, `/query` (allow only statements that `json_serialize_sql` reports as `SELECT`).
   - **Auth contract (new):** the client derives a proof from the passphrase (an AES-GCM ciphertext of a fixed marker). `POST /auth/init` stores only `SHA-256(proof)`; `POST /auth/login` compares hashes and issues a random token that is also stored hashed (`sessions`). `GET /auth/salt` is removed: it used to return the exact login proof to unauthenticated callers.
   - Uninitialized instances expose only `/health` and `/auth/*`.
7. `jizifin-migrate`.
8. `jizifin-wasm` and the thin-frontend rewrite (§6).
9. Docker, compose, Caddyfile and Makefile (§7, §9).
10. **Freeze.** Everything compiles and lints clean, the schema is final and every route exists. Tag `framework-freeze`; after that, code changes only for defects found by tests.

### Phase B: Tests (after the freeze) — about 2–3 weeks

1. Auditor unit tests: every calculator and invariant, plus the parity cases in §10. Port the math cases from the Python and Vitest suites, e.g. `test_spec_financial_cases.py`, `test_scd2_category_splits.py`, `numerical_precision`, `splits_allocations`.
2. Store and API integration tests on in-memory DuckDB (`Connection::open_in_memory`) with `tower::ServiceExt::oneshot`.
3. Migrator test: production copy → new schema. Checksums must match, and the Auditor's settlements must equal the Phase A reference outputs month by month.
4. Fault injection: a deliberately broken allocator must trip `ZeroSum` and return 500.
5. Frontend Vitest suite reduced to rendering, crypto and the WASM boundary.
6. **Exit criteria:** all suites green, every historical month re-seals without drift, and Auditor p95 under 50 ms for a one-month scope.

### Phase C: Cutover — about 1 day

1. Stop the Python backend.
2. `docker compose run --rm migrate`.
3. `make deploy-backend`, then deploy the frontend.

**Rollback:** redeploy `jizifin-backend:python-last`. The migrator never modifies `finance.db`, so the old database is intact.

### Phase D: Lakehouse (future)

- `ATTACH 'ducklake:/data/lake/catalog.ducklake' AS lake (DATA_PATH '/data/lake/files')`, with a nightly snapshot of `entries` and `month_seals`.
- `SnapshotLoader` can then read history from the lake.

---

## 9. Containers and docs

- **Image:** `rust:1-bookworm` + `cargo-chef` builder → `gcr.io/distroless/cc-debian12`, multi-arch `linux/arm64,linux/amd64`, with the baked DuckDB extension directory.
- **`Dockerfile.test`:** Rust toolchain + `cargo-nextest`, used in Phase B only:
  ```bash
  docker run --rm -v $(pwd)/backend:/app jizifin-backend-test cargo nextest run
  ```
- **Native development:** `cargo run -p jizifin-api` on Apple Silicon. DuckDB is bundled, so no Homebrew install is needed.
- **Docs to update at the freeze and at cutover:** `AGENTS.md` (stack, new schema and encryption matrix, Auditor, test policy), `README.md`, `.agents/rules/*.md`, `BackendDocs.svelte`, `SystemDocs.svelte`.

---

## 10. Parity hazards (Python → Rust)

| Hazard | Fix |
| :--- | :--- |
| Python `round()` rounds ties to even; Rust `f64::round` rounds ties away from zero | `round_ties_even` |
| `math.floor` vs `as i64` truncation | `.floor()` before the cast |
| Tie-break input `f"{seed}:{user}"` | Seed is now `"{entry_id}:{member_id}"`. All history is resealed by the migrator; no old-salt compatibility. |
| `//` floors vs `/` truncation | `div_euclid` |
| Base64URL encoding | `URL_SAFE_NO_PAD`, lenient padding on decode |
| `decrypt_text` returns its input on failure | Mirror this behaviour; do not use `?` |
| Error envelope `{"detail": …}` read by the client | Custom `ApiError` and `JsonBody<T>` extractor |
| `'{month}-31'` string dates | `Month::last_day()` |

---

## 11. Decisions (resolved)

1. **Tie-break salt:** new member-ID seed everywhere. The migrator reseals all history. No legacy salt path.
2. **No legacy or redundancy:** no v1 API, no legacy SQLite import, no `joint_account` singleton, no compatibility shims. The migrator is deleted after cutover.
3. **Personal-pay categories:** `pay_mode` on `categories`. The migrator seeds `personal` for the categories the old code hard-coded.
4. **Special deduction rule:** becomes an `OffsetRule` entity (data, not code). The migrator seeds the existing Jane/John rule.
5. **Trail:** returned only with `?explain=1`. `SnapshotCache` holds 64 entries.

## 12. Effort

| Phase | Estimate |
| :--- | :--- |
| A: framework | 5–7 weeks |
| B: tests | 2–3 weeks |
| C: cutover | 1 day |
| **Total** | **about 7–10 weeks** |
