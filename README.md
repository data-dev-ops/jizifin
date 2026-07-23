# Personal Finance Tracker (Jizifin)

A monorepo personal finance tracking application designed for managing shared household expenses, incomes, budgets, and paybacks. Built with a Python/FastAPI backend and a Vanilla Svelte frontend.

---

## 🚀 Features

- **Dashboard & Analytics:** Comprehensive overview of balances, monthly totals, and category breakdowns.
- **Expense & Income Tracking:** Log shared and personal expenses, and keep an append-only ledger for income (including salary carry-forward).
- **Split Management:** Configure exact percentage splits per user for different expense categories.
- **Payback Calculator:** Automatically calculates who owes whom based on who paid and the expected split ratio, including a custom Combined Fixed vs. Apartment deduction rule.
- **Realtime Updates:** WebSocket integration for live updates across clients when new expenses are added.
- **Budgeting & Projects:** Track ongoing budgets and project-specific expenses.
- **Recurring Expenses:** Manage and track subscriptions and fixed monthly costs.
- **Data Privacy:** Client-side encryption using SubtleCrypto (AES-GCM 256-bit with static IVs) to ensure zero-knowledge database storage at rest, with secure server-side bulk backup import/export utilities.

---

## 🛠 Tech Stack

**Backend:**
- Python 3.14
- FastAPI (Pydantic v2 validation)
- SQLite (`aiosqlite`) with WAL mode
- `uv` for dependency management

**Frontend:**
- Vanilla Svelte (No SvelteKit SSR)
- Tailwind CSS
- Chart.js (Native Canvas rendering)
- Vite

**Infrastructure:**
- Docker Compose
- Caddy Server (Reverse Proxy & TLS termination)

---

## 📦 Installation & Setup

### Option 1: Docker Compose (Recommended Cluster Setup)
You can run the entire production-like stack using Docker Compose. The setup consists of three containers in a bridge network (`app-network`):
- **`finance-tracker-backend`**: Runs the FastAPI server on port 8000.
- **`finance-tracker-frontend`**: Runs the Svelte app using Vite on port 5173.
- **`finance-tracker-caddy`**: Serves as the single entry gateway, binding host ports `80` and `443`.

#### The Role of Caddy Reverse Proxy
Caddy orchestrates routing and traffic control for the cluster:
- **TLS Termination:** Automatically provisions and renews SSL certificates for `jizifin.duckdns.org` over HTTPS, while serving `http://localhost` and `http://127.0.0.1` over HTTP for local development.
- **API Routing:** Proxies all paths matching `/api/*` to the backend service at `http://backend:8000` (stripping the `/api` prefix).
- **Frontend Routing:** Proxies all other requests to the frontend service at `http://frontend:5173`.
- **WebSocket Upgrade:** Forward HTTP connection upgrade headers automatically, allowing client WebSockets to connect to `/ws/finance` transparently.

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
- Python 3.14
- `uv` package manager (`uv 0.11.26` or compatible)
- Node.js (tested with `v24.18.0`)
- `npm`

#### 1. Start the Backend:
```bash
cd backend
/Users/jim/.local/bin/uv run uvicorn app.main:app --reload --port 8000
```
This boots the FastAPI dev server on `http://localhost:8000` and creates or connects to `finance.db`.

#### 2. Start the Frontend:
```bash
cd frontend
/Users/jim/.nvm/versions/node/v24.18.0/bin/npm install
/Users/jim/.nvm/versions/node/v24.18.0/bin/npm run dev
```
This starts the Vite server on `http://localhost:5173`. Note that local client development routes requests to `/api` which must be proxied to `http://localhost:8000` (configured in `vite.config.js`).

---

## 🧪 Frontend Testing & Quality Assurance

The application features a comprehensive automated testing suite built with **Vitest**, **@testing-library/svelte**, **JSDOM**, and **jsdom-testing-mocks**.

### Test Suite Structure
- **Setup & Polyfills (`src/test/setup.js`)**: Polyfills Node `webcrypto` for browser AES-GCM 256-bit encryption, Canvas 2D context for Chart.js graphics, `ResizeObserver`, and establishes a global fetch router for API endpoints.
- **Unit & Component Tests (`src/test/`)**: Contains 18 test files with 72 tests covering cryptographic key derivation, API error handling, Svelte components (`App.svelte`, `ExpenseForm`, `BudgetManager`, `IncomeTab`, `SplitManager`, `ProjectsTab`, `TagsTab`, `QueryConsole`, `PaybackVisual`, `UserManager`, etc.), form validations, date locking, and inline deletion flows.

### Running Frontend Tests
Run the test suite from the root or frontend directory:
```bash
# From project root:
/Users/jim/.nvm/versions/node/v24.18.0/bin/npm --prefix frontend test

# Or from frontend directory:
cd frontend
/Users/jim/.nvm/versions/node/v24.18.0/bin/npm test
```

> ⚠️ **Mandatory Verification Directive:** Whenever modifying or adding code in `frontend/src/lib/*` or `frontend/src/App.svelte`, developers and AI agents MUST run the test suite to verify 100% pass rate with zero regressions before declaring a task complete.

---

## 🏗 Architecture & Design Principles

- **Database Constraints:** Currency amounts are strictly `INTEGER` cents to prevent floating-point calculation errors. Conversion to decimal euros (`cents / 100.0`) happens only at presentation/response boundaries.
- **No ORMs:** Backend endpoint logic executes raw, optimized, ANSI-compliant SQL queries directly.
- **Zero Knowledge:** Browser encrypts all text fields before hitting the API. The backend database holds ciphertext representations of transactions, ensuring data confidentiality. Deterministic encryption is used so database operations like `GROUP BY category` or query matches remain functional.
- **SOLID Principles:** Write explicit, readable, modular code. Avoid complex one-liners. Keep components open for extension but closed for modification.

---

## 📜 License
Private.
