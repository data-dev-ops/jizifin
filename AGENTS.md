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

## 🏗️ 2. ARCHITECTURAL BOUNDARIES

### BACKEND: Python 3.14 / FastAPI / SQLite
- **Validation:** Strict `Pydantic v2` typing for all endpoints.
- **Database:** `aiosqlite`. MUST initialize with `PRAGMA journal_mode=WAL;` and `PRAGMA foreign_keys=ON;`.
- **Querying:** **NO ORMs.** Write native, optimized, ANSI-compliant SQL directly in endpoint logic.
- **Data Types:** Financial values are strictly `INTEGER` (cents). Dates are strictly `TEXT` (YYYY-MM-DD). Convert to decimals (cents/100.0) ONLY at the response/presentation layer.
- **Realtime:** Use native `fastapi.WebSocket`. Fan-out broadcast pattern for `expense_created` events.

### FRONTEND: Svelte / Tailwind / Chart.js
- **Framework:** Vanilla Svelte (`.svelte`). **NO React-isms. NO SvelteKit SSR.**
- **State:** Use native Svelte writable stores (`stores.js`) for reactive data bridging.
- **Styling:** Exclusively utility-first Tailwind CSS. **NO scoped `<style>` blocks** unless strictly required for canvas manipulation.
- **Visualization:** Vanilla `Chart.js` on a `<canvas>` element. Update reactively via `chart.update()` inside WebSocket payloads. **NO D3 or heavy wrapper libraries.**

---

## 🗄️ 3. DATABASE SCHEMA & LOGIC CONSTRAINTS

### Core Tables
1. **`splits`**
   - `category` (TEXT, PK, max 32)
   - `cost_pct_jim` (REAL, <= 100.0)
   - `cost_pct_zina` (REAL, <= 100.0)
   - *Constraint:* `cost_pct_jim + cost_pct_zina = 100.0`
2. **`expenses`**
   - `name` (TEXT, max 96)
   - `cost_cents` (INTEGER)
   - `expense_date` (TEXT, YYYY-MM-DD)
   - `who_paid` (TEXT, CHECK: 'Jim' or 'Zina')
   - `category` (TEXT, FK -> splits.category)
3. **`income`** *(Note: Append-only ledger)*
   - `name` (TEXT, max 96)
   - `amount_cents` (INTEGER)
   - `who` (TEXT, 'Jim' or 'Zina')
   - `category` (TEXT, max 32)
   - `income_date` (TEXT, YYYY-MM-DD)

### Read-Only Pre-Calculated Views (DO NOT RECALCULATE IN APP LOGIC)
- `view_monthly_total`: `[total_amount, expense_count, month]`
- `view_monthly_by_category`: `[category, total_amount, expense_count, cost_pct_jim, cost_pct_zina]`
- `view_monthly_by_payer`: `[who_paid, total_amount, expense_count]`

### Complex Domain Logic (Already Implemented)
- **Paybacks (`/analytics/paybacks`):** Calculates who owes whom by comparing `who_paid` against the `splits` expectation. Includes hardcoded deduction rule: "Combined Fixed" paid by Zina is subtracted from "Apartment" paid by Jim. 
- **Income Salary Carry-forward (`/analytics/income-by-person`):** If a person has no `SALARY` income logged in the current month, the system fetches their most recent `SALARY` entry from prior months and carries it forward automatically.

### Available Features & Locations
- **Analytics:** `frontend/src/lib/AnalyticsSummary.svelte`
- **Budgets:** `frontend/src/lib/BudgetManager.svelte`
- **Expenses:** `frontend/src/lib/ExpenseForm.svelte` & `frontend/src/lib/ExpenseList.svelte`
- **Income:** `frontend/src/lib/IncomeChart.svelte`
- **Paybacks:** `frontend/src/lib/PaybackVisual.svelte`
- **Projects:** `frontend/src/lib/ProjectsTab.svelte`
- **Direct Queries:** `frontend/src/lib/QueryConsole.svelte`
- **Realtime Tracking:** `frontend/src/lib/RealtimeChart.svelte`
- **Recurring Expenses:** `frontend/src/lib/RecurringManager.svelte`
- **Splits:** `frontend/src/lib/SplitManager.svelte`

---

## 📂 4. REPO TOPOLOGY

```text
finance_tracker/
├── docker-compose.yml     # Compose cluster orchestration
├── backend/
│   ├── Dockerfile         # Backend container definition
│   ├── pyproject.toml     # Python dependencies
│   ├── app/
│   │   ├── main.py        # Routes, WS Broadcaster, Analytics logic
│   │   ├── models.py      # Pydantic Schemas
│   │   └── database.py    # aiosqlite connection pool & WAL init
│   └── finance.db         # SQLite instance
└── frontend/
    ├── Dockerfile         # Frontend container definition
    ├── package.json       # Node dependencies
    ├── tailwind.config.js # Tailwind CSS config
    ├── vite.config.js     # Vite bundler config
    └── src/
        ├── App.svelte     # Tab router
        └── lib/
            ├── AnalyticsSummary.svelte
            ├── BudgetManager.svelte
            ├── ExpenseForm.svelte
            ├── ExpenseList.svelte
            ├── IncomeChart.svelte
            ├── PaybackVisual.svelte
            ├── ProjectsTab.svelte
            ├── QueryConsole.svelte
            ├── RealtimeChart.svelte
            ├── RecurringManager.svelte
            ├── SplitManager.svelte
            ├── api.js     # API wrapper
            └── stores.js  # Global State
```

---

## 🚨 5. LLM CODE GENERATION RULES

1. **Minimize Context Overhead:** When refactoring, output ONLY the modified functions or cleanly marked functional diff blocks. Do not rewrite unmodified code.
2. **Flat Composition:** Avoid deep component trees in Svelte.
3. **Zero Deprecation:** Use stable, established APIs. Do not invent beta features for FastAPI or Svelte.
4. **Data Integrity:** Never hallucinate or alter the whole-cent to float-euro conversion math. Database receives whole integers. UI presents decimals.
5. **SOLID Principles:** Adhere rigorously to SOLID principles. Target single-responsibility functions and classes, keeping components open for extension but closed for modification.
6. **Use-of-Understanding:** Produce readable, explicitly clear "use-of-understanding" code. Prioritize clear, maintainable logic and comments over opaque, clever one-liners.
```
