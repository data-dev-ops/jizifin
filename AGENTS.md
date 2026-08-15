# 🤖 SYSTEM CAPROM: FinanceTracker LLM Agent Directives

**TARGET:** Gemini Pro / Advanced LLM Agent
**CONTEXT:** Monorepo Personal Finance Tracker (Multi-user Household)
**PRIME DIRECTIVE:** Strictly adhere to the technical stack, execution paths, and database schemas defined below. Prioritize zero-regression, ANSI-compliant SQL, and flat Svelte component design.

---

## ⚙️ 1. ENVIRONMENT & EXECUTION
Standard tooling paths and execution commands for development, testing, and cluster orchestration:

- **Python Runtime:** Python 3.14+
- **Dependency Manager:** `uv` (uv 0.11+ or compatible)
- **Node Runtime:** Node.js (v20+ or v24+)
- **NPM:** `npm`

**Execution Commands:**
- Backend Dev Server: `uv run --directory backend uvicorn app.main:app --reload --port 8000`
- Add Python Package: `uv add --directory backend <package>`
- Backend Test Suite & Coverage: `uv run --directory backend pytest --cov=app --cov-report=xml:coverage.xml --cov-report=term`
- Frontend Dev Server: `npm --prefix frontend run dev`
- Frontend Install: `npm --prefix frontend install <package>`
- Frontend Test Suite: `npm --prefix frontend test`
- Frontend Test Coverage: `npm --prefix frontend run test:coverage`
- Full-Stack Coverage & Sonar: `./scripts/run-tests-and-sonar.sh`
- **Docker Compose Cluster:** Orchestrates `backend`, `frontend`, `caddy`, and local `sonarqube` containers via `docker-compose.yml`. Run `docker compose up --build -d` from the root to start the full stack. Production deployment explicitly starts `backend frontend caddy` only.

---

## 🏗️ 2. ARCHITECTURAL BOUNDARIES & CRYPTOGRAPHIC DESIGN

### 🖥️ Backend: FastAPI / Python 3.14 / SQLite
- **Validation:** Strict Pydantic v2 schemas for all requests, responses, and analytics objects.
- **Database Connection:** Driven by `aiosqlite`. Every connection is initialized with WAL mode (`PRAGMA journal_mode=WAL;`) and foreign keys enabled (`PRAGMA foreign_keys=ON;`).
- **Querying:** No ORMs. All endpoints write native, optimized, ANSI-compliant SQL directly in their logic.
- **Data Types:** Currency is represented exclusively as `INTEGER` cents at the database layer. Decimals (cents/100.0) are calculated and exposed only at the presentation and response layers. Dates are formatted as `TEXT` (YYYY-MM-DD).
- **Realtime Ticketing:** Native `fastapi.WebSocket` implementation. Fan-out broadcast pattern handles `expense_created` notifications to keep connected clients updated in real time.

### 🌐 Frontend: Vanilla Svelte / Tailwind CSS / Chart.js
- **State Management:** Svelte writable stores (`stores.js`) serve as the reactive data bridge for local client state.
- **Styling:** Exclusively utility-first Tailwind CSS. Scoped `<style>` blocks are prohibited unless strictly necessary (e.g., canvas or keyframes that cannot be handled via standard Tailwind classes).
- **Visualization:** Raw Chart.js rendered on `<canvas>` elements. Updates are triggered reactively via `chart.update()` inside WebSocket ticker payloads. No external heavy wrappers.

### 🔒 Client-Server Cryptographic Split
To maintain zero-knowledge privacy for the household financial history, data is encrypted before sending it to the server. The cryptographic tasks are split between client and server as follows:

1. **Client-Side Cryptography (`crypto.js`)**:
   - **Key Derivation:** Derives a 256-bit AES-GCM `CryptoKey` from the user's master passphrase using the browser's Web Crypto API with PBKDF2, 100,000 iterations, SHA-256, and a static salt `"jizifin-salt-pbkdf2"`.
   - **Encryption/Decryption:** Encrypts sensitive fields (using `encryptText`) before dispatching POST/PUT payloads, converting the binary ciphertext to a Base64URL string (stripping padding). Decrypts received data (using `decryptText`) before updating Svelte stores.
   - **Static IV:** AES-GCM encryption uses a static 12-byte IV `[106, 105, 122, 105, 102, 105, 110, 45, 99, 114, 121, 112]` (equivalent to `"jizifin-cryp"`).

2. **Server-Side Cryptography (`crypto_utils.py`)**:
   - **Database Backups:** Serves bulk database export (`/auth/export`) and import (`/auth/import`) endpoints. Using the Python `cryptography` library, it derives the key using the exact same PBKDF2 parameters and salt.
   - **Bulk Processing:** For exports, it takes a copy of the database and decrypts sensitive columns in-place on the filesystem temporarily before streaming it to the user. For imports, it encrypts the uploaded plaintext database in-place on the server before replacing the active database file. The database is never kept in plaintext on the server's persistent disk.

3. **Deterministic AES-GCM Implications**:
   - **Queryability & Referential Integrity:** Because the encryption is deterministic (static IV), the exact same plaintext string always encrypts to the exact same ciphertext Base64URL string. This allows the backend to perform exact matches (`who_paid = ?`), enforce `PRIMARY KEY` uniqueness (e.g. `splits.category`), group records (`GROUP BY category`), and validate foreign keys (e.g. `expenses.who_paid` matching `users.name`).
   - **Security Weakness:** The use of a static IV breaks the semantic security of AES-GCM. It exposes the ciphertexts to frequency analysis and XOR pattern/replay leakages if an attacker obtains the database file.
   - **Encrypted Columns:** `users.name`, `splits.category`, `income_categories.category`, `projects.name`, `expenses.name`, `expenses.who_paid`, `expenses.category`, `expense_overrides.user_name`, `income.name`, `income.who`, `income.category`, `recurring_expenses.name`, `recurring_expenses.who_paid`, `recurring_expenses.category`, `budgets.category`, `split_allocations.category`, `split_allocations.user_name`, `tags.name`, `tags.description`, `joint_account.name`, `joint_account_deposits.user_name`, `joint_account_corrections.note`, `jobs.name`, `jobs.who`, `jobs.notes`.
   - **Plaintext Columns:** Numeric amounts (cents), dates, integer primary/foreign keys, and the `settlements` table.

### 🚨 Coding Style Conventions & Deviations
- **API Error Handling Flow**:
  - `frontend/src/lib/api.js` utilizes a central `request()` helper that throws an explicit `Error` object on non-2xx HTTP response codes (propagating status and response body).
  - Svelte components (such as `Login.svelte`, `ExpenseForm.svelte`, or `IncomeTab.svelte`) call api methods inside `try...catch` blocks and assign `err.message` to local reactive error variables (e.g., `formError`, `jobError`) to render alert blocks in the user interface.

---

## 🗄️ 3. DATABASE SCHEMA & LOGIC CONSTRAINTS

### Database Tables (SQLite v4 Schema)
All database interactions are defined in `backend/app/database.py`. The tables are:

1. **`app_config`** (Key-value store for app-wide settings)
   - `key` (TEXT PRIMARY KEY)
   - `value` (TEXT NOT NULL) — stores the encrypted magic word `magic_word` to validate passphrases.

2. **`users`** (Household members)
   - `name` (TEXT PRIMARY KEY, CHECK(length(name) <= 256)) — Encrypted.
   - `color` (TEXT NOT NULL DEFAULT '#6366f1')
   - `is_active` (INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0, 1)))
   - `created_at` (TEXT NOT NULL DEFAULT (datetime('now')))

3. **`splits`** (Category registry)
   - `category` (TEXT PRIMARY KEY, CHECK(length(category) <= 256)) — Encrypted.

4. **`income_categories`** (Income category registry)
   - `category` (TEXT PRIMARY KEY, CHECK(length(category) <= 256)) — Encrypted.
   - No FK from `income.category` — historical entries survive category deletion intentionally.

5. **`projects`** (Target budget goals)
   - `id` (INTEGER PRIMARY KEY AUTOINCREMENT)
   - `name` (TEXT NOT NULL UNIQUE CHECK(length(name) <= 256)) — Encrypted.
   - `target_cents` (INTEGER NOT NULL CHECK(target_cents > 0))
   - `target_date` (TEXT NOT NULL CHECK(target_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'))

6. **`tags`** (Open-ended label tags)
   - `id` (INTEGER PRIMARY KEY AUTOINCREMENT)
   - `name` (TEXT NOT NULL UNIQUE CHECK(length(name) <= 256)) — Encrypted.
   - `color` (TEXT NOT NULL DEFAULT '#f59e0b')
   - `description` (TEXT CHECK(length(description) <= 512)) — Encrypted.
   - `created_at` (TEXT NOT NULL DEFAULT (datetime('now')))
   - `is_joint` (INTEGER NOT NULL DEFAULT 0 CHECK(is_joint IN (0, 1)))
   - `is_active` (INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0, 1)))

7. **`expenses`** (Core expense ledger)
   - `id` (INTEGER PRIMARY KEY AUTOINCREMENT)
   - `name` (TEXT NOT NULL CHECK(length(name) <= 256)) — Encrypted.
   - `cost_cents` (INTEGER NOT NULL CHECK(cost_cents > 0))
   - `expense_date` (TEXT NOT NULL CHECK(expense_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'))
   - `who_paid` (TEXT NOT NULL REFERENCES users(name) ON UPDATE CASCADE) — Encrypted.
   - `category` (TEXT NOT NULL REFERENCES splits(category) ON UPDATE CASCADE) — Encrypted.
   - `project_id` (INTEGER REFERENCES projects(id) ON DELETE SET NULL)
   - `tag_id` (INTEGER REFERENCES tags(id) ON DELETE SET NULL)
   - `is_joint` (INTEGER NOT NULL DEFAULT 0 CHECK(is_joint IN (0, 1))) — 1 if paid directly by joint account

8. **`expense_overrides`** (Per-expense override split allocations)
   - `expense_id` (INTEGER NOT NULL REFERENCES expenses(id) ON DELETE CASCADE)
   - `user_name` (TEXT NOT NULL REFERENCES users(name) ON UPDATE CASCADE ON DELETE CASCADE) — Encrypted.
   - `pct` (REAL NOT NULL CHECK(pct >= 0.0 AND pct <= 100.0))
   - *Primary Key*: `(expense_id, user_name)`

9. **`income`** (Append-only ledger for one-off bonuses, gifts, tax returns)
   - `id` (INTEGER PRIMARY KEY AUTOINCREMENT)
   - `name` (TEXT NOT NULL CHECK(length(name) <= 256)) — Encrypted.
   - `amount_cents` (INTEGER NOT NULL CHECK(amount_cents > 0))
   - `who` (TEXT NOT NULL REFERENCES users(name) ON UPDATE CASCADE) — Encrypted.
   - `category` (TEXT NOT NULL CHECK(length(category) <= 256)) — Encrypted.
   - `income_date` (TEXT NOT NULL CHECK(income_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'))
   - `is_joint` (INTEGER NOT NULL DEFAULT 0 CHECK(is_joint IN (0, 1))) — 1 if deposited to joint account
   - *Indexes*: `idx_income_who_date` on `(who, income_date DESC)`

10. **`recurring_expenses`** (Templates for automated expenses)
    - `id` (INTEGER PRIMARY KEY AUTOINCREMENT)
    - `name` (TEXT NOT NULL CHECK(length(name) <= 256)) — Encrypted.
    - `cost_cents` (INTEGER NOT NULL CHECK(cost_cents > 0))
    - `who_paid` (TEXT NOT NULL REFERENCES users(name) ON UPDATE CASCADE) — Encrypted.
    - `category` (TEXT NOT NULL REFERENCES splits(category) ON UPDATE CASCADE) — Encrypted.
    - `day_of_month` (INTEGER NOT NULL CHECK(day_of_month >= 1 AND day_of_month <= 31))
    - `is_joint` (INTEGER NOT NULL DEFAULT 0 CHECK(is_joint IN (0, 1)))

11. **`budgets`** (Monthly limit configuration)
    - `category` (TEXT NOT NULL REFERENCES splits(category) ON UPDATE CASCADE ON DELETE CASCADE) — Encrypted.
    - `month` (TEXT NOT NULL CHECK(month = 'ALL' OR month GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]'))
    - `limit_cents` (INTEGER NOT NULL CHECK(limit_cents >= 0))
    - *Primary Key*: `(category, month)`

12. **`settlements`** (Month locking logs)
    - `month` (TEXT PRIMARY KEY CHECK(month GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]'))
    - `settled_at` (TEXT NOT NULL)
    - `net_balance_transferred_cents` (INTEGER NOT NULL)

13. **`split_allocations`** (Default split allocations)
    - `category` (TEXT NOT NULL REFERENCES splits(category) ON UPDATE CASCADE ON DELETE CASCADE) — Encrypted.
    - `user_name` (TEXT NOT NULL REFERENCES users(name) ON UPDATE CASCADE ON DELETE CASCADE) — Encrypted.
    - `pct` (REAL NOT NULL CHECK(pct >= 0.0 AND pct <= 100.0))
    - *Primary Key*: `(category, user_name)`

14. **`joint_account`** (Singleton joint account config — id always 1)
    - `id` (INTEGER PRIMARY KEY CHECK(id = 1))
    - `name` (TEXT NOT NULL CHECK(length(name) <= 256)) — Encrypted.
    - `balance_cents` (INTEGER NOT NULL DEFAULT 0)
    - `safety_margin_pct` (INTEGER NOT NULL DEFAULT 10 CHECK(0..100))
    - `deposit_split_mode` (TEXT NOT NULL DEFAULT 'even' CHECK IN ('salary','even','manual'))
    - `expected_total_cents` (INTEGER, nullable — overrides per-cat sum when set)

15. **`joint_account_categories`** (Categories paid from joint account)
    - `category` (TEXT PRIMARY KEY REFERENCES splits(category) ON UPDATE CASCADE ON DELETE CASCADE)

16. **`joint_account_deposits`** (Per-user monthly deposit config)
    - `user_name` (TEXT PRIMARY KEY REFERENCES users(name) ON UPDATE CASCADE ON DELETE CASCADE) — Encrypted.
    - `amount_cents` (INTEGER NOT NULL DEFAULT 0 CHECK >= 0)
    - `day_of_month` (INTEGER NOT NULL DEFAULT 1 CHECK 1..31)

17. **`joint_account_expected_costs`** (Per-category expected monthly cost)
    - `category` (TEXT PRIMARY KEY REFERENCES splits(category) ON UPDATE CASCADE ON DELETE CASCADE)
    - `expected_cents` (INTEGER NOT NULL CHECK >= 0)

18. **`joint_account_corrections`** (Signed balance corrections — deposits and withdrawals)
    - `id` (INTEGER PRIMARY KEY AUTOINCREMENT)
    - `amount_cents` (INTEGER NOT NULL — positive = top-up, negative = withdrawal)
    - `correction_date` (TEXT NOT NULL GLOB YYYY-MM-DD)
    - `note` (TEXT CHECK(length <= 512)) — Encrypted, nullable.

19. **`jobs`** (Employment streams, contracts, and regular income timelines)
    - `id` (INTEGER PRIMARY KEY AUTOINCREMENT)
    - `name` (TEXT NOT NULL CHECK(length(name) <= 256)) — Encrypted.
    - `who` (TEXT NOT NULL REFERENCES users(name) ON UPDATE CASCADE) — Encrypted.
    - `amount_cents` (INTEGER NOT NULL CHECK(amount_cents > 0))
    - `frequency` (TEXT NOT NULL DEFAULT 'monthly' CHECK(frequency IN ('monthly', 'weekly', 'biweekly', 'annual')))
    - `start_date` (TEXT NOT NULL CHECK(start_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'))
    - `end_date` (TEXT CHECK(end_date IS NULL OR end_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'))
    - `notes` (TEXT CHECK(notes IS NULL OR length(notes) <= 512)) — Encrypted.
    - `is_active` (INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0, 1)))
    - *Indexes*: `idx_jobs_who_dates` on `(who, start_date DESC)`

### Database Views (Read-Only)
Views are dropped and recreated on startup to reflect any schema modifications:

1. **`view_monthly_total`** (Total month spending)
   ```sql
   CREATE VIEW view_monthly_total AS
   SELECT
       COALESCE(ROUND(SUM(cost_cents) / 100.0, 2), 0.0) AS total_amount,
       COUNT(*)                                           AS expense_count,
       strftime('%Y-%m', 'now')                          AS month
   FROM expenses
   WHERE strftime('%Y-%m', expense_date) = strftime('%Y-%m', 'now')
   ```

2. **`view_monthly_by_category`** (Total month spending grouped by category)
   ```sql
   CREATE VIEW view_monthly_by_category AS
   SELECT
       category,
       ROUND(SUM(cost_cents) / 100.0, 2) AS total_amount,
       COUNT(*)                           AS expense_count
   FROM   expenses
   WHERE  strftime('%Y-%m', expense_date) = strftime('%Y-%m', 'now')
   GROUP  BY category
   ```

3. **`view_expenses_by_month_category`** (Monthly spending grouped by month YYYY-MM and category)
   ```sql
   CREATE VIEW view_expenses_by_month_category AS
   SELECT
       strftime('%Y-%m', expense_date)   AS month,
       category,
       ROUND(SUM(cost_cents) / 100.0, 2) AS total_amount,
       COUNT(*)                           AS expense_count
   FROM   expenses
   GROUP  BY strftime('%Y-%m', expense_date), category
   ```

4. **`view_monthly_by_payer`** (Total month spending grouped by payer)
   ```sql
   CREATE VIEW view_monthly_by_payer AS
   SELECT
       who_paid,
       ROUND(SUM(cost_cents) / 100.0, 2) AS total_amount,
       COUNT(*)                           AS expense_count
   FROM   expenses
   WHERE  strftime('%Y-%m', expense_date) = strftime('%Y-%m', 'now')
   GROUP  BY who_paid
   ```

5. **`view_project_summary`** (Aggregated total spent cents per project)
   ```sql
   CREATE VIEW view_project_summary AS
   SELECT
       p.id,
       p.name,
       p.target_cents,
       p.target_date,
       COALESCE(SUM(e.cost_cents), 0) AS total_spent_cents,
       COUNT(e.id)                     AS expense_count
   FROM projects p
   LEFT JOIN expenses e ON e.project_id = p.id
   GROUP BY p.id, p.name, p.target_cents, p.target_date
   ```

6. **`view_tag_totals`** (All-time tag spending aggregates)
   ```sql
   CREATE VIEW view_tag_totals AS
   SELECT
       t.id,
       t.name,
       t.color,
       t.description,
       t.is_joint,
       t.is_active,
       COALESCE(ROUND(SUM(e.cost_cents) / 100.0, 2), 0.0) AS total_amount,
       COUNT(e.id)                                          AS expense_count,
       MIN(e.expense_date)                                  AS first_date,
       MAX(e.expense_date)                                  AS last_date
   FROM tags t
   LEFT JOIN expenses e ON e.tag_id = t.id
   GROUP BY t.id, t.name, t.color, t.description, t.is_joint, t.is_active
   ```

7. **`view_joint_account_monthly`** (Joint-account category spending by month)
   ```sql
   CREATE VIEW view_joint_account_monthly AS
   SELECT
       strftime('%Y-%m', e.expense_date)   AS month,
       e.category,
       ROUND(SUM(e.cost_cents) / 100.0, 2) AS total_amount,
       COUNT(*)                             AS expense_count
   FROM expenses e
   INNER JOIN joint_account_categories jac ON jac.category = e.category
   GROUP BY strftime('%Y-%m', e.expense_date), e.category
   ```

### Complex Domain Logic

- **Jobs & Income Analytics (`/analytics/income-by-person`, `/income/latest-salary`, `/jobs`)**:
  Calculates effective monthly base salary per active household member for a target month `YYYY-MM`:
  1. Finds all active jobs where `start_date <= '{target_month}-31'` AND (`end_date IS NULL` OR `end_date >= '{target_month}-01'`) AND `is_active = 1`.
  2. Normalizes frequencies:
     - `monthly`: `amount_cents`
     - `weekly`: `round(amount_cents * 52 / 12)`
     - `biweekly`: `round(amount_cents * 26 / 12)`
     - `annual`: `round(amount_cents / 12)`
  3. If a user has no jobs configured in the DB, it falls back to the legacy historical `SALARY` append-only entry carry-forward.
  4. Sums all one-off non-salary income logged for that month (`BONUS`, `GIFT`, etc.) to produce the total income per person and effective salary ratios.

- **Paybacks Calculation (`/analytics/paybacks`)**:
  Computes payback balances based on individual transactions. For each expense:
  1. **Joint account exclusion:** Expenses whose category is assigned to the joint account are excluded entirely from payback calculations (loaded from `joint_account_categories`).
  2. It reads the effective split override if present, falling back to split allocations, and finally to an equal split.
  3. Resolves personal-pay categories (`PERSONAL COST`, `LEISURE`, `GIFT`) by renaming them dynamically to include the payer name and assigning them a 100% split share to the payer.
  4. Accumulates the net balance per user in cents (positive represents overpayment, negative represents debt).
  5. **Special Deduction Rule:** Subtracts the smaller of Jane's "Combined Fixed" payment and John's "Apartment" payment from John's net balance, and adds it to Jane's net balance (simulating Jane paying John).
  6. Runs a greedy debt simplification algorithm that matches creditors against debtors to yield a minimal list of debt transfer objects (`DebtItem`).

---

## 📂 4. REPO TOPOLOGY

### Monorepo Map

#### Root Configuration Files & Workflow Automation
- **`docker-compose.yml`**: Multi-container architecture orchestrating `backend`, `frontend`, `caddy`, and local `sonarqube`.
- **`sonar-project.properties`**: SonarQube static code analysis configuration.
- **`scripts/run-tests-and-sonar.sh`**: Helper script generating full-stack coverage reports for local SonarQube ingest.
- **`.github/workflows/ci.yml`**: Continuous Integration workflow running Vitest and Pytest test coverage suites.
- **`.github/workflows/deploy.yml`**: Continuous deployment workflow for DigitalOcean droplet deployments.
- **`Caddyfile`**: Routes requests for `jizifin.duckdns.org` (HTTPS/TLS) and local `http://localhost`, proxying `/api/*` to backend and other paths to frontend.
- **`PROJECT.md`**: Project requirements and boundaries.
- **`AGENTS.md`**: System caprom directives for LLM agents and developers.
- **`README.md`**: General setup, features, and test guide.

#### Backend Application (`backend/`)
- **`backend/Dockerfile`**: Configures Python 3.14 environment, installs dependencies via `uv`, exposes port 8000.
- **`backend/pyproject.toml`**: Stores Python project metadata and dependencies.
- **`backend/uv.lock`**: Lockfile securing exact Python package versions.
- **`backend/finance.db`**: Local SQLite database instance (at rest).
- **`backend/app/__init__.py`**: Initialises the `app` package.
- **`backend/app/main.py`**: Declares FastAPI routes, lifespan hooks, WebSocket connection manager for `/ws/finance`, job CRUD endpoints, and financial analytics.
- **`backend/app/models.py`**: Pydantic v2 schemas representing input/output models for all endpoints.
- **`backend/app/database.py`**: Database pool configuration, WAL mode, foreign keys, table and view initializations.
- **`backend/app/crypto_utils.py`**: Server-side cryptography routines executing PBKDF2 key derivation and AES-GCM bulk encryption/decryption for database backups.
- **`backend/tests/`**: Pytest test suite containing 309+ tests (`test_jobs_and_salary.py`, `test_ledger_transfers.py`, `test_budgeting_engine.py`, `test_concurrency_security.py`, `test_import_export_analytics.py`, `test_categories_tags.py`, etc.).

#### Frontend Application (`frontend/`)
- **`frontend/Dockerfile`**: Configures Node.js container and exposes Vite port 5173.
- **`frontend/package.json`**: Manages node dependencies and scripts.
- **`frontend/tailwind.config.js`**: Utility-first Tailwind styling tokens.
- **`frontend/vite.config.js`**: Vite configuration defining dev proxying and build parameters.
- **`frontend/src/main.js`**: Hooks the Svelte application into the DOM.
- **`frontend/src/App.svelte`**: Main application shell, tab routing, sidebar, and selected month switcher.
- **`frontend/src/lib/api.js`**: Central API integration with transparent AES-GCM encryption/decryption on all transaction and job requests.
- **`frontend/src/lib/crypto.js`**: Client-side WebCrypto PBKDF2 and AES-GCM encryption routines with static IV.
- **`frontend/src/lib/stores.js`**: Reactive Svelte writable stores (`jobs`, `incomeEntries`, `expenses`, `users`, `splits`, `projects`, `tags`, `jointAccount`, etc.).
- **`frontend/src/lib/AnalyticsSummary.svelte`**: Monthly totals summary and category spending doughnut chart.
- **`frontend/src/lib/BudgetManager.svelte`**: Monthly category budget limit configuration.
- **`frontend/src/lib/ExpenseForm.svelte`**: Forms for logging/editing expenses with split allocations, tag selection, and conditional project dropdown.
- **`frontend/src/lib/ExpenseList.svelte`**: List of the month's expenses with search & filtering, inline quick tag assignment popover, comprehensive Edit Expense modal, and inline deletion confirmations.
- **`frontend/src/lib/IncomeChart.svelte`**: Monthly income visualization with base salary vs one-off breakdown.
- **`frontend/src/lib/IncomeTab.svelte`**: Unified Income & Employment panel — monthly summary cards, employment streams list with rate/frequency badges, 1-click raise/promotion/leave adjustments, one-off income ledger, and category manager navigation.
- **`frontend/src/lib/JointAccountTab.svelte`**: Joint account management panel — balance overview, category assignment, deposit schedules, expected costs, balance corrections, and settlement.
- **`frontend/src/lib/Login.svelte`**: Master passphrase authentication and database backup import/export.
- **`frontend/src/lib/PaybackVisual.svelte`**: Payback debt visualizer and settlement month locking.
- **`frontend/src/lib/ProjectsTab.svelte`**: Target budget goals, estimated completion timelines, and expense form project selector toggle.
- **`frontend/src/lib/QueryConsole.svelte`**: SQL query console with client-side output decryption.
- **`frontend/src/lib/RealtimeChart.svelte`**: WebSocket live expense ticker chart.
- **`frontend/src/lib/RecurringManager.svelte`**: Automated recurring expense templates.
- **`frontend/src/lib/SettingsTab.svelte`**: Central Settings & Personalization panel — household members, feature modules (opt-in Joint Account, show projects toggle), tab navigation visibility, entry defaults with 1-click currency presets, chart & split visualization styles, mobile display preferences, and decrypted SQLite database export.
- **`frontend/src/lib/SplitManager.svelte`**: Percentage split allocation manager with dynamic salary ratio resets.
- **`frontend/src/lib/TagsTab.svelte`**: Open-ended event tag manager with spending charts.
- **`frontend/src/lib/UserManager.svelte`**: Household member configuration and color palette management.
- **`frontend/vitest.config.js`**: Vitest test configuration with JSDOM and Svelte testing plugins.
- **`frontend/src/test/`**: Vitest test suite with 33 test files and 268+ tests.

---

## 🐳 5. SETUP & CLUSTER INSTRUCTIONS

The application runs as a cluster coordinated via `docker-compose.yml` in a shared bridge network (`app-network`). Caddy handles routing and TLS termination:

```
          Public Traffic (HTTP / HTTPS)
                    │
                    ▼
       ┌───────────────────────────┐
       │   Caddy (Reverse Proxy)   │
       │   Ports: 80 / 443         │
       └─────────────┬─────────────┘
                     │
         ┌───────────┴───────────┐
         ▼                       ▼
┌──────────────────┐   ┌──────────────────┐
│     frontend     │   │     backend      │
│   Port: 5173     │   │    Port: 8000    │
└──────────────────┘   └──────────────────┘
```

### Caddy Reverse Proxy & TLS Configuration
1. **Host Entry Point:** Caddy binds to ports `80` and `443` on the host. It auto-provisions and maintains SSL certificates for `jizifin.duckdns.org` over HTTPS, while serving `http://localhost` and `http://127.0.0.1` over HTTP without TLS for local development.
2. **Backend API Routing:** All requests starting with `/api/*` have their prefix stripped by Caddy's `handle_path` block and are proxied to `http://backend:8000`.
3. **Frontend Routing:** All other paths are proxied to `http://frontend:5173`, serving the Svelte single page application.
4. **WebSocket Support:** Transparent HTTP connection upgrading forwards WebSocket traffic to `/ws/finance`.

---

## 📝 6. DOCUMENTATION MAINTENANCE & COMPLIANCE

Whenever developer workflows, directory layouts, database schemas, or architectural boundaries change, BOTH `AGENTS.md` and `README.md` must be updated to keep documents aligned and prevent AI hallucinations.

---

## 🚨 7. LLM CODE GENERATION RULES

1. **Minimize Context Overhead:** Output ONLY the modified functions or cleanly marked diff blocks.
2. **Flat Composition:** Avoid deep component trees in Svelte.
3. **Zero Deprecation:** Use stable, established APIs.
4. **Data Integrity:** Database receives whole integer cents. UI formats decimal currency units.
5. **SOLID Principles:** Target single-responsibility functions and classes.
6. **Mandatory Test Verification:** After making ANY modification or addition to backend or frontend components, you MUST execute `uv run --directory backend pytest` and `npm --prefix frontend test` and verify that all test suites pass 100% with zero regressions.

---

## 🛡️ 8. PERMISSION HANDLING & SANDBOX RECOVERY PROTOCOL

1. **Immediate Boundary Recognition**: If a command returns a system protection boundary error, treat it as an immutable constraint and pivot inside workspace bounds.
2. **Execution Discipline**: All command working directories (`Cwd`) and file read/writes MUST remain strictly inside the project root.
3. **No `cd` Commands**: Never execute `cd` commands in `run_command`. Set `Cwd` explicitly in tool parameters.
4. **Tooling Enforcement**: Use standard package managers:
   - Python / UV: `uv`
   - Node / NPM: `npm`
