# Backend Technical Documentation

Architecture, database schema, data invariants, and API endpoint reference for the Jizifin backend.

---

## 1. Stack and Runtime Architecture

The Jizifin backend is an asynchronous REST API and WebSocket broadcaster built on Python 3.14, FastAPI, and SQLite via `aiosqlite`.

| Layer | Technology | Version / Spec | Role |
|---|---|---|---|
| Runtime | Python | `3.14+` | Async execution engine |
| Web Framework | [FastAPI](https://fastapi.tiangolo.com/) | `^0.115.0` | Asynchronous routing, OpenAPI generation, dependency injection, WebSockets |
| Server | [Uvicorn](https://www.uvicorn.org/) | `^0.30.0` | ASGI web server |
| Database Engine | [SQLite](https://www.sqlite.org/) via `aiosqlite` | WAL mode | Embedded single-file relational database with WAL journal and foreign key enforcement |
| Schema Validation | [Pydantic v2](https://docs.pydantic.dev/) | `^2.8.0` | Request validation, response serialization, strict type casting |
| Task Scheduler | [APScheduler](https://apscheduler.readthedocs.io/) | `^3.10.4` | Background generation of scheduled recurring expenses |
| Cryptography | `cryptography` | `^43.0.0` | PBKDF2 key derivation and AES-GCM verification for database export/import |
| Test Harness | `pytest` + `pytest-asyncio` + `httpx` | `^8.3.0` | Integration scenarios, debt graph simplification, and schema audits |

### Database Lifecycle and Connection Management

All database interactions flow through `aiosqlite` in `app/database.py`. Every connection is initialized with:
```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
PRAGMA synchronous = NORMAL;
```
- **WAL Mode (Write-Ahead Logging)**: Enables concurrent reads while writes are occurring, preventing database locks under multi-user traffic.
- **Foreign Keys**: Enforced on every connection (`ON UPDATE CASCADE`, `ON DELETE CASCADE`, `ON DELETE SET NULL`).

---

## 2. Core Invariants and Mathematical Algorithms

### 2.1 No ORM Policy (Raw ANSI SQL)
All queries in `app/database.py` and `app/main.py` are written in native ANSI SQL with parameterized query values (`?`) to prevent SQL injection and avoid ORM performance overhead.

### 2.2 Integer Cent Currency Math
All monetary amounts are stored in the database strictly as `INTEGER` cents (`cost_cents`, `amount_cents`, `target_cents`, `balance_cents`). Floating-point currency calculations are prohibited in the database layer. Decimal conversion (`cents / 100.0`) is performed only at presentation and response boundaries.

### 2.3 Largest Remainder Distribution (Hare-Niemeyer)
Split calculations for multi-user expenses use the Largest Remainder algorithm with SHA-256 salted tie-breaking (`allocate_cents_largest_remainder`):

```python
def allocate_cents_largest_remainder(
    total_cents: int,
    percentages: dict[str, float],
    tx_salt: str = ""
) -> dict[str, int]:
    """
    Distributes total_cents among users according to their percentage shares.
    Guarantees that sum(result.values()) == total_cents exactly.
    """
    total_pct = sum(percentages.values())
    if total_pct == 0:
        return {u: 0 for u in percentages}

    # 1. Exact shares and floor quotients
    exact_shares = {u: (total_cents * pct) / total_pct for u, pct in percentages.items()}
    floor_shares = {u: math.floor(share) for u, share in exact_shares.items()}
    remainders = {u: exact_shares[u] - floor_shares[u] for u in percentages}

    # 2. Salted deterministic tie-breaker for identical remainders
    def sort_key(u: str):
        tie_hash = hashlib.sha256(f"{tx_salt}:{u}".encode()).hexdigest()
        return (remainders[u], tie_hash)

    # 3. Distribute remaining cents
    leftover = total_cents - sum(floor_shares.values())
    sorted_users = sorted(percentages.keys(), key=sort_key, reverse=True)
    
    result = floor_shares.copy()
    for i in range(leftover):
        result[sorted_users[i % len(sorted_users)]] += 1

    return result
```

### 2.4 Connected-Component Graph Debt Simplification
The `/analytics/paybacks` endpoint simplifies debt among household members:
1. **Joint Account Exclusion**: Categories assigned to joint accounts are excluded from personal payback calculations.
2. **Personal Categories**: Categories matching `PERSONAL COST`, `LEISURE`, or `GIFT` are attributed 100% to the payer.
3. **Graph Partitioning**: Partitions members into connected components based on transaction split participation.
4. **Greedy Debt Reduction**: In each subgraph, balances are sorted into net debtors and net creditors, greedily settling max transfers until all balances reach 0 cents.

### 2.5 Tag Timeline Boundary Validation
When an expense is assigned a `tag_id`, the backend validates that `tag.start_date <= expense.expense_date <= tag.end_date`. If the expense date falls outside the tag's active window, the API responds with `HTTP 422 Unprocessable Content`.

---

## 3. Database Schema Reference

The database comprises 20 tables and 7 SQL views:

```
Database Schema (SQLite v4)
├── Core Tables
│   ├── app_config                 # System key-value pairs (magic_word validation)
│   ├── users                      # Household members (name [PK, Encrypted], color, is_active)
│   ├── splits                     # Expense categories (category [PK, Encrypted])
│   ├── split_allocations          # Default per-category percentage splits
│   ├── income_categories          # Income category registry
│   ├── projects                   # Target savings goals and completion targets
│   ├── tags                       # Timeline event labels (start_date, end_date)
│   ├── expenses                   # Core transaction ledger (cost_cents, who_paid, category)
│   ├── expense_overrides          # Transaction-specific split percentage overrides
│   ├── income                     # Append-only one-off income records
│   ├── jobs                       # Recurring employment contracts and timelines
│   ├── salary_overrides           # Month-specific salary adjustments (overtime, leave)
│   ├── recurring_expenses         # Recurring expense templates
│   ├── budgets                    # Category monthly spending limits
│   └── settlements                # Historical month lock records
├── Joint Account Tables
│   ├── joint_account              # Singleton joint pool configuration (id=1, balance_cents)
│   ├── joint_account_categories   # Category mappings routed to joint pool
│   ├── joint_account_deposits     # Per-user monthly deposit amounts
│   ├── joint_account_expected_costs # Monthly expected budget per joint category
│   └── joint_account_corrections  # Manual balance deposits and withdrawals
└── SQL Views
    ├── view_monthly_total         # Active month spending aggregate
    ├── view_monthly_by_category   # Active month spending by category
    ├── view_expenses_by_month_category # Historical spending by month and category
    ├── view_monthly_by_payer      # Active month spending grouped by payer
    ├── view_project_summary       # Project cumulative spent cents and counts
    ├── view_tag_totals            # Tag all-time total cents and date ranges
    └── view_joint_account_monthly # Monthly spending in joint categories
```

---

## 4. API Endpoints Catalog

### 4.1 Authentication and System (`/auth`, `/`)

| Method | Path | Request Body | Response | Status | Description |
|---|---|---|---|---|---|
| `GET` | `/` | None | `{"status": "ok", "app": "Jizifin"}` | 200 | Health check probe |
| `POST` | `/auth/login` | `{"passphrase": "..."}` | `{"authenticated": true}` | 200, 401 | Verifies master passphrase against `app_config.magic_word` |
| `POST` | `/auth/salt` | `{"salt": "..."}` | `{"status": "saved"}` | 200 | Sets initial setup magic word on first run |
| `GET` | `/auth/export` | None (Bearer header) | Binary `.db` SQLite stream | 200 | Exports decrypted database file for backup |
| `POST` | `/auth/import` | Multipart SQLite file | `{"status": "imported"}` | 200, 422 | Encrypts and replaces active SQLite database |

### 4.2 Household Members (`/users`)

| Method | Path | Request Body | Response | Status | Description |
|---|---|---|---|---|---|
| `GET` | `/users` | None | `UserResponse[]` | 200 | Lists all household members |
| `POST` | `/users` | `UserCreate` | `UserResponse` | 201, 409 | Creates a new user member |
| `PUT` | `/users/{name}` | `UserUpdate` | `UserResponse` | 200, 404 | Updates user color or active status |
| `DELETE` | `/users/{name}` | None | `{"status": "deleted"}` | 200, 404, 409 | Deactivates or deletes member |

### 4.3 Categories and Splits (`/splits`)

| Method | Path | Request Body | Response | Status | Description |
|---|---|---|---|---|---|
| `GET` | `/splits` | None | `SplitResponse[]` | 200 | Lists categories and default allocations |
| `POST` | `/splits` | `SplitCreate` | `SplitResponse` | 201, 409, 422 | Creates category with percentage allocations (must sum to 100%) |
| `PUT` | `/splits/{category}` | `SplitUpdate` | `SplitResponse` | 200, 404, 422 | Updates default percentage allocations |
| `DELETE` | `/splits/{category}` | None | `{"status": "deleted"}` | 200, 404, 409 | Deletes category if no transactions reference it |

### 4.4 Expenses Ledger (`/expenses`)

| Method | Path | Query / Body | Response | Status | Description |
|---|---|---|---|---|---|
| `GET` | `/expenses` | `?month=YYYY-MM&category=...` | `ExpenseResponse[]` | 200 | Lists expenses matching month and category filters |
| `POST` | `/expenses` | `ExpenseCreate` | `ExpenseResponse` | 201, 422 | Logs an expense with optional tag, project, or split overrides |
| `PUT` | `/expenses/{id}` | `ExpenseUpdate` | `ExpenseResponse` | 200, 404, 422 | Updates an existing expense |
| `DELETE` | `/expenses/{id}` | None | `{"status": "deleted"}` | 200, 404 | Deletes an expense by ID |

### 4.5 Income and Employment Contracts (`/income`, `/jobs`)

| Method | Path | Request Body | Response | Status | Description |
|---|---|---|---|---|---|
| `GET` | `/income` | `?month=YYYY-MM` | `IncomeResponse[]` | 200 | Lists one-off income records |
| `POST` | `/income` | `IncomeCreate` | `IncomeResponse` | 201, 422 | Logs a one-off income entry |
| `DELETE` | `/income/{id}` | None | `{"status": "deleted"}` | 200, 404 | Deletes an income entry |
| `GET` | `/jobs` | None | `JobResponse[]` | 200 | Lists all employment contracts |
| `POST` | `/jobs` | `JobCreate` | `JobResponse` | 201, 422 | Creates an employment stream (monthly/weekly/biweekly/annual) |
| `PUT` | `/jobs/{id}` | `JobUpdate` | `JobResponse` | 200, 404, 422 | Updates contract terms, salary rate, or end date |
| `DELETE` | `/jobs/{id}` | None | `{"status": "deleted"}` | 200, 404 | Deletes a job stream |
| `POST` | `/income/salary-overrides` | `SalaryOverrideIn` | `SalaryOverrideResponse` | 200, 422 | Sets a month-specific salary override for a member |

### 4.6 Joint Account (`/joint-account`)

| Method | Path | Request Body | Response | Status | Description |
|---|---|---|---|---|---|
| `GET` | `/joint-account` | None | `JointConfigResponse` | 200 | Retrieves joint pool balance, expected costs, and deposit shares |
| `PUT` | `/joint-account` | `JointConfigUpdate` | `JointConfigResponse` | 200, 422 | Updates safety margin %, deposit split mode, and category bindings |
| `POST` | `/joint-account/corrections` | `JointCorrectionCreate` | `JointCorrectionResponse` | 201, 422 | Records a manual balance deposit (+) or withdrawal (-) |

### 4.7 Projects, Tags, and Budgets (`/projects`, `/tags`, `/budgets`)

| Method | Path | Request Body | Response | Status | Description |
|---|---|---|---|---|---|
| `GET` | `/projects` | None | `ProjectResponse[]` | 200 | Lists target savings goals and completion estimates |
| `POST` | `/projects` | `ProjectCreate` | `ProjectResponse` | 201, 422 | Creates a savings goal |
| `GET` | `/projects/{id}/settlement` | None | `ProjectSettlementResponse` | 200, 404 | Calculates point-in-time equity and debt settlement transfers |
| `GET` | `/tags` | None | `TagResponse[]` | 200 | Lists tags and date range windows |
| `POST` | `/tags` | `TagCreate` | `TagResponse` | 201, 422 | Creates a tag with optional start/end date constraints |
| `GET` | `/budgets` | `?month=YYYY-MM` | `BudgetStatusRow[]` | 200 | Compares actual spending against category monthly limits |
| `POST` | `/budgets` | `BudgetCreate` | `BudgetResponse` | 201, 422 | Sets category spending limit in cents |

### 4.8 Analytics and Settlements (`/analytics`, `/settlements`)

| Method | Path | Query Parameters | Response | Description |
|---|---|---|---|---|
| `GET` | `/analytics/monthly-total` | None | `MonthlyTotal` | Returns total amount spent and count for active month |
| `GET` | `/analytics/by-category` | None | `MonthlyCategoryRow[]` | Returns active month spending grouped by category |
| `GET` | `/analytics/by-payer` | None | `MonthlyPayerRow[]` | Returns active month spending grouped by payer |
| `GET` | `/analytics/paybacks` | `?month=YYYY-MM` | `PaybackSummary` | Computes simplified debt transfers between household members |
| `GET` | `/analytics/income-by-person` | `?month=YYYY-MM` | `IncomeByPersonRow[]` | Returns total monthly income per person from jobs and overrides |
| `GET` | `/settlements` | None | `SettlementRow[]` | Lists settled and locked historical months |
| `POST` | `/settlements` | `SettlementCreate` | `SettlementRow` | Locks a calendar month after net balance transfer |

### 4.9 Real-Time WebSockets (`/ws/finance`)

- **Path**: `WS /ws/finance`
- **Protocol**: Raw JSON frames.
- **Fan-Out Dispatcher**: When write mutations occur (`POST /expenses`, `PUT /expenses`, `DELETE /expenses`), the backend broadcasts:
  ```json
  {
    "event": "expense_created",
    "expense_id": 142,
    "month": "2026-08"
  }
  ```
  Connected clients receive the message and refresh analytics stores in real time.

---

## 5. Automated Testing and CI Execution

The test suite runs with `pytest` inside Docker:

```bash
docker run --rm \
  -v $(pwd)/backend/app:/app/app \
  -v $(pwd)/backend/tests:/app/tests \
  jizifin-backend-test pytest
```

Test coverage includes:
- Multi-user allocations and Hare-Niemeyer cent distribution (`test_split_math.py`)
- Disjoint subgraph graph debt simplification (`test_debt_graph.py`)
- Tag active window date validation (`test_tags_validation.py`)
- SQLite WAL mode concurrency and foreign key cascade rules (`test_database.py`)
- API integration endpoints and auth magic word verification (`test_api.py`)
