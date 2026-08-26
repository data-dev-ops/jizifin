# Personal Finance Tracker (Jizifin)

A monorepo personal finance tracking application designed for managing shared household expenses, employment incomes, budgets, joint accounts, and paybacks across multi-member and multi-couple households. Built with a high-performance Python/FastAPI backend and a reactive Vanilla Svelte frontend.

---

## 🚀 Key Features

- **Dashboard & Analytics:** Comprehensive overview of balances, monthly totals, category breakdowns, and dynamic real-time spending charts with multi-select household scope filtering (Everyone, individual members, and joint accounts).
- **Jobs & Period-Specific Salary Adjustments:** Define employment contracts and regular income streams per person with customizable frequency (monthly, weekly, bi-weekly, annual), timeline start/end dates, and 1-click raise/promotion/leave adjustments. Supports month-specific salary overrides (`salary_overrides`) for temporary overtime, unpaid leave, or sickness—dynamically recalculating proportional salary ratios strictly for that period without modifying base contracts.
- **Device Display Profiles (Desktop vs. Mobile):** Distinct persistent profiles stored independently in `localStorage` (`jizifin_profile_desktop` and `jizifin_profile_mobile`). Automatically detects client device hardware with live status badge, supporting profile switching, explicit saving, profile cloning, and default resetting.
- **Public Privacy Shield (Stealth Blur):** Frosted-glass CSS blur with hover/tap peek unmasking. Toggleable via a top header action button (`👁️` / `🕶️`) or Settings, protecting account balances, salary details, and transaction amounts in public environments.
- **Quick Display & Workflow Presets:** 1-Click functional interface presets:
  - `⚡ Streamlined`: Minimal form inputs and compact single-line ledger rows for rapid day-to-day logging.
  - `👓 High Legibility`: 115% larger typography, high-contrast borders, and whole-unit summary rounding.
  - `🏡 Standard / Balanced`: Default comprehensive layout with compact density and core household modules.
  - `📊 Detailed / Power User`: Exact `.00` precision everywhere, detailed metadata chips, and advanced tabs (Query Console, Budgets).
- **View Depth & Rapid Logging Accelerators:** Configurable project lifecycle filters (`Active Only`, `In Progress`, `All Archive`), ledger row density (`Minimal`, `Compact`, `Detailed`), currency precision mode, smart form memory (`Remember Last Used`, `Static Defaults`, `Empty`), and granular form field toggles.
- **Exact Basis-Point & Float Percentage Splits:** Configure category split allocations with full basis-point float precision (`0.01%`), supporting proportional income splits (e.g. $26.6667\% / 13.3333\% / 25.9259\% / 11.1111\% / 18.5185\%$) without forced integer coercion.
- **Signed Hare-Niemeyer / Largest Remainder Math:** Integer-cent split distribution supporting positive transactions, zero, and negative refunds/credit memos using mathematical `math.floor`, paired with a deterministic transaction-salted SHA-256 tie-breaker to prevent alphabetical bias.
- **Multi-Tenant Joint Accounts:** Isolated joint accounts owned by specific household sub-groups (e.g., Couple AB, Couple CD) with custom safety margins, expected monthly costs, per-user monthly deposit schedules, and signed balance corrections.
- **Payback Calculator & Graph Decomposition:** Computes exact net balances based on payer, category shares, and joint exclusions. Employs connected-component graph decomposition to isolate subgroup debts from cross-household transfers before executing greedy debt simplification.
- **Dynamic Tag Timelines & Boundary Enforcement:** Open-ended color-coded tag labeling system with `start_date` and `end_date` active windows. Validates that expenses fall strictly within active tag milestones and prevents retrospective tag window shrinkage.
- **Projects & Settlement Equity Balance Sheets:** Long-term project budget targets with estimated completion dates and dedicated point-in-time participant equity balance sheets (`GET /projects/{id}/settlement`), tracking effective funding vs assigned liability.
- **Expense Tracking & Full Management:** Log and manage shared/personal expenses with split percentage overrides, project allocations, and tag associations. Features a live search/filter toolbar, quick inline tag popover assignment, a full-featured Edit Expense modal, and date-locking for settled historical months.
- **Centralized Settings & Personalization:** Comprehensive settings panel managing household members and color palettes, feature modules, navigation tab visibility, entry defaults with 1-click currency presets (€, $, £, CHF, ¥, kr), chart & split visualization styles, mobile layout density, and zero-knowledge encrypted database backups.
- **Zero-Knowledge Privacy:** Client-side AES-GCM 256-bit encryption (via Web Crypto API) ensures all names, descriptions, notes, and category labels are stored encrypted at rest on the server, with secure in-place server-side database export/import utilities.
- **Modern UI & Design System:** Modern typography powered by Plus Jakarta Sans and JetBrains Mono, `tabular-nums` formatting on all currency and financial amounts, and glassmorphic depth.

---

## 🛠 Tech Stack

**Backend:**
- Python 3.14
- FastAPI (Strict Pydantic v2 validation)
- SQLite (`aiosqlite`) with WAL mode and foreign key enforcement
- `uv` for dependency and virtual environment management
- Pytest with coverage reporting (`pytest-cov`, `pytest-asyncio`)

**Frontend:**
- Vanilla Svelte (Flat component architecture, no heavy SSR framework)
- Tailwind CSS (Utility-first styling, glassmorphism tokens)
- Chart.js (Native Canvas 2D rendering)
- Vite build tool & development server
- Vitest with `@testing-library/svelte` and JSDOM

**Infrastructure & Orchestration:**
- Docker Compose
- Caddy Server (Reverse Proxy, automatic HTTPS/TLS termination, and WebSocket proxying)
- SonarQube static code quality analysis

---

## 📦 Installation & Setup

### Option 1: Docker Compose (Recommended Cluster Setup)
You can run the full production-like stack using Docker Compose. The setup consists of containers in a shared bridge network (`app-network`):
- **`finance-tracker-backend`**: Runs the FastAPI server on port 8000.
- **`finance-tracker-frontend`**: Runs the Svelte application using Vite on port 5173.
- **`finance-tracker-caddy`**: Serves as the single entry gateway, binding host ports `80` and `443`.
- **`finance-tracker-sonarqube`**: Local SonarQube server on port 9000 (for quality scans).

#### The Role of Caddy Reverse Proxy
Caddy orchestrates routing and traffic control for the cluster:
- **TLS Termination:** Automatically provisions and renews SSL certificates for `jizifin.duckdns.org` over HTTPS, while serving `http://localhost` and `http://127.0.0.1` over HTTP for local development.
- **API Routing:** Proxies all paths matching `/api/*` to the backend service at `http://backend:8000` (stripping the `/api` prefix).
- **Frontend Routing:** Proxies all other requests to the frontend service at `http://frontend:5173`.
- **WebSocket Upgrade:** Forwards HTTP connection upgrade headers automatically, allowing client WebSockets to connect to `/ws/finance` transparently.

#### Running the Cluster
1. Ensure ports 80 and 443 are free.
2. Run the build and start command from the project root:
   ```bash
   docker compose up --build -d
   ```
3. Access the application at `https://jizifin.duckdns.org` in production or `http://localhost` when running locally.
4. Inspect logs using:
   ```bash
   docker compose logs -f
   ```

---

### Option 2: Local Development Setup

**Prerequisites:**
- Python 3.14+
- `uv` package manager (`uv 0.11+` or compatible)
- Node.js (`v20+` or `v24+`)
- `npm`

#### 1. Start the Backend:
```bash
cd backend
uv run uvicorn app.main:app --reload --port 8000
```
This boots the FastAPI dev server on `http://localhost:8000` and creates or connects to `finance.db`.

#### 2. Start the Frontend:
```bash
cd frontend
npm install
npm run dev
```
This starts the Vite server on `http://localhost:5173`. Local client development routes requests to `/api` which are proxied to `http://localhost:8000` (configured in `vite.config.js`).

---

## 🧪 Testing & Quality Assurance

The application features full-stack automated test suites ensuring zero regression and mathematical precision across all domain logic.

> **CRITICAL REQUIREMENT:** All future code changes, migrations, and additions must execute and pass 100% of both test suites (`pytest` and `vitest`) with zero regressions before being committed.

### 1. Backend Test Suite & Integration Scenarios (Pytest)
- **Framework:** Pytest, `pytest-asyncio`, and `pytest-cov`.
- **Coverage:** **328 passed tests** across 17 test modules, including:
  - `tests/test_scenarios_integration.py`: End-to-end integration scenarios verifying isolated joint accounts, basis-point income splits, negative refund cent rounding, salted tie-breaking invariance over 500 transactions, tag active timeline bounds, point-in-time project equity snapshots, and graph-decomposed couple debt isolation.
  - Core domain suites: `test_jobs_and_salary.py`, `test_ledger_transfers.py`, `test_budgeting_engine.py`, `test_concurrency_security.py`, `test_import_export_analytics.py`, `test_categories_tags.py`, `test_multi_household_couples.py`, etc.

```bash
# Run full backend test suite with coverage:
uv run --directory backend pytest --cov=app --cov-report=xml:coverage.xml --cov-report=term

# Run integration scenario tests specifically:
uv run --directory backend pytest tests/test_scenarios_integration.py

# Run backend tests inside Docker container:
docker run --rm -v $(pwd)/backend/app:/app/app -v $(pwd)/backend/tests:/app/tests jizifin-backend-test pytest
```

### 2. Frontend Test Suite (Vitest)
- **Framework:** Vitest, `@testing-library/svelte`, JSDOM, and `jsdom-testing-mocks`.
- **Coverage:** **312 passed tests** across 36 test files covering encryption/decryption, stores, device profiles, workflow presets, API error handling, Svelte components (`IncomeTab`, `SplitManager`, `SettingsTab`, `JointAccountTab`, `ExpenseForm`, `ExpenseList`, `BudgetManager`, `TagsTab`, `ProjectsTab`, `QueryConsole`, etc.), form validations, and user workflows.

```bash
# Run Vitest test suite:
npm --prefix frontend test

# Generate frontend lcov coverage:
npm --prefix frontend run test:coverage

# Run frontend tests inside Docker container:
docker run --rm \
  -v $(pwd)/frontend/src:/app/src \
  -v $(pwd)/frontend/index.html:/app/index.html \
  -v $(pwd)/frontend/tailwind.config.js:/app/tailwind.config.js \
  -v $(pwd)/frontend/vite.config.js:/app/vite.config.js \
  jizifin-frontend-test npm test
```

---

## 🔍 SonarQube & CI/CD Pipelines

- **Local SonarQube Analysis:** Run `./scripts/run-tests-and-sonar.sh` to generate frontend and backend coverage reports and ingest them into local SonarQube (`http://localhost:9000`).
- **GitHub CI (`.github/workflows/ci.yml`)**: Runs frontend and backend test suites on every `push` and `pull_request`, opening an issue on failures and uploading coverage reports to SonarQube.
- **DigitalOcean Continuous Deployment (`.github/workflows/deploy.yml`)**: Automates live server deployment via `docker compose up --build -d backend frontend caddy`.

---

## 🏗 Architecture & Design Principles

- **Integer Cents Precision:** All currencies are represented as whole `INTEGER` cents at the database layer to eliminate floating-point rounding errors. Presentation and decimal formatting (`cents / 100.0`) occur strictly at the presentation boundary.
- **Mathematical Exactness:** Split distributions use Hare-Niemeyer / Largest Remainder with signed math floor and transaction-salted SHA-256 tie-breaking.
- **No ORMs:** Backend endpoint logic executes raw, optimized, ANSI-compliant SQL queries directly with `aiosqlite`.
- **Zero-Knowledge Privacy:** Client derives a 256-bit AES-GCM key from the user passphrase. Sensitive text columns are encrypted before transmission. Deterministic encryption enables exact matching, indexing, and foreign key referential integrity without plaintext exposure on the server disk.

---

## 📜 License
Private.

