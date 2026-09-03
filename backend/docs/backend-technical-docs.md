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
All queries in `app/database.py` and `app/main.py` are written in native, ANSI-compliant SQL with parameterized query values (`?`) to prevent SQL injection and avoid ORM performance overhead.

### 2.2 Integer Cent Currency Math
All monetary amounts are stored in the database strictly as `INTEGER` cents (`cost_cents`, `amount_cents`, `target_cents`, `balance_cents`). Floating-point currency calculations are prohibited in the database layer. Decimal conversion (`cents / 100.0`) is performed exclusively at presentation and response boundaries.

### 2.3 Signed Largest Remainder Distribution (Hare-Niemeyer)
Split calculations for multi-user expenses, refunds, and credit memos use the Largest Remainder algorithm with mathematical floor and SHA-256 salted tie-breaking (`allocate_cents_largest_remainder`):

```python
def allocate_cents_largest_remainder(
    total_cents: int,
    percentages: dict[str, float],
    tx_salt: str = ""
) -> dict[str, int]:
    """
    Distributes total_cents among users according to their percentage shares.
    Guarantees that sum(result.values()) == total_cents exactly.
    Uses math.floor() to maintain zero-sum invariants on negative refunds.
    """
    total_pct = sum(percentages.values())
    if total_pct == 0:
        return {u: 0 for u in percentages}

    # 1. Exact shares and floor quotients
    exact_shares = {u: (total_cents * pct) / total_pct for u, pct in percentages.items()}
    floor_shares = {u: math.floor(share) for u, share in exact_shares.items()}
    remainders = {u: exact_shares[u] - floor_shares[u] for u in percentages}

    # 2. Deterministic salted tie-breaker for identical remainders
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
The `/analytics/paybacks` endpoint computes minimal debt settlements between household members:
1. **Joint Account Exclusion**: Direct joint account transactions (`is_joint = 1`) and categories assigned to joint accounts are excluded from personal payback calculations.
2. **Personal Categories**: Categories matching `PERSONAL COST`, `LEISURE`, or `GIFT` are attributed 100% to the payer.
3. **Graph Partitioning**: Partitions members into disjoint connected components based on transaction split participation, ensuring debts do not cross outside participating groups (e.g. independent couples).
4. **Greedy Debt Reduction**: In each subgraph, balances are sorted into net debtors and net creditors, greedily settling max transfers until all balances reach 0 cents.

### 2.5 Tag Timeline Boundary Validation
When an expense is assigned a `tag_id`, the backend validates that `tag.start_date <= expense.expense_date <= tag.end_date`. Violations return `HTTP 422 Unprocessable Content`. Attempting to shrink a tag window such that existing assigned expenses would be orphaned outside proposed dates is rejected with `HTTP 422`.

### 2.6 Slowly Changing Dimension Type 2 (SCD2) Split Agreements
Category allocations support date-bounded SCD2 split agreements (`split_agreements`, `split_allocations`):
- **Baseline Agreement**: Ongoing agreement (`end_date IS NULL`).
- **Date-Bounded Overrides**: Agreements with defined `start_date` and `end_date` taking precedence over the baseline agreement for expenses falling within their active window.
- **Precedence Hierarchy**: Expense overrides (`expense_overrides`) > SCD2 Split Agreements (`split_agreements`) > Baseline Allocations (`split_allocations`) > Equal split.

### 2.7 Multi-Joint Account Isolation & Project Equity Decomposition
- **Multi-Joint Accounts**: Isolated joint accounts (`joint_accounts`, `joint_account_members`) manage independent cash balances, safety margins, monthly deposit schedules (`joint_account_deposits`), monthly execution logs (`joint_account_monthly_deposits`), and signed balance corrections (`joint_account_corrections`).
- **Project Equity Settlement (`GET /projects/{id}/settlement`)**: Decomposes joint-account funded project expenses into co-owners' equity proportions based on their scheduled monthly deposit shares, evaluating effective funding against assigned liability.

### 2.8 Category Renaming Cascade & Deletion Protection
- Renaming an expense category cascades across referencing tables (`ON UPDATE CASCADE`).
- Deleting a category is protected: if any expense, recurring template, budget, or agreement references the category, deletion is blocked with `HTTP 409 Conflict`.

### 2.9 Deterministic AES-GCM Indexing & Querying Rules
Ciphertext is encoded to Base64URL without padding using PBKDF2 (100k iterations, static salt `"jizifin-salt-pbkdf2"`, static IV `"jizifin-cryp"`):
- **Valid on Encrypted Columns**: Exact matches (`col = ?`, `IN (...)`), equality joins (`ON a.col = b.col`), `GROUP BY`, and foreign key cascades.
- **FORBIDDEN on Encrypted Columns**: Range filters (`<`, `>`, `BETWEEN`), `LIKE` wildcards, and text collation `ORDER BY`.

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

## 3. Database Schema Reference

The database comprises **24 tables** and **9 SQL views** (defined in `app/database.py`):

```
Database Schema (SQLite)
├── Core Tables (16)
│   ├── app_config                 # System key-value pairs (magic_word validation)
│   ├── users                      # Household members (name [PK, Encrypted], color, is_active)
│   ├── splits                     # Expense categories (category [PK, Encrypted])
│   ├── split_allocations          # Default per-category percentage splits
│   ├── income_categories          # Income category registry (historical survival)
│   ├── projects                   # Target budget milestone targets
│   ├── project_users              # Multi-user project members (project_id, user_name)
│   ├── tags                       # Event & label tags with timeline window (start_date, end_date)
│   ├── expenses                   # Core transaction ledger (cost_cents, who_paid, category)
│   ├── expense_overrides          # Per-expense custom split percentage overrides
│   ├── income                     # Append-only one-off income records
│   ├── jobs                       # Recurring employment contracts and timelines
│   ├── salary_overrides           # Month-specific salary adjustments (overtime, leave)
│   ├── recurring_expenses         # Automated expense templates
│   ├── budgets                    # Monthly category spending limits
│   └── settlements                # Monthly reconciliation & lock log
├── SCD2 Category Split Timelines (2)
│   ├── split_agreements           # SCD2 category split timelines (id, category, start_date, end_date)
│   └── split_allocations          # User percentage shares per agreement
├── Joint Account Tables (7)
│   ├── joint_accounts             # Multi-joint account registry (id [PK], name, balance_cents)
│   ├── joint_account_members      # Joint account participant roster (account_id, user_name)
│   ├── joint_account              # Singleton backwards-compat table (id = 1)
│   ├── joint_account_categories   # Categories paid from joint accounts
│   ├── joint_account_deposits     # Monthly user deposit schedules
│   ├── joint_account_monthly_deposits # Deposit execution & payment log (month, user, is_paid)
│   ├── joint_account_expected_costs   # Expected monthly cost per category
│   └── joint_account_corrections  # Signed balance corrections (+/-)
└── SQL Views (9)
    ├── view_monthly_total         # Active month spending aggregate
    ├── view_monthly_by_category   # Active month spending by category
    ├── view_expenses_by_month_category # Historical spending by month and category
    ├── view_monthly_by_payer      # Active month spending grouped by payer
    ├── view_project_summary       # Project cumulative spent cents and counts
    ├── view_tag_totals            # Tag all-time total cents and date ranges
    ├── view_joint_account_monthly # Monthly spending in joint categories
    ├── view_current_split_allocations # Active category split allocations resolved for current date
    └── view_split_agreements_active   # Active split agreement timelines ordered by start date
```

---

## 4. API Endpoints Catalog

### 4.1 Authentication and System (`/auth`, `/`)

| Method | Path | Request Body | Response | Status | Description |
|---|---|---|---|---|---|
| `GET` | `/` | None | `{"status": "ok", "app": "Jizifin"}` | 200 | Health check probe |
| `GET` | `/auth/status` | None | `AuthStatusResponse` | 200 | Checks if database has been initialized with a master passphrase |
| `POST` | `/auth/login` | `{"passphrase": "..."}` | `AuthLoginResponse` | 200, 401 | Verifies master passphrase against `app_config.magic_word` |
| `POST` | `/auth/salt` | `{"salt": "..."}` | `{"status": "saved"}` | 200 | Sets initial setup magic word on first run |
| `POST` | `/auth/logout` | None | `{"status": "logged_out"}` | 200 | Clears active session |
| `POST` | `/auth/reset` | None | `{"status": "reset"}` | 200 | Clears authentication configuration |
| `POST` | `/auth/export` | None (Bearer token) | Binary `.db` SQLite stream | 200 | Decrypts and exports database file for backup |
| `POST` | `/auth/import` | Multipart SQLite file | `{"status": "imported"}` | 200, 422 | Re-encrypts and replaces active SQLite database with migration |

### 4.2 Household Members (`/users`)

| Method | Path | Request Body | Response | Status | Description |
|---|---|---|---|---|---|
| `GET` | `/users` | `?include_inactive=bool` | `UserResponse[]` | 200 | Lists household members |
| `POST` | `/users` | `UserCreate` | `UserResponse` | 201, 409 | Creates a new user member |
| `PUT` | `/users/{name}` | `UserUpdate` | `UserResponse` | 200, 404 | Updates user color or active status |
| `DELETE` | `/users/{name}` | None | None | 204, 404, 409 | Deactivates or deletes member |

### 4.3 Categories, Allocations, and SCD2 Agreements (`/splits`)

| Method | Path | Request Body | Response | Status | Description |
|---|---|---|---|---|---|
| `GET` | `/splits` | None | `SplitResponse[]` | 200 | Lists categories, allocations, and active agreements |
| `POST` | `/splits` | `SplitCreate` | `SplitResponse` | 201, 409, 422 | Creates category with allocations (must sum to 100%) |
| `PUT` | `/splits/{category}` | `SplitUpdate` | `SplitResponse` | 200, 404, 422 | Renames category (`ON UPDATE CASCADE`) or updates default allocations |
| `DELETE` | `/splits/{category}` | None | None | 204, 404, 409 | Deletes category (blocked with 409 if referenced) |
| `POST` | `/splits/{category}/agreements` | `SplitAgreementCreate` | `SplitAgreementResponse` | 201, 422 | Creates date-bounded SCD2 split agreement |
| `PUT` | `/splits/agreements/{id}` | `SplitAgreementUpdate` | `SplitAgreementResponse` | 200, 404, 422 | Updates SCD2 split agreement dates or allocations |
| `DELETE` | `/splits/agreements/{id}` | None | None | 204, 404 | Deletes SCD2 split agreement |

### 4.4 Expenses Ledger (`/expenses`)

| Method | Path | Query / Body | Response | Status | Description |
|---|---|---|---|---|---|
| `GET` | `/expenses` | `?month=YYYY-MM&category=...&who_paid=...&is_joint=...` | `ExpenseResponse[]` | 200 | Lists expenses matching filter criteria |
| `POST` | `/expenses` | `ExpenseCreate` | `ExpenseResponse` | 201, 422 | Logs expense with tag validation, project linking, or split overrides |
| `PUT` | `/expenses/{id}` | `ExpenseUpdate` | `ExpenseResponse` | 200, 404, 422 | Updates existing expense (enforcing tag boundaries) |
| `DELETE` | `/expenses/{id}` | None | None | 204, 404 | Deletes an expense by ID |

### 4.5 Income and Employment Contracts (`/income`, `/jobs`)

| Method | Path | Request Body | Response | Status | Description |
|---|---|---|---|---|---|
| `GET` | `/income` | `?month=YYYY-MM` | `IncomeResponse[]` | 200 | Lists one-off income records |
| `POST` | `/income` | `list[IncomeCreate]` | `list[IncomeResponse]` | 201, 422 | Logs one-off income entries in batch |
| `DELETE` | `/income/{id}` | None | None | 204, 404 | Deletes an income entry |
| `GET` | `/income-categories` | None | `IncomeCategoryResponse[]` | 200 | Lists income categories |
| `POST` | `/income-categories` | `IncomeCategoryCreate` | `IncomeCategoryResponse` | 201, 409 | Adds an income category |
| `PUT` | `/income-categories/{cat}` | `IncomeCategoryUpdate` | `IncomeCategoryResponse` | 200, 404 | Renames income category |
| `DELETE` | `/income-categories/{cat}` | None | None | 204, 404, 409 | Deletes an income category |
| `GET` | `/jobs` | None | `JobResponse[]` | 200 | Lists all employment contracts |
| `POST` | `/jobs` | `JobCreate` | `JobResponse` | 201, 422 | Creates employment contract (`monthly`/`weekly`/`biweekly`/`annual`) |
| `PUT` | `/jobs/{id}` | `JobUpdate` | `JobResponse` | 200, 404, 422 | Updates contract terms, salary rate, or end date |
| `DELETE` | `/jobs/{id}` | None | None | 204, 404 | Deletes a job stream |
| `GET` | `/income/salary-overrides` | `?month=YYYY-MM&user_name=...` | `SalaryOverrideResponse[]` | 200 | Lists month-specific salary adjustments |
| `PUT` | `/income/salary-overrides` | `SalaryOverrideIn` | `SalaryOverrideResponse` | 200, 404, 422 | Upserts month-specific salary override |
| `DELETE` | `/income/salary-overrides/{user}/{month}` | None | None | 204, 404 | Deletes month-specific salary override |
| `GET` | `/income/latest-salary` | `?salary_cat=...&month=...` | `LatestSalaryRow[]` | 200 | Evaluates effective base salaries across contracts and overrides |

### 4.6 Multi-Joint Accounts (`/joint-accounts`, `/joint-account`)

| Method | Path | Request Body | Response | Status | Description |
|---|---|---|---|---|---|
| `GET` | `/joint-accounts` | None | `JointAccountResponse[]` | 200 | Lists all configured joint accounts and their member rosters |
| `POST` | `/joint-accounts` | `JointAccountCreate` | `JointAccountResponse` | 201, 422 | Creates new joint account with specific members |
| `GET` | `/joint-accounts/{id}` | None | `JointAccountResponse` | 200, 404 | Gets specific joint account config |
| `PATCH` | `/joint-accounts/{id}` | `JointAccountUpdate` | `JointAccountResponse` | 200, 404 | Updates joint account balance, members, or safety margin |
| `DELETE` | `/joint-accounts/{id}` | None | None | 204, 404 | Deletes joint account and associated allocations |
| `GET` | `/joint-account/categories` | `?account_id=...` | `string[]` | 200 | Lists categories mapped to joint account |
| `POST` | `/joint-account/categories` | `{"category": "...", "account_id": 1}` | `{"category": "...", "account_id": 1}` | 201, 409 | Maps category to joint account |
| `DELETE` | `/joint-account/categories/{cat}` | `?account_id=...` | None | 204, 404 | Unlinks category from joint account |
| `GET` | `/joint-account/deposits` | `?account_id=...` | `JointAccountDepositResponse[]` | 200 | Lists member monthly deposit schedules |
| `PUT` | `/joint-account/deposits` | `list[JointAccountDepositCreate]` | `JointAccountDepositResponse[]` | 200, 422 | Replaces monthly deposit schedules |
| `GET` | `/joint-account/expected-costs` | `?account_id=...` | `JointAccountExpectedCostResponse[]` | 200 | Lists expected monthly costs per category |
| `PUT` | `/joint-account/expected-costs` | `list[JointAccountExpectedCostCreate]` | `JointAccountExpectedCostResponse[]` | 200, 422 | Replaces expected monthly costs |
| `GET` | `/joint-account/corrections` | `?account_id=...` | `JointAccountCorrectionResponse[]` | 200 | Lists signed balance corrections |
| `POST` | `/joint-account/corrections` | `JointAccountCorrectionCreate` | `JointAccountCorrectionResponse` | 201, 422 | Logs balance correction (+ top-up, - withdrawal) |
| `DELETE` | `/joint-account/corrections/{id}` | None | None | 204, 404 | Deletes correction and reverses balance effect |
| `GET` | `/joint-account/monthly-deposits` | `?month=YYYY-MM&account_id=...` | `JointAccountMonthlyDepositRow[]` | 200 | Gets deposit execution tracking (paid, overdue, pending) |
| `POST` | `/joint-account/monthly-deposits` | `JointAccountMonthlyDepositUpdate` | `JointAccountMonthlyDepositRow` | 200, 422 | Updates deposit payment status and adjusts cash balance |
| `GET` | `/joint-account/dashboard` | `?month=YYYY-MM&account_id=...` | `JointAccountDashboardResponse` | 200, 404 | Returns actuals vs expected costs and deposit status |
| `POST` | `/joint-account/settle` | `{"mode": "...", "month": "..."}` | `dict` | 200, 400 | Computes direct payment or adjusts next month's deposits |

### 4.7 Projects, Tags, Budgets, and Recurring Templates

| Method | Path | Request Body | Response | Status | Description |
|---|---|---|---|---|---|
| `GET` | `/projects` | None | `ProjectResponse[]` | 200 | Lists target budget milestones, member lists, and completion estimates |
| `POST` | `/projects` | `ProjectCreate` | `ProjectResponse` | 201, 422 | Creates budget milestone with user membership roster |
| `PUT` | `/projects/{id}` | `ProjectUpdate` | `ProjectResponse` | 200, 404 | Updates milestone target, date, or members |
| `DELETE` | `/projects/{id}` | None | None | 204, 404 | Deletes project |
| `GET` | `/projects/{id}/settlement` | None | `ProjectSettlementResponse` | 200, 404 | Evaluates point-in-time participant equity and debt transfers |
| `GET` | `/tags` | None | `TagTotalRow[]` | 200 | Lists tags, date windows, and cumulative spend |
| `POST` | `/tags` | `TagCreate` | `TagResponse` | 201, 422 | Creates timeline tag with active window constraints |
| `PUT` | `/tags/{id}` | `TagUpdate` | `TagResponse` | 200, 404, 422 | Updates tag window (validates assigned expenses remain within window) |
| `DELETE` | `/tags/{id}` | None | None | 204, 404 | Deletes tag (unassigns from linked expenses) |
| `GET` | `/analytics/tags/{id}` | None | `TagDetailResponse` | 200, 404 | Detailed breakdown of expenses under tag |
| `GET` | `/budgets` | None | `BudgetResponse[]` | 200 | Lists configured monthly category budgets |
| `POST` | `/budgets` | `BudgetCreate` | `BudgetResponse` | 201, 422 | Sets category spending limit in cents |
| `DELETE` | `/budgets/{cat}/{month}` | None | None | 204, 404 | Deletes category budget limit |
| `GET` | `/analytics/budgets` | `?month=YYYY-MM` | `BudgetStatusRow[]` | 200 | Compares actual spending against monthly limits |
| `GET` | `/recurring` | `?month=YYYY-MM` | `RecurringResponse[]` | 200 | Lists recurring expense templates and month occurrences |
| `POST` | `/recurring` | `RecurringCreate` | `RecurringResponse` | 201, 422 | Creates recurring expense template |
| `PUT` | `/recurring/{id}` | `RecurringUpdate` | `RecurringResponse` | 200, 404, 422 | Updates recurring expense template |
| `DELETE` | `/recurring/{id}` | None | None | 204, 404 | Deletes recurring expense template |
| `GET` | `/analytics/recurring` | `?month=YYYY-MM` | `RecurringAnalyticsSummary` | 200 | Forecasts recurring commitments for target month |

### 4.8 Analytics and Settlements (`/analytics`, `/settlements`, `/query`)

| Method | Path | Query Parameters | Response | Description |
|---|---|---|---|---|
| `GET` | `/analytics/monthly-total` | None | `MonthlyTotal` | Returns total amount spent and count for active month |
| `GET` | `/analytics/by-category` | None | `MonthlyCategoryRow[]` | Returns active month spending grouped by category |
| `GET` | `/analytics/by-payer` | None | `MonthlyPayerRow[]` | Returns active month spending grouped by payer |
| `GET` | `/analytics/paybacks` | `?month=...&dynamic_income_cats=...` | `PaybackSummary` | Computes simplified debt transfers between household members |
| `GET` | `/analytics/income-by-person` | `?salary_cat=...&month=...` | `IncomeByPersonRow[]` | Total monthly income per person from jobs, bonuses, and overrides |
| `GET` | `/settlements` | None | `SettlementResponse[]` | Lists settled and locked historical months |
| `POST` | `/settlements` | `SettlementCreate` | `SettlementResponse` | Locks a calendar month after net balance transfer |
| `POST` | `/query` | `QueryRequest` (`sql`) | `QueryResponse` | Developer SQL console executing queries up to 50 rows |

### 4.9 Real-Time WebSockets (`/ws/finance`)

- **Path**: `WS /ws/finance`
- **Protocol**: Raw JSON frames over WebSocket.
- **Fan-Out Dispatcher**: When write mutations occur (`POST /expenses`, `PUT /expenses`, `DELETE /expenses`, `POST /joint-accounts`, etc.), the backend broadcasts:
  ```json
  {
    "event": "expense_created",
    "expense_id": 142,
    "month": "2026-08"
  }
  ```
  Connected clients receive the message and refresh analytics stores in real time.

---

## 5. Automated Testing and Test Harness

The backend test suite contains **339 tests across 20 test files** enforcing zero regression, integer cent precision, and cryptographic integrity:

### Test Execution Commands
- **Primary Host CLI**:
  ```bash
  uv run --directory backend pytest
  ```
- **Pre-Built Docker Container Fallback**:
  ```bash
  docker run --rm \
    -v $(pwd)/backend/app:/app/app \
    -v $(pwd)/backend/tests:/app/tests \
    jizifin-backend-test pytest
  ```

### Shared Integration Test Database (`backend/tests/test.db`)
- **Deterministic Generator (`generate_test_db.py`)**: Reproducibly builds the git-tracked SQLite database `backend/tests/test.db`.
- **Pre-Seeded Synthetic Household**:
  - 5 household members: Couple 1 (`Alice` & `Bob`), Couple 2 (`Charlie` & `Dave`), Single Adult (`Eve`).
  - 2 Isolated Joint Accounts: `Couple AB Joint` (60/40 equity) and `Couple CD Joint` (60/40 equity).
  - 5 Conflict-Free Monthly Scenario Partitions:
    * `2026-06`: Scenario 2 Solar Project & Tag 1 (`tag:solar-phase-1`) timeline bounds
    * `2026-07`: Scenario 1 Isolated Joint Accounts & Basis-point proportional income splits
    * `2026-08`: Scenarios 4 & 5 Overlapping SCD2 category overrides & out-of-pocket vs joint funding
    * `2026-09`: Scenario 4 Post-override baseline fallback
    * `2026-10`: Scenario 3 Automated dynamic income recalibration & multi-category cascades
- **Zero Sensitive Data**: All user names, salts, and passphrases in the test suite use synthetic standard identities (`Alice`, `Bob`, `test-master-passphrase`).
- **Integration Fixtures (`conftest.py`)**: `integration_db` clones `test.db` to an isolated tempfile per test, giving integration tests zero cross-test pollution and maximum execution speed.
