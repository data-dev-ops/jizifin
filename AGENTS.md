# 🤖 FinanceTracker Agent Directives

**CONTEXT:** Monorepo personal finance tracker (multi-user household), running on Rust + DuckDB 2.0 with a thin Svelte client.
**PRIME DIRECTIVE:** Follow the stack, execution paths, schema and crypto contract below. Prioritise zero regression, small methods, deterministic integer-cent maths and zero-knowledge privacy.
**DEEP REFERENCE:** [docs/RUST_MIGRATION_PLAN.md](docs/RUST_MIGRATION_PLAN.md) holds the design history, the HTTP API tables, the spike results and the parity hazards. The Python/SQLite implementation is gone; it lives only in git (`git show legacy-python`).

---

## ⚙️ 1. EXECUTION (Docker first)

Never rely on host `cargo` or `npm`; they may be missing. Run from the repo root.

| Task | Command |
| :--- | :--- |
| Lint + type-check the Rust workspace | `make lint-backend` (clippy with `-D warnings`) |
| Backend tests | `make test-backend` (cargo-nextest in `jizifin-backend-test`) |
| Build the WASM CSV parser (needed before frontend dev/build/tests) | `make wasm` |
| Frontend tests | `make test-frontend` (always `-- --run`; Vitest otherwise hangs in watch mode) |
| Frontend production build | `cd frontend && docker run --rm -v "$PWD:/app" -v jizifin-node-modules:/app/node_modules -w /app node:22-alpine sh -c 'npm install --include=optional --no-audit --no-fund >/dev/null 2>&1; npx vite build'`, then `git checkout frontend/package-lock.json` |
| Deploy a new backend | `make deploy-backend` (builds first; the running container is swapped only if the build succeeds) |
| Full stack | `docker compose up -d backend frontend caddy` |

Notes:
- Stale Docker fingerprints: `touch backend/crates/*/src/lib.rs backend/crates/*/src/main.rs` before a clippy run.
- `backend/.cargo/config.toml` forces a linker symbol that `libduckdb-sys` needs; do not remove it. Without it the release binary does not link, and clippy cannot tell.
- Never write the household passphrase to a file, log, env file or document.
- Do not run test suites for documentation-only or git-only requests.

---

## 🏗️ 2. ARCHITECTURE

```
Caddy :80/:443 ──/api/* (prefix stripped), /ws/*──▶ backend:8000 (axum)
               └─ everything else ───────────────▶ frontend:5173 (Svelte)
backend: routes ─▶ repositories (CRUD, /views) ─▶ DuckDB file /data/finance.duckdb
                └▶ Auditor (in-process) ─ snapshot load ─▶ calculators ─▶ invariants ─▶ AuditedResult
```

### Backend (`backend/`, Rust workspace)
| Crate | Role |
| :--- | :--- |
| `jizifin-domain` | Pure types, no IO: `Cents`, `Month`, `Ciphertext`, typed ids, entities |
| `jizifin-auditor` | Sole owner of contentious maths; snapshot cache; invariants |
| `jizifin-store` | DuckDB schema/migrations, one repository struct per aggregate, backup/restore, `DuckSnapshotLoader` |
| `jizifin-api` | axum routes, auth/sessions, WebSocket hub, scheduler, SQL console |
| `jizifin-wasm` | CSV bank-statement parser compiled to WASM for the browser |

Rules that apply to every change:
1. **DuckDB only.** No other engine, no ORM, no fallback. Write native SQL in repositories.
2. **No legacy, no redundancy.** No compatibility shims, no dead code, no duplicated helpers: extract one.
3. **Small methods** (clippy-enforced, `backend/clippy.toml`): ≤ 20 lines, ≤ 4 parameters (use a parameter struct beyond that), cognitive complexity ≤ 8. Put `tracing::…!` calls in tiny helpers. A new crate must add `[lints] workspace = true`.
4. **Contentious maths lives in the Auditor.** Settlements, deposit proposals, budgets, income, ratios, split resolution, cash flow and project equity are computed only in `jizifin-auditor`. Plain aggregates (sums, counts, shares of a total) are store repositories exposed as `/views/*`.
5. **Integer cents everywhere.** Amounts are `INTEGER`/`BIGINT` cents and `Cents(i64)` in Rust; the browser formats them for display only. Never truncate percentage floats.
6. **Single writer.** `Database::exclusive()` serialises writes (DuckDB aborts overlapping updates of one row). Reads are pooled. Every DB call runs in `spawn_blocking` via `AppState::read/write/write_meta`.
7. Every `pub` item has a `///` doc line. Errors are `{"detail": "..."}`: 422 invalid, 401 no session, 404 missing, 409 conflict/sealed month/still referenced, 500 internal or failed audit invariant.

### Auditor
Every public method runs `snapshot → calculate → verify invariants → AuditedResult { value, inputs_hash, data_version, checks }`. A failing invariant is HTTP 500 and the value is never returned. Snapshots are cached per `(scope, data_version)` (LRU of 64); `Database::write` bumps the version.

- **Allocation:** signed Hare-Niemeyer (largest remainder) with `.floor()`; ties broken by SHA-256 of `"{seed}:{member_id}"` (entries use the entry id as seed).
- **Share precedence:** personal pay → entry override → income ratio → agreement in force (bounded beats open-ended, then later `valid_from`, then higher id) → equal split.
- **Settlement:** per-category buckets, net = paid − expected, offset rules, connected-component isolation, greedy debt simplification. Expenses charged to a joint account (`account_id` set) never enter it; expenses paid out of pocket in a joint category do. Offset rules count a member's spend in shared and personal buckets alike, and members tied by an active rule settle in one component.
- **Income:** a job pays `days worked in the month ÷ days in the month` of its monthly equivalent (bounded by `valid_from` and `valid_to`). A job that starts mid-month right after another of the member's jobs ended that month pays nothing then; its first payout, one month later, also covers the days worked in the start month. Project equity of a joint account is the cumulative deposits up to the entry date.
- **Sealed months** (`month_seals`): `GET /settlements/{month}` returns the stored result plus a `drift` flag; entry writes in a sealed month fail with 409, as do job, income-override and agreement changes that touch a sealed month. The drift check always recomputes the whole household, whatever `?members=` says. `DELETE /settlements/{month}/seal` unseals (event `settlements.unsealed`).
- Endpoints: `/settlements/{month}[?explain=1]`, `/accounts/{id}/deposit-proposal`, `/accounts/{id}/dashboard`, `/budgets/status`, `/budgets/proposals`, `/splits/salary-ratios|effective|proposal`, `/income/effective`, `/cashflow`, `/projects/{id}/settlement`, `/entries/{id}/shares`. Full shapes: plan §7.

### Frontend (`frontend/`, Svelte + Tailwind + Chart.js)
The browser does the minimum:
1. Derives the AES-GCM key and sends the auth proof.
2. Encrypts text fields before writes (`seal`) and decrypts labels after reads (`open`).
3. Renders server values and formats cents/dates (`format.js`).
4. Plaintext-only work: substring search and sorting of the loaded page, CSV parsing in WASM.

**It does not sum, average, round, split or compare money.** If a screen needs a number the server does not provide, add it to the Auditor (contentious) or to `/views/*` (plain aggregate). Styling is utility-first Tailwind; scoped `<style>` only for canvas/keyframes. Charts are raw Chart.js on `<canvas>`. `lib/api.js` `request()` throws `ApiError` with the server's `detail`; components bind `err.message` to banners. State lives in `lib/stores.js`; `lib/live.js` listens on `/ws/finance` and bumps `ledgerVersion`. Keep components flat; avoid deep prop drilling.

### 🔒 Zero-knowledge crypto contract
- **Key:** PBKDF2 (100,000 iterations, SHA-256, static salt `"jizifin-salt-pbkdf2"`) → 256-bit AES-GCM key.
- **IV:** static 12 bytes `"jizifin-cryp"`; ciphertext is Base64URL without padding (`crypto.js`). Deterministic by design.
- **Login proof:** `encryptText("FinanceTrackerAuth")`. The server stores only `SHA-256(proof)`; it never sees the passphrase.
- The server holds ciphertext in `*_ct` columns and plaintext numbers/dates/ids/flags.
- **On `*_ct` columns only:** equality, `IN`, joins and `GROUP BY`. **Forbidden:** range filters, `LIKE`, collation `ORDER BY` (sort in the browser).
- Backups (`GET/POST /backup`) are a DuckDB file of ciphertext and numbers; sessions are excluded.

---

## 🗄️ 3. SCHEMA (`backend/crates/jizifin-store/migrations/0001_init.sql`)

Surrogate `BIGINT` keys from sequences. **No foreign keys, by decision** (DuckDB blocks updating a UNIQUE column of a referenced parent); repositories enforce integrity with `require`/`unreferenced`. Deleting a category is refused with 409 while entries or recurring templates use it; its agreements and budgets cascade with it, by design. One unified `entries` ledger; SCD2 agreements and budgets; no stored contentious values except `month_seals`. Bump `LATEST_VERSION` and add a numbered SQL file for every schema change.

| Table | Key columns (`*_ct` = ciphertext) |
| :--- | :--- |
| `app_config`, `sessions`, `schema_version` | auth proof hash; token hashes (7-day idle TTL); migration level |
| `members` | `id`, `name_ct` UNIQUE, `color`, `is_active` |
| `categories` | `id`, `kind` expense/income, `name_ct`, `pay_mode` shared/personal/income_ratio |
| `agreements`, `agreement_shares` | per-category SCD2: `valid_from`, `valid_to` NULL = open-ended; `pct` DOUBLE |
| `accounts`, `account_members`, `account_categories` | joint accounts: `safety_margin_pct`, `expected_total_cents`, `deposit_mode`, `due_day`, `fixed_deposit_cents`, `expected_cents` |
| `projects`, `project_members`, `tags` | targets and event windows |
| `recurring_templates` | `kind`, `amount_cents`, `day_of_month`, `recurrence`, `valid_from/to`, `is_active` |
| `entries` | `kind` expense/income/deposit/correction, `entry_date`, signed `amount_cents`, `member_id`, `category_id`, `account_id`, `project_id`, `tag_id`, `recurring_id`, `name_ct`, `note_ct`; UNIQUE `(recurring_id, entry_date)` |
| `entry_share_overrides` | per-entry custom split |
| `jobs`, `income_overrides` | contracts (monthly/weekly/biweekly/annual) and month-specific amounts |
| `budgets` | SCD2 limits, `valid_to` NULL = standing |
| `offset_rules` | debit/credit member+category pairs |
| `month_seals` | `month`, `result_json`, `inputs_hash`, `sealed_at` |

Indexes: `entries(entry_date, id)`, `(member_id, entry_date)`, `(category_id, entry_date)`, `(account_id, entry_date)`, `(project_id)`. Keep range filters SARGable (compare `entry_date` to bounds, never wrap it in a function).

---

## 📂 4. REPO TOPOLOGY

```
backend/   Cargo workspace (crates above), Dockerfile, Dockerfile.test, clippy.toml, .cargo/config.toml
frontend/  src/lib (api, stores, live, format, crypto, csv, *.svelte), src/lib/docs, src/test (Vitest), Dockerfile.prod (builds WASM)
docs/      RUST_MIGRATION_PLAN.md, getting-started.md
scripts/   capture-legacy-reference.sh (one-off), run-tests-and-sonar.sh, get-sonar-report.sh
.agents/   rules/ (always-on), skills/ (jizifin-backend-developer, jizifin-auditor, jizifin-query-optimizer); .claude/skills links here
docker-compose.yml, Caddyfile, Makefile
data/      DuckDB file (git-ignored, bind-mounted at /data)
```

---

## 🚨 5. VERIFICATION POLICY

- After changing Rust: `make lint-backend`. After changing the frontend: `make wasm` then the production build above.
- **Test status:** the Rust test suite is not written yet (plan §12, "Phase B"). The frontend Vitest suite still contains suites for modules that no longer exist (plan §12.6 lists them for deletion). Do not treat a green or red run of those as evidence either way.
- Before declaring behaviour correct, check it against a real run: the API on a migrated scratch database, or the live host.
- Documentation synchronisation: when schema, endpoints or workflows change, update this file, `README.md` and `docs/RUST_MIGRATION_PLAN.md` together.

## 🛡️ 6. SANDBOX RULES

If a command hits a sandbox boundary, pivot to Docker. Keep all commands and edits inside the project root. Prefer absolute paths over `cd`.
