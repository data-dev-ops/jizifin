# 🤖 SYSTEM CAPROM: FinanceTracker LLM Agent Directives

**TARGET:** Gemini Pro / Advanced LLM Agent
**CONTEXT:** Monorepo Personal Finance Tracker (Jim & Zina)
**PRIME DIRECTIVE:** Strictly adhere to the technical stack, execution paths, and database schemas defined below. Prioritize zero-regression, ANSI-compliant SQL, and flat Svelte component design.

---

## ⚙️ 1. ENVIRONMENT & EXECUTION (STRICT PATHS)
Do NOT assume standard system locations. You MUST use the following binary paths for all terminal commands, script executions, and package management.

- **Python Runtime:** `/Users/jim/.local/bin/python` (Python 3.14)
- **Dependency Manager:** `/Users/jim/.local/bin/uv` (uv 0.11.26, Apple Silicon)
- **Node Runtime:** `/Users/jim/.nvm/versions/node/v24.18.0/bin/node`
- **NPM:** `/Users/jim/.nvm/versions/node/v24.18.0/bin/npm`

**Execution Commands:**
- Backend Dev Server: `/Users/jim/.local/bin/uv run uvicorn app.main:app --reload --port 8000`
- Add Python Package: `/Users/jim/.local/bin/uv add <package>`
- Frontend Dev Server: `/Users/jim/.nvm/versions/node/v24.18.0/bin/npm run dev`
- Frontend Install: `/Users/jim/.nvm/versions/node/v24.18.0/bin/npm install <package>`
- **Docker Compose Cluster:** Orchestrates `backend` and `frontend` containers via `docker-compose.yml`. Run `docker compose up --build -d` from the root to start the full stack.

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
   - **Encrypted Columns:** `users.name`, `splits.category`, `income_categories.category`, `projects.name`, `expenses.name`, `expenses.who_paid`, `expenses.category`, `expense_overrides.user_name`, `income.name`, `income.who`, `income.category`, `recurring_expenses.name`, `recurring_expenses.who_paid`, `recurring_expenses.category`, `budgets.category`, `split_allocations.category`, `split_allocations.user_name`, `tags.name`, `tags.description`.
   - **Plaintext Columns:** Numeric amounts (cents), dates, integer primary/foreign keys, and the `settlements` table.

### 🚨 Coding Style Conventions & Deviations
- **API Error Handling Flow**:
  - `frontend/src/lib/api.js` utilizes a central `request()` helper that throws an explicit `Error` object on non-2xx HTTP response codes (propagating status and response body).
  - Svelte components (such as `Login.svelte` or `ExpenseForm.svelte`) call api methods inside `try...catch` blocks and assign `err.message` to local reactive error variables (e.g., `errorMsg`) to render red alert blocks in the user interface.

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

5. **`tags`** (Open-ended label tags)
   - `id` (INTEGER PRIMARY KEY AUTOINCREMENT)
   - `name` (TEXT NOT NULL UNIQUE CHECK(length(name) <= 256)) — Encrypted.
   - `color` (TEXT NOT NULL DEFAULT '#f59e0b')
   - `description` (TEXT CHECK(length(description) <= 512)) — Encrypted.
   - `created_at` (TEXT NOT NULL DEFAULT (datetime('now')))

6. **`expenses`** (Core expense ledger)
   - `id` (INTEGER PRIMARY KEY AUTOINCREMENT)
   - `name` (TEXT NOT NULL CHECK(length(name) <= 256)) — Encrypted.
   - `cost_cents` (INTEGER NOT NULL CHECK(cost_cents > 0))
   - `expense_date` (TEXT NOT NULL CHECK(expense_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'))
   - `who_paid` (TEXT NOT NULL REFERENCES users(name) ON UPDATE CASCADE) — Encrypted.
   - `category` (TEXT NOT NULL REFERENCES splits(category) ON UPDATE CASCADE) — Encrypted.
   - `project_id` (INTEGER REFERENCES projects(id) ON DELETE SET NULL)
   - `tag_id` (INTEGER REFERENCES tags(id) ON DELETE SET NULL)

7. **`expense_overrides`** (Per-expense override split allocations)
   - `expense_id` (INTEGER NOT NULL REFERENCES expenses(id) ON DELETE CASCADE)
   - `user_name` (TEXT NOT NULL REFERENCES users(name) ON UPDATE CASCADE ON DELETE CASCADE) — Encrypted.
   - `pct` (REAL NOT NULL CHECK(pct >= 0.0 AND pct <= 100.0))
   - *Primary Key*: `(expense_id, user_name)`

8. **`income`** (Append-only ledger)
   - `id` (INTEGER PRIMARY KEY AUTOINCREMENT)
   - `name` (TEXT NOT NULL CHECK(length(name) <= 256)) — Encrypted.
   - `amount_cents` (INTEGER NOT NULL CHECK(amount_cents > 0))
   - `who` (TEXT NOT NULL REFERENCES users(name) ON UPDATE CASCADE) — Encrypted.
   - `category` (TEXT NOT NULL CHECK(length(category) <= 256)) — Encrypted.
   - `income_date` (TEXT NOT NULL CHECK(income_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'))
   - *Indexes*: `idx_income_who_date` on `(who, income_date DESC)`

9. **`recurring_expenses`** (Templates for automated expenses)
   - `id` (INTEGER PRIMARY KEY AUTOINCREMENT)
   - `name` (TEXT NOT NULL CHECK(length(name) <= 256)) — Encrypted.
   - `cost_cents` (INTEGER NOT NULL CHECK(cost_cents > 0))
   - `who_paid` (TEXT NOT NULL REFERENCES users(name) ON UPDATE CASCADE) — Encrypted.
   - `category` (TEXT NOT NULL REFERENCES splits(category) ON UPDATE CASCADE) — Encrypted.
   - `day_of_month` (INTEGER NOT NULL CHECK(day_of_month >= 1 AND day_of_month <= 31))

10. **`budgets`** (Monthly limit configuration)
    - `category` (TEXT NOT NULL REFERENCES splits(category) ON UPDATE CASCADE ON DELETE CASCADE) — Encrypted.
    - `month` (TEXT NOT NULL CHECK(month = 'ALL' OR month GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]'))
    - `limit_cents` (INTEGER NOT NULL CHECK(limit_cents >= 0))
    - *Primary Key*: `(category, month)`

11. **`settlements`** (Month locking logs)
    - `month` (TEXT PRIMARY KEY CHECK(month GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]'))
    - `settled_at` (TEXT NOT NULL)
    - `net_balance_transferred_cents` (INTEGER NOT NULL)

12. **`split_allocations`** (Default split allocations)
    - `category` (TEXT NOT NULL REFERENCES splits(category) ON UPDATE CASCADE ON DELETE CASCADE) — Encrypted.
    - `user_name` (TEXT NOT NULL REFERENCES users(name) ON UPDATE CASCADE ON DELETE CASCADE) — Encrypted.
    - `pct` (REAL NOT NULL CHECK(pct >= 0.0 AND pct <= 100.0))
    - *Primary Key*: `(category, user_name)`

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
   *Impact:* Because categories are stored as deterministic ciphertexts, they are grouped correctly.

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
   *Impact:* Payer usernames are grouped correctly as deterministic ciphertexts.

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
       COALESCE(ROUND(SUM(e.cost_cents) / 100.0, 2), 0.0) AS total_amount,
       COUNT(e.id)                                          AS expense_count,
       MIN(e.expense_date)                                  AS first_date,
       MAX(e.expense_date)                                  AS last_date
   FROM tags t
   LEFT JOIN expenses e ON e.tag_id = t.id
   GROUP BY t.id, t.name, t.color, t.description
   ```
   *Impact:* Performs aggregations using the tag integer IDs (unencrypted).

### Complex Domain Logic
- **Paybacks Calculation (`/analytics/paybacks`)**:
  Computes payback balances based on individual transactions. For each expense:
  1. It reads the effective split override if present, falling back to split allocations, and finally to an equal split.
  2. Resolves personal-pay categories (`PERSONAL COST`, `LEISURE`, `GIFT`) by renaming them dynamically to include the payer name and assigning them a 100% split share to the payer.
  3. Accumulates the net balance per user in cents (positive represents overpayment, negative represents debt).
  4. **Special Deduction Rule:** Subtracts the smaller of Zina's "Combined Fixed" payment and Jim's "Apartment" payment from Jim's net balance, and adds it to Zina's net balance (simulating Zina paying Jim).
  5. Runs a greedy debt simplification algorithm that matches creditors against debtors to yield a minimal list of debt transfer objects (`DebtItem`).
- **Income Salary Carry-forward (`/analytics/income-by-person`)**:
  Calculates the income logged per active user for a target month. Since salary entries are append-only:
  1. It fetches only the latest `SALARY` category entry per user for the target month.
  2. If a user has no `SALARY` entry in the target month, the system fetches their most recent historical `SALARY` entry from previous months and carries it forward automatically (setting `is_carried=True` in the payload).

---

## 📂 4. REPO TOPOLOGY

### Monorepo Map

#### Root Configuration Files
- **`docker-compose.yml`**: Defines the multi-container application architecture, orchestrating the backend service, Svelte frontend environment, and Caddy proxy server.
- **`Caddyfile`**: Routes public requests for `jizifin.duckdns.org` (HTTPS with automatic TLS) and local development requests for `http://localhost` / `http://127.0.0.1` (HTTP), proxying requests for `/api/*` to the backend and other paths to the frontend.
- **`PROJECT.md`**: Outlines overall project guidelines, architectural boundaries, and technology requirements.
- **`AGENTS.md`**: This document (developer and LLM instructions).
- **`README.md`**: General setup and deployment guide.

#### Backend Application (`backend/`)
- **`backend/Dockerfile`**: Configures python 3.14 environment, installs dependencies via `uv`, and exposes port 8000.
- **`backend/pyproject.toml`**: Stores Python project metadata and dependencies.
- **`backend/uv.lock`**: Lockfile securing exact Python package versions.
- **`backend/finance.db`**: Local SQLite database instance (at rest).
- **`backend/app/__init__.py`**: Initialises the `app` package.
- **`backend/app/main.py`**: Declares FastAPI routes, lifespan startup/shutdown hooks (initialising SQLite tables/views and starting APScheduler), defines the WebSocket connection manager for `/ws/finance`, and hosts analytics calculations.
- **`backend/app/models.py`**: Holds Pydantic v2 schemas representing input and output models for all endpoints.
- **`backend/app/database.py`**: Handles database connection pool configuration, sets SQLite journal mode to WAL, enforces foreign keys, and initializes tables and views.
- **`backend/app/crypto_utils.py`**: Server-side cryptography routines executing PBKDF2 key derivation and deterministic AES-GCM bulk encryption/decryption of SQLite databases during imports/exports.

#### Frontend Application (`frontend/`)
- **`frontend/Dockerfile`**: Configures Node.js container, installs node dependencies, and exposes the Vite development port 5173.
- **`frontend/package.json`**: Manages node dependencies and scripts.
- **`frontend/tailwind.config.js`**: Tailors utility-first Tailwind classes.
- **`frontend/vite.config.js`**: Vite configuration defining development routing and build parameters.
- **`frontend/src/main.js`**: Hooks the Svelte application shell into the index DOM.
- **`frontend/src/App.svelte`**:Renders the tab navigation, sidebar, and selected month switcher. Performs startup database loads.
- **`frontend/src/lib/api.js`**: Integrates Svelte stores with backend endpoints, mapping responses to state stores and handling transparent encryption/decryption on transactions.
- **`frontend/src/lib/crypto.js`**: Client-side cryptography utilizing the browser's SubtleCrypto API to execute PBKDF2 key derivation and deterministic AES-GCM operations with a static IV.
- **`frontend/src/lib/stores.js`**: Stores reactive variables (such as active tables, auth salts, crypto keys, preferences).
- **`frontend/src/lib/AnalyticsSummary.svelte`**: Shows a dashboard card summary of monthly totals and a category spending doughnut chart (via Chart.js).
- **`frontend/src/lib/BudgetManager.svelte`**: Handles adding, editing, and deleting monthly category spending limits.
- **`frontend/src/lib/ExpenseForm.svelte`**: Forms for creating/updating expenses, supporting splits, overrides, project/tag selection, and date-locks.
- **`frontend/src/lib/ExpenseList.svelte`**: Renders a list of the month's expenses, handling deletion confirmations.
- **`frontend/src/lib/IncomeChart.svelte`**: Displays incomes, using dashed segments to represent carried-forward salary entries.
- **`frontend/src/lib/IncomeTab.svelte`**: Full income management panel — per-month ledger with delete, Log Income form (name, amount, who, dynamic category, date), and inline income category manager (add/remove). All crypto transparent via api.js.
- **`frontend/src/lib/Login.svelte`**: Prompts for master passphrase, derives key, imports/restores databases, and unlocks the app.
- **`frontend/src/lib/PaybackVisual.svelte`**: Visualizes payback debts and settlement operations.
- **`frontend/src/lib/ProjectsTab.svelte`**: Manages projects with targets and estimated completions.
- **`frontend/src/lib/QueryConsole.svelte`**: Allows executing raw SQL statements, decrypting text output before displaying.
- **`frontend/src/lib/RealtimeChart.svelte`**: Connects to the WebSockets ticker to plot dynamic additions.
- **`frontend/src/lib/RecurringManager.svelte`**: Formulates templates to log expenses on a specified day of the month automatically.
- **`frontend/src/lib/SplitManager.svelte`**: Renders percentage allocations per category.
- **`frontend/src/lib/TagsTab.svelte`**: Configures tags, showing tag details and charts.
- **`frontend/src/lib/UserManager.svelte`**: Manages active users, colours, and deletions.

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
1. **Host Entry Point:** Caddy binds to ports `80` and `443` on the host. It auto-provisions and maintains SSL certificates for `jizifin.duckdns.org` over HTTPS, while serving `http://localhost` and `http://127.0.0.1` over HTTP without TLS for local developer setups.
2. **Backend API Routing:** All requests starting with `/api/*` have their prefix stripped by Caddy's `handle_path` block and are proxied to `http://backend:8000`.
3. **Frontend Routing:** All other paths are proxied to `http://frontend:5173`, serving the Svelte single page application.
4. **WebSocket Support:** Caddy handles transparent HTTP connection upgrading. Requests to `/ws/finance` are forwarded directly to the frontend's handler, which routes to the backend correctly when the client establishes the WebSocket connection.

### How to Run the Cluster
From the project root directory, run:
```bash
docker compose up --build -d
```
This builds container images and runs them in the background. You can inspect logs via:
```bash
docker compose logs -f
```

---

## 📝 6. DOCUMENTATION MAINTENANCE & COMPLIANCE

Whenever developer workflows, directory layouts, database schemas, or architectural boundaries change, BOTH `AGENTS.md` and `README.md` must be updated to keep documents aligned and prevent AI hallucinations.

### Step-by-Step Documentation Update Checklist:
1. **Verify Database Structure:** If a schema migration is added, read `backend/app/database.py` and inspect the columns, types, check constraints, and views. Update Section 3 of `AGENTS.md` to match the schema exactly.
2. **Verify Cryptographic Rules:** If new columns are marked as sensitive or tag/settlement encryption changes, verify columns in both `crypto.js` and `crypto_utils.py`. Update Section 2's cryptographic column list.
3. **Update Directory Maps:** If files are added, renamed, or deleted, update Section 4 of `AGENTS.md` with the new file role description.
4. **Review Setup Steps:** If docker images, ports, or routing definitions in Caddyfile alter, update Section 5 of `AGENTS.md` and the setup instructions in `README.md`.
5. **Cross-validate:** Check that no outdated references exist in `AGENTS.md` or `README.md`.

---

## 🚨 7. LLM CODE GENERATION RULES

1. **Minimize Context Overhead:** When refactoring, output ONLY the modified functions or cleanly marked functional diff blocks. Do not rewrite unmodified code.
2. **Flat Composition:** Avoid deep component trees in Svelte.
3. **Zero Deprecation:** Use stable, established APIs. Do not invent beta features for FastAPI or Svelte.
4. **Data Integrity:** Never hallucinate or alter the whole-cent to float-euro conversion math. Database receives whole integers. UI presents decimals.
5. **SOLID Principles:** Adhere rigorously to SOLID principles. Target single-responsibility functions and classes, keeping components open for extension but closed for modification.
6. **Use-of-Understanding:** Produce readable, explicitly clear "use-of-understanding" code. Prioritize clear, maintainable logic and comments over opaque, clever one-liners.
