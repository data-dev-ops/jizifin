# Personal Finance Tracker (Jizifin)

A monorepo personal finance tracking application designed for managing shared household expenses, employment incomes, budgets, joint accounts, and paybacks. Built with a high-performance Python/FastAPI backend and a reactive Vanilla Svelte frontend.

---

## 🚀 Key Features

- **Dashboard & Analytics:** Comprehensive overview of balances, monthly totals, category breakdowns, and dynamic real-time spending charts.
- **Jobs & Employment Streams:** Define employment contracts and regular income streams per person with customizable frequency (monthly, weekly, bi-weekly, annual), timeline start/end dates, and 1-click raise/promotion/leave adjustments. Effective monthly base salaries and household split ratios are computed automatically for any active or historical month.
- **One-Off Income & Bonus Ledger:** Append-only ledger for ad-hoc income such as performance bonuses, tax returns, gifts, and dividends, supporting personal or joint account destinations.
- **Expense Tracking & Full Management:** Log and manage shared/personal expenses with split percentage overrides, project allocations, and tag associations. Features a live search/filter toolbar, quick inline tag popover assignment, a full-featured Edit Expense modal, and date-locking for settled historical months.
- **Split Management:** Configure exact percentage splits per user for different expense categories or instantly re-calculate split allocations based on current salary ratios.
- **Joint Account Management:** Track shared account balances, safety margins, per-category expected costs, per-user monthly deposit obligations, manual balance corrections (top-ups/withdrawals), and automated settlement modes with payback exclusion. Configurable as an opt-in module.
- **Payback Calculator & Debt Simplification:** Computes exact net balances based on payer, category shares, and joint exclusions, running a greedy debt simplification algorithm and applying custom household deduction rules.
- **Budgets & Projects:** Track monthly spending limits per category and monitor long-term project budget targets with estimated completion projections. Includes toggleable expense form project selector integration.
- **Tags & Labels:** Open-ended color-coded tag labeling system for tracking multi-category events (e.g., vacations, renovations, weddings) across time, with 1-click inline tag assignment directly on expense rows.
- **Recurring Expenses:** Formulate templates to log routine subscription costs and fixed bills automatically on a specified day of the month.
- **Centralized Settings & Personalization:** Comprehensive 7-domain settings panel managing household members and color palettes, feature modules, navigation tab visibility, entry defaults with 1-click currency presets (€, $, £, CHF, ¥, kr), chart & split visualization styles, mobile layout density, and zero-knowledge encrypted database backups.
- **Zero-Knowledge Privacy:** Client-side AES-GCM 256-bit encryption (via Web Crypto API) ensures all names, descriptions, notes, and category labels are stored encrypted at rest on the server, with secure in-place server-side database export/import utilities.

---

## 🛠 Tech Stack

**Backend:**
- Python 3.14
- FastAPI (Strict Pydantic v2 validation)
- SQLite (`aiosqlite`) with WAL mode and foreign key enforcement
- `uv` for dependency and virtual environment management
- Pytest with coverage reporting

**Frontend:**
- Vanilla Svelte (Flat component architecture, no heavy SSR framework)
- Tailwind CSS (Utility-first styling)
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

The application features full-stack automated test suites ensuring zero regression and mathematical precision.

### 1. Frontend Test Suite (Vitest)
- **Framework:** Vitest, `@testing-library/svelte`, JSDOM, and `jsdom-testing-mocks`.
- **Setup & Polyfills (`src/test/setup.js`)**: Polyfills Node `webcrypto` for browser AES-GCM 256-bit encryption, Canvas 2D context for Chart.js graphics, `ResizeObserver`, and establishes a global fetch router.
- **Coverage:** 33 test suites with 268+ tests covering encryption/decryption, API error handling, Svelte components (`IncomeTab`, `SplitManager`, `SettingsTab`, `JointAccountTab`, `ExpenseForm`, `ExpenseList`, `BudgetManager`, `TagsTab`, `ProjectsTab`, `QueryConsole`, etc.), form validations, and user workflows.

```bash
# Run Vitest test suite:
npm --prefix frontend test

# Generate frontend lcov coverage:
npm --prefix frontend run test:coverage
```

### 2. Backend Test Suite (Pytest)
- **Framework:** Pytest, `pytest-asyncio`, and `pytest-cov`.
- **Coverage:** 306+ tests covering jobs and salary timelines, joint account management, ledger transfers, currency precision, budgeting engine, locking reconciliation, and deterministic cryptography.

```bash
# Run Pytest test suite:
uv run --directory backend pytest --cov=app --cov-report=xml:coverage.xml --cov-report=term
```

---

## 🔍 SonarQube & CI/CD Pipelines

- **Local SonarQube Analysis:** Run `./scripts/run-tests-and-sonar.sh` to generate frontend and backend coverage reports and ingest them into local SonarQube (`http://localhost:9000`).
- **GitHub CI (`.github/workflows/ci.yml`)**: Runs frontend and backend test suites on every `push` and `pull_request`, opening an issue on failures and uploading coverage reports to SonarQube.
- **DigitalOcean Continuous Deployment (`.github/workflows/deploy.yml`)**: Automates live server deployment via `docker compose up --build -d backend frontend caddy`.

---

## 🏗 Architecture & Design Principles

- **Integer Cents Precision:** All currencies are represented as whole `INTEGER` cents at the database layer to eliminate floating-point rounding errors. Presentation and decimal formatting (`cents / 100.0`) occur strictly at the presentation boundary.
- **No ORMs:** Backend endpoint logic executes raw, optimized, ANSI-compliant SQL queries directly with `aiosqlite`.
- **Zero-Knowledge Privacy:** Client derives a 256-bit AES-GCM key from the user passphrase. Sensitive text columns are encrypted before transmission. Deterministic encryption enables exact matching, indexing, and foreign key referential integrity without plaintext exposure on the server disk.

---

## 📜 License
Private.
