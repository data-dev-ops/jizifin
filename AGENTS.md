# 🤖 SYSTEM CAPROM: FinanceTracker LLM Agent Directives

**TARGET:** Gemini Pro / Advanced LLM Agent  
**CONTEXT:** Monorepo Personal Finance Tracker (Multi-user Household)  
**PRIME DIRECTIVE:** Strictly adhere to the technical stack, execution paths, and database schemas defined below. Prioritize zero-regression, ANSI-compliant SQL, flat Svelte component design, and deterministic cryptographic integrity.

---

## ⚙️ 1. MULTI-SYSTEM ENVIRONMENT & EXECUTION STEERING

Agents work across varied host systems, sandboxes, and CI environments. Always use this prioritized execution strategy:

### Tooling & Execution Priority
1. **Primary Host CLI**: Use local tools when available on `PATH` and permitted by environment policies:
   - Python / UV: `uv run --directory backend ...`
   - Node / NPM: `npm --prefix frontend ...`
2. **NVM / Custom Node Fallback**: If `npm` or `node` is missing from the non-interactive subshell `PATH`, resolve Node via installed NVM paths (e.g., `~/.nvm/versions/node/$(ls ~/.nvm/versions/node 2>/dev/null | tail -1)/bin/npm`).
3. **Docker Container Fallback (Clean Host / Sandbox Bound)**: If host tools are unavailable or commands hit sandbox permission deny rules, execute test and build commands inside the pre-built Docker containers:
   - Backend Container: `jizifin-backend-test`
   - Frontend Container: `jizifin-frontend-test`
4. **Cluster Orchestration**: Multi-container stack (`backend`, `frontend`, `caddy`, `sonarqube`) runs via `docker compose`. Ignore `sonarqube` unless requested

### Execution Commands Reference

| Task | Host CLI Command | Docker Container Fallback |
| :--- | :--- | :--- |
| **Backend Tests (Full)** | `uv run --directory backend pytest` | `docker run --rm -v $(pwd)/backend/app:/app/app -v $(pwd)/backend/tests:/app/tests jizifin-backend-test pytest` |
| **Backend Integration Scenarios** | `uv run --directory backend pytest tests/test_scenarios_integration.py` | `docker run --rm -v $(pwd)/backend/app:/app/app -v $(pwd)/backend/tests:/app/tests jizifin-backend-test pytest tests/test_scenarios_integration.py` |
| **Backend Dev Server** | `uv run --directory backend uvicorn app.main:app --reload --port 8000` | `docker compose up backend` |
| **Frontend Tests (Full)** | `npm --prefix frontend test -- --run` | `docker run --rm -v $(pwd)/frontend/src:/app/src -v $(pwd)/frontend/index.html:/app/index.html -v $(pwd)/frontend/tailwind.config.js:/app/tailwind.config.js -v $(pwd)/frontend/vite.config.js:/app/vite.config.js jizifin-frontend-test npm test -- --run` |
| **Frontend Dev Server** | `npm --prefix frontend run dev` | `docker compose up frontend` |
| **Full Stack Cluster** | `docker compose up --build -d` | `docker compose up -d backend frontend caddy` (production) |
| **Sonar & Full Coverage** | `./scripts/run-tests-and-sonar.sh` | Local SonarQube on `http://localhost:9000` |

---

## 🏗️ 2. ARCHITECTURAL BOUNDARIES & CRYPTOGRAPHIC DESIGN

### 🖥️ Backend: FastAPI / Python 3.14 / SQLite
- **Validation:** Strict Pydantic v2 schemas for all requests, responses, and analytics.
- **Database Driver:** `aiosqlite` with WAL mode (`PRAGMA journal_mode=WAL;`) and foreign keys enforced (`PRAGMA foreign_keys=ON;`).
- **Querying:** No ORMs. All endpoints write native, optimized, ANSI-compliant SQL directly.
- **Currency & Dates:** Currency is represented strictly as `INTEGER` cents at the database layer (decimals `cents / 100.0` exposed only at presentation layer). Dates are `TEXT` formatted as `YYYY-MM-DD`.
- **Realtime Broadcast:** Native `fastapi.WebSocket` fan-out broadcasting live ledger events on `/ws/finance`.

### 🌐 Frontend: Vanilla Svelte / Tailwind CSS / Chart.js
- **State Management:** Svelte writable stores (`stores.js`) serve as the reactive data bridge for local client state.
- **Styling:** Exclusively utility-first Tailwind CSS. Scoped `<style>` blocks are prohibited unless strictly necessary (e.g., canvas or keyframes).
- **Visualization:** Raw Chart.js rendered on `<canvas>` elements, reactively updated via `chart.update()`. No heavy component wrappers.
- **Error Handling:** Central API helper `request()` in `frontend/src/lib/api.js` throws explicit `Error` objects on non-2xx status codes; Svelte components catch and bind `err.message` to local reactive error banners.

### 🔒 Client-Server Cryptographic Split (Zero-Knowledge Privacy)
To maintain zero-knowledge privacy for household finances, sensitive data is encrypted before leaving the client:

1. **Client-Side Cryptography (`crypto.js`)**:
   - **Key Derivation:** Derives a 256-bit AES-GCM `CryptoKey` from the user passphrase using PBKDF2 (100,000 iterations, SHA-256, static salt `"jizifin-salt-pbkdf2"`).
   - **Static IV:** AES-GCM uses static 12-byte IV `[106, 105, 122, 105, 102, 105, 110, 45, 99, 114, 121, 112]` (`"jizifin-cryp"`). Ciphertext is encoded to Base64URL (no padding).
   - **Encryption/Decryption:** Payload fields are encrypted via `encryptText` before POST/PUT and decrypted via `decryptText` before updating stores.
2. **Server-Side Cryptography (`crypto_utils.py`)**:
   - Manages bulk database export (`/auth/export`) and import (`/auth/import`). Uses the Python `cryptography` library with identical PBKDF2 salt and static IV.
   - Decrypts database copies in-place on the server filesystem temporarily during export streaming, and re-encrypts imported databases before replacing active storage. Plaintext is never persisted on disk.
3. **Deterministic AES-GCM Querying Rules**:
   - **Valid on Encrypted Columns**: Exact matches (`col = ?`, `IN (...)`), equality joins (`ON a.col = b.col`), `GROUP BY`, and foreign key cascades.
   - **FORBIDDEN on Encrypted Columns**: Range filters (`<`, `>`, `BETWEEN`), `LIKE` wildcards, and text collation `ORDER BY`.
   - **Plaintext Columns**: Amounts (`cost_cents`, `amount_cents`), dates (`expense_date`, `start_date`), IDs, flags (`is_joint`, `is_active`), and `settlements`.

```
┌────────────────────────────────────────────────────────────────────────┐
│                        COLUMN ENCRYPTION MATRIX                        │
├───────────────────────────────────┬────────────────────────────────────┤
│ ENCRYPTED (Base64URL)             │ PLAINTEXT                          │
├───────────────────────────────────┼────────────────────────────────────┤
│ users.name                        │ All *_cents (cost, amount, target) │
│ splits.category                   │ All dates (YYYY-MM-DD, month)      │
│ income_categories.category        │ All integer IDs and foreign keys   │
│ projects.name                     │ Boolean flags (is_joint, is_active)│
│ expenses.name, who_paid, category │ colors (#hex)                      │
│ income.name, who, category        │ settlements table                  │
│ recurring_expenses.name, who, cat │ day_of_month, frequencies          │
│ jobs.name, who, notes             │ percentages (pct, REAL)            │
│ split_allocations.category, user  │ safety_margin_pct                  │
│ tags.name, description            │ deposit_split_mode                 │
│ joint_accounts.name               │ magic_word (app_config)            │
│ joint_account_deposits.user_name  │                                    │
│ joint_account_corrections.note    │                                    │
│ salary_overrides.user_name, note  │                                    │
└───────────────────────────────────┴────────────────────────────────────┘
```

---

## 🌟 3. COMPREHENSIVE FEATURE OVERVIEW

The application provides the following core capabilities:

1. **Multi-User & Co-Housing Support**:
   - Houses multiple members and independent couples under one instance.
   - Dynamic user configuration with custom colors and active/inactive status.
   - Connected-component graph isolation ensures debt settlements between couples do not cross over into unrelated household tenants.
2. **Multi-Joint Accounts**:
   - Multiple isolated joint accounts (`joint_accounts`, `joint_account_members`) with independent balances, safety margins, and category bindings.
   - Expenses and recurring commitments can be charged directly to a specific joint account (`is_joint = 1`, `joint_account_id`).
   - Per-account monthly deposit schedules (`joint_account_deposits`), monthly execution tracking (`joint_account_monthly_deposits`), and signed balance corrections (`joint_account_corrections`).
   - Direct joint expenses are automatically excluded from peer-to-peer payback debt calculations.
3. **Category Management & SCD2 Split Agreements**:
   - Dynamic category registry with renaming cascades across all referencing tables (`ON UPDATE CASCADE`) and delete protection (blocked with HTTP 409 if referenced).
   - Slowly Changing Dimensions Type 2 (SCD2) category split agreements (`split_agreements`, `split_allocations`) supporting date-bounded overrides. Overrides take precedence over baseline agreements during their active window.
4. **Jobs & Period-Specific Salary Adjustments**:
   - Employment contracts (`jobs`) tracking regular income streams with normalized frequencies (`monthly`, `weekly`, `biweekly`, `annual`) and start/end dates.
   - Month-specific salary overrides (`salary_overrides`) for temporary unpaid leave, sickness, or overtime, dynamically adjusting proportional split ratios for target periods without mutating underlying contracts.
5. **Project Multi-User Membership & Settlement**:
   - Target budget goals (`projects`) with user membership tracking (`project_users`).
   - Dedicated point-in-time participant equity balance sheets (`GET /projects/{id}/settlement`), evaluating effective funding (direct personal + joint proportion) against assigned liability.
6. **Dynamic Tag Timelines & Boundary Enforcement**:
   - Event and label tags (`tags`) with active date windows (`start_date`, `end_date`).
   - Strict timeline window validation on expense creation/update (`HTTP 422` if expense falls outside tag bounds).
   - Rejection of tag timeline shrinkage if existing assigned expenses would be orphaned outside proposed dates.
7. **Core Ledger & Recurring Commitments**:
   - Expense tracking with smart form memory, search & filtering, inline tag assignment, and comprehensive edit modals.
   - Automated recurring expense templates (`recurring_expenses`) with day-of-month and frequency scheduling.
   - Month-locking settlements (`settlements`) to seal historical periods.
8. **7-Domain Personalization & Settings**:
   - Independent Desktop vs Mobile display profiles saved in `localStorage`.
   - 1-Click functional presets: `Streamlined`, `High Legibility`, `Standard / Balanced`, and `Detailed / Power User`.
   - Public stealth privacy shield (frosted-glass blur with hover peek).
   - Custom currency symbol presets (€, $, £, CHF, ¥, kr), whole-unit rounding, and configurable row density (`minimal`, `compact`, `detailed`).
9. **Interactive Documentation Hub (DocsHub)**:
   - In-app technical and user documentation suite (`DocsHub.svelte`, `BackendDocs.svelte`, `FrontendDocs.svelte`, `GettingStartedDocs.svelte`, `SystemDocs.svelte`) backed by repository markdown guides.
10. **Database Migration Resilience**:
    - Automatic in-memory/import schema migration via `ensure_column` and dynamic table updaters, guaranteeing legacy backups import cleanly with backfilled defaults.

---

## 🗄️ 4. DATABASE SCHEMA & QUERY OPTIMIZATION

Authoritative schema definitions, migrations, and view initializations reside in `backend/app/database.py`.

### Database Table Catalog (24 Tables)

| Table | Purpose | Primary Key | Key Foreign Keys | Encrypted Columns |
| :--- | :--- | :--- | :--- | :--- |
| `app_config` | App key-value configuration | `key` | None | `value` (magic_word) |
| `users` | Household members | `name` | None | `name` |
| `splits` | Expense category registry | `category` | None | `category` |
| `income_categories` | Income category registry | `category` | None (historical survival) | `category` |
| `projects` | Budget milestone targets | `id` (AUTO) | None | `name` |
| `project_users` | Multi-user project members | `(project_id, user_name)` | `projects(id)`, `users(name)` | `user_name` |
| `tags` | Event & label tags with timeline | `id` (AUTO) | None | `name`, `description` |
| `expenses` | Core expense ledger | `id` (AUTO) | `users(name)`, `splits(category)`, `projects(id)`, `tags(id)`, `joint_accounts(id)` | `name`, `who_paid`, `category` |
| `expense_overrides` | Per-expense custom split overrides | `(expense_id, user_name)` | `expenses(id)`, `users(name)` | `user_name` |
| `income` | Append-only income ledger | `id` (AUTO) | `users(name)` | `name`, `who`, `category` |
| `jobs` | Employment contracts & regular income | `id` (AUTO) | `users(name)` | `name`, `who`, `notes` |
| `salary_overrides` | Month-specific salary adjustments | `(user_name, month)` | `users(name)` | `user_name`, `note` |
| `recurring_expenses` | Automated expense templates | `id` (AUTO) | `users(name)`, `splits(category)`, `joint_accounts(id)` | `name`, `who_paid`, `category` |
| `budgets` | Monthly category spending limits | `(category, month)` | `splits(category)` | `category` |
| `settlements` | Monthly reconciliation & lock log | `month` | None | Plaintext |
| `split_agreements` | SCD2 category split timelines | `id` (AUTO) | `splits(category)` | `category`, `note` |
| `split_allocations` | User percentage shares per agreement | `id` (AUTO) | `split_agreements(id)`, `splits(category)`, `users(name)` | `category`, `user_name` |
| `joint_accounts` | Multi-joint account registry | `id` (AUTO) | None | `name` |
| `joint_account_members`| Joint account participant roster | `(account_id, user_name)` | `joint_accounts(id)`, `users(name)` | `user_name` |
| `joint_account` | Singleton backwards-compat table | `id = 1` | None | `name` |
| `joint_account_categories` | Categories paid from joint accounts | `(category, account_id)` | `splits(category)`, `joint_accounts(id)` | `category` |
| `joint_account_deposits` | Monthly user deposit schedules | `(user_name, account_id)` | `users(name)`, `joint_accounts(id)` | `user_name` |
| `joint_account_monthly_deposits` | Deposit execution & payment log | `(month, user_name, account_id)` | `users(name)`, `joint_accounts(id)` | `user_name` |
| `joint_account_expected_costs` | Expected monthly cost per category | `(category, account_id)` | `splits(category)`, `joint_accounts(id)` | `category` |
| `joint_account_corrections` | Signed balance corrections | `id` (AUTO) | `joint_accounts(id)` | `note` |

### Database Views Catalog (9 Read-Only Views)
Recreated on application startup to guarantee schema alignment:

1. `view_monthly_total`: Current month's aggregated spending total (`cost_cents / 100.0`) and transaction count.
2. `view_monthly_by_category`: Current month spending grouped by category.
3. `view_monthly_by_payer`: Current month spending grouped by `who_paid`.
4. `view_expenses_by_month_category`: Monthly spending grouped by `(YYYY-MM, category)`.
5. `view_project_summary`: Target vs aggregated total spent cents per project.
6. `view_tag_totals`: All-time spending aggregates, transaction counts, and active date bounds per tag.
7. `view_joint_account_monthly`: Joint account monthly spending grouped by `(month, category, account_id)`.
8. `view_current_split_allocations`: Active category split allocations resolved from SCD2 agreements for current date.
9. `view_split_agreements_active`: Active category split agreement timelines ordered by start date.

### Query Performance & Execution Plan (EQP) Optimization
The `jizifin-query-optimizer` skill defines indexing standards to guarantee high throughput and zero table scans:
- **Canonical Indexes**:
  - `idx_expenses_date_id` on `expenses (expense_date DESC, id DESC)`
  - `idx_expenses_who_date` on `expenses (who_paid, expense_date DESC)`
  - `idx_expenses_category_date` on `expenses (category, expense_date DESC)`
  - `idx_expenses_project_id` on `expenses (project_id) WHERE project_id IS NOT NULL`
  - `idx_expenses_tag_id` on `expenses (tag_id) WHERE tag_id IS NOT NULL`
  - `idx_expenses_joint_date` on `expenses (is_joint, expense_date DESC)`
  - `idx_income_who_date` on `income (who, income_date DESC)`
  - `idx_jobs_who_dates` on `jobs (who, start_date, end_date)`
  - `idx_split_agreements_cat_dates` on `split_agreements (category, start_date, end_date)`
  - `idx_split_allocations_agreement` on `split_allocations (agreement_id)`
  - `idx_split_allocations_category` on `split_allocations (category)`
- **SARGability Rule**: Avoid calling SQL functions on index columns (e.g. use `WHERE expense_date >= 'YYYY-MM-01' AND expense_date <= 'YYYY-MM-31'` instead of `WHERE strftime('%Y-%m', expense_date) = ...`).

---

## 🧮 5. COMPLEX DOMAIN LOGIC & INVARIANTS

### 1. Signed Hare-Niemeyer / Largest Remainder Distribution (`allocate_cents_largest_remainder`)
Computes exact integer cent allocations for positive expenses, zero values, and negative credit memos/refunds:
1. Calculates exact float cent shares: `share = total_cents * pct / total_pct`.
2. Computes base cent shares using mathematical floor: `floor_share = math.floor(share)` (essential for correct negative quotient assignments on refunds).
3. Evaluates fractional remainder: `remainder = share - floor_share`.
4. Distributes remaining cents (`total_cents - sum(floor_shares)`) to users in descending order of remainder.
5. **Deterministic Salted Tie-Breaker**: For users with identical fractional remainders, tie-breaking order is determined by SHA-256 hash of `f"{tx_salt}:{user}"` (using expense ID, date, or category), eliminating alphabetical drift across hundreds of transactions.

### 2. Payback Calculation & Graph Decomposition (`/analytics/paybacks`)
1. **Joint Account Exclusion**: Direct joint account transactions (`is_joint = 1`) and categories assigned to joint accounts are excluded from peer reimbursement calculations.
2. **Allocation Resolution**: Evaluates per-expense overrides (`expense_overrides`), then active SCD2 split agreements (`split_agreements`), falling back to baseline splits or equal distribution.
3. **Personal-Pay Categories**: Categories designated for personal spend (`PERSONAL COST`, `LEISURE`, `GIFT`) are dynamically remapped to the payer with 100% liability.
4. **Special Deduction Rule**: Evaluates the smaller of Jane's "Combined Fixed" and John's "Apartment" payments, adjusting net positions to simulate direct reimbursement.
5. **Connected-Component Graph Isolation**: Partitions members into disjoint connected subgraphs based on transaction splits before executing greedy debt simplification, ensuring debts do not cross outside participating groups.

### 3. Tag Active Window Validation (`POST /expenses`, `PUT /expenses`, `PUT /tags/{id}`)
- Validates that `tag.start_date <= expense.expense_date <= tag.end_date`. Violations return `HTTP 422 Unprocessable Content`.
- Updating a tag timeline validates all currently assigned expenses; if any expense would fall outside the proposed window, the update is rejected with `HTTP 422`.

### 4. Jobs & Salary Override Analytics (`/income/latest-salary`, `/analytics/income-by-person`)
- Evaluates active contracts in `jobs` during target month `YYYY-MM` (`start_date <= '{month}-31'` and `end_date IS NULL OR end_date >= '{month}-01'`).
- Normalizes frequencies: `monthly` (`cents`), `weekly` (`round(cents * 52 / 12)`), `biweekly` (`round(cents * 26 / 12)`), `annual` (`round(cents / 12)`).
- Month-specific overrides in `salary_overrides` take precedence over contract amounts for that specific month.
- Falls back to legacy historical `SALARY` append entries only if user has no configured jobs.

---

## 📂 6. REPO TOPOLOGY

```
jizifin/
├── backend/
│   ├── app/
│   │   ├── main.py            # FastAPI endpoints, lifespan, WebSockets, analytics
│   │   ├── database.py        # Schema DDL, migrations, views, connection pool
│   │   ├── models.py          # Pydantic v2 validation models
│   │   └── crypto_utils.py    # Server-side PBKDF2/AES-GCM backup streaming
│   ├── docs/                  # Backend technical documentation markdown
│   ├── tests/                 # Pytest test suite (339 tests, 20 test files)
│   ├── pyproject.toml         # Python project configuration and dependencies
│   └── Dockerfile             # Python 3.14 container definition
├── frontend/
│   ├── src/
│   │   ├── lib/
│   │   │   ├── api.js         # Central client API with automatic AES-GCM encryption
│   │   │   ├── crypto.js      # WebCrypto PBKDF2 and AES-GCM static IV routines
│   │   │   ├── stores.js      # Reactive Svelte writable stores
│   │   │   ├── docs/          # In-app interactive documentation components
│   │   │   └── *.svelte       # Feature tab and management components
│   │   ├── App.svelte         # Main application shell, tabs, privacy shield
│   │   └── main.js            # DOM mount point
│   ├── docs/                  # Frontend technical documentation markdown
│   ├── src/test/              # Vitest test suite (343 tests, 39 test files)
│   ├── package.json           # Node scripts and dependencies
│   └── Dockerfile             # Vite / Node container definition
├── docs/                      # General and getting-started documentation
├── scripts/                   # Test automation and SonarQube runner scripts
├── docker-compose.yml         # Local and production multi-container orchestration
└── Caddyfile                  # Reverse proxy, TLS termination, and WebSocket routing
```

---

## 🚨 7. ZERO-REGRESSION POLICY & MANDATORY VERIFICATION

### Critical Invariant: Zero Regression Verification
Any code modification, schema adjustment, feature addition, or refactor **must verify 100% test passage across both backend and frontend suites**:

- **Backend Test Suite (339 Tests across 20 test files)**:
  - *Host CLI*: `uv run --directory backend pytest`
  - *Docker Fallback*: `docker run --rm -v $(pwd)/backend/app:/app/app -v $(pwd)/backend/tests:/app/tests jizifin-backend-test pytest`
- **Frontend Test Suite (343 Tests across 39 test files)**:
  - *Host CLI*: `npm --prefix frontend test -- --run`
  - *Docker Fallback*: `docker run --rm -v $(pwd)/frontend/src:/app/src -v $(pwd)/frontend/index.html:/app/index.html -v $(pwd)/frontend/tailwind.config.js:/app/tailwind.config.js -v $(pwd)/frontend/vite.config.js:/app/vite.config.js jizifin-frontend-test npm test -- --run`

### Mathematical & Architectural Code Generation Rules
1. **Never Truncate Floating-Point Percentages**: Keep basis-point precision (`AllocationEntry.pct` float, `toFixed(4)`). Never coerce to integer with `int()` or `Math.round()`.
2. **Integer Cent Database Boundary**: Stored values are always `INTEGER` cents. Currency formatting (`cents / 100.0`) occurs exclusively at presentation/response boundaries.
3. **Signed Math Floor**: Always use `math.floor()` in Largest Remainder distribution to guarantee zero-sum invariants on negative refunds.
4. **Flat Component Composition**: Keep Svelte components modular and avoid deep hierarchical prop drilling.
5. **Documentation Synchronization**: When schema, indexes, endpoints, or workflows change, update `AGENTS.md` and `README.md` in tandem.

---

## 🛡️ 8. PERMISSION HANDLING & SANDBOX RECOVERY PROTOCOL

1. **System Boundary Recognition**: If a terminal command returns a sandbox deny or protection boundary error, pivot immediately to workspace bounds or Docker container execution.
2. **No `cd` Commands**: Never execute `cd` commands in `run_command`. Always specify the working directory via the `Cwd` parameter.
3. **Workspace Enclosure**: All commands, scripts, and file modifications must remain strictly confined to the project root.
