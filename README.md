# Personal Finance Tracker

A monorepo personal finance tracking application designed for managing shared expenses, incomes, budgets, and paybacks. Built with a Python/FastAPI backend and a Vanilla Svelte frontend.

## 🚀 Features

- **Dashboard & Analytics:** Comprehensive overview of balances, monthly totals, and category breakdowns.
- **Expense & Income Tracking:** Log shared and personal expenses, and keep an append-only ledger for income (including salary carry-forward).
- **Split Management:** Configure exact percentage splits (e.g., Jim 60% / Zina 40%) for different expense categories.
- **Payback Calculator:** Automatically calculates who owes whom based on who paid and the expected split ratio. 
- **Realtime Updates:** WebSocket integration for live updates across clients when new expenses are added.
- **Budgeting & Projects:** Track ongoing budgets and project-specific expenses.
- **Recurring Expenses:** Manage and track subscriptions and fixed monthly costs.

## 🛠 Tech Stack

**Backend:**
- Python 3.14
- FastAPI
- SQLite (`aiosqlite`) with WAL mode
- `uv` for dependency management

**Frontend:**
- Vanilla Svelte (No SvelteKit SSR)
- Tailwind CSS
- Chart.js (Native Canvas)
- Vite

## 📦 Installation & Setup

### Option 1: Docker Compose (Recommended)
You can run the entire stack (frontend and backend) using Docker Compose.

```bash
# Build and start the containers in detached mode
docker compose up --build -d
```

### Option 2: Local Development

**Prerequisites:**
- Python 3.14
- `uv` package manager (`uv 0.11.26` or compatible)
- Node.js (tested with `v24.18.0`)
- `npm`

**1. Start the Backend:**
```bash
cd backend
uv run uvicorn app.main:app --reload --port 8000
```

**2. Start the Frontend:**
```bash
cd frontend
npm install
npm run dev
```

## 🏗 Architecture & Design Principles

- **Database Constraint:** Values are strictly stored as integers (cents) to avoid floating-point math errors. They are only converted to decimals (`cents / 100.0`) at the presentation layer.
- **No ORMs:** The backend uses native, optimized ANSI-compliant SQL queries directly in the endpoint logic.
- **State Management:** The frontend relies on native Svelte writable stores (`stores.js`) for reactive data bridging.
- **SOLID & Use-of-Understanding:** Code should be explicit, clear, and open for extension but closed for modification. Avoid opaque one-liners in favor of readable logic.

## 📜 License
Private.
