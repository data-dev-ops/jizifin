---
name: jizifin-query-optimizer
description: Immediately inspects, diagnoses, and optimizes SQLite query execution plans, indexes, views, and analytics for the jizifin project, adhering to deterministic AES-GCM and zero-regression constraints.
---

# 🚀 Jizifin Query & Execution Plan Optimizer

Use this skill whenever you write or modify SQL queries, database schemas, analytical endpoints, or views in the Jizifin codebase.

It incorporates the architectural directives, cryptographic split, and zero-regression rules defined in [`AGENTS.md`](file:///home/jim/Documents/jizifin/AGENTS.md) and [`README.md`](file:///home/jim/Documents/jizifin/README.md).

---

## 🎯 Core Objectives

1. **Eliminate Full Table Scans (`SCAN TABLE`)**: Ensure high-traffic ledger tables (`expenses`, `income`, `jobs`) are queried via indexed lookups.
2. **Eliminate Ephemeral Temporary B-Trees (`USE TEMP B-TREE FOR ORDER BY / GROUP BY`)**: Supply directional indexes matching query sort orders.
3. **Eliminate On-The-Fly Automatic Indexes (`SEARCH ... USING AUTOMATIC COVERING INDEX`)**: SQLite does not auto-index foreign keys. Explicitly index FKs (`project_id`, `tag_id`, `who_paid`, `category`).
4. **Preserve Deterministic Zero-Knowledge Encryption Semantics**: Enforce exact-match indexing on Base64URL ciphertexts while keeping range/sort indexing strictly on plaintext columns.
5. **Zero-Regression Verification**: Every query or index change must pass full Pytest and integration test suites.

---

## ⚡ Fast Diagnostic Protocol

### 1. Run Automated Analyzer Script
From the repository root:
```bash
uv run --directory backend python ../.agents/skills/jizifin-query-optimizer/scripts/analyze_query_plan.py
```
To analyze a specific SQL query:
```bash
uv run --directory backend python ../.agents/skills/jizifin-query-optimizer/scripts/analyze_query_plan.py "SELECT * FROM expenses WHERE expense_date >= '2025-01-01' ORDER BY expense_date DESC"
```

### 2. Fast Python One-Liner (Isolated in-memory test)
```bash
uv run --directory backend python -c "
import asyncio, aiosqlite
from app.database import _init_db_schema

async def check(sql):
    async with aiosqlite.connect(':memory:') as db:
        await _init_db_schema(db)
        async with db.execute(f'EXPLAIN QUERY PLAN {sql}') as cur:
            for row in await cur.fetchall():
                print(dict(row)['detail'])

asyncio.run(check('''SELECT * FROM view_monthly_by_category'''))
"
```

---

## 🔍 SQLite Execution Plan (EQP) Warning Matrix

| EQP Output Pattern | Root Cause in Jizifin | Actionable Remedy |
| :--- | :--- | :--- |
| `SCAN TABLE expenses` | Missing index on filter or join column (`expense_date`, `who_paid`, `category`, `project_id`, `tag_id`) | Add targeted single or composite index in `_init_db_schema()` |
| `USE TEMP B-TREE FOR ORDER BY` | Query sorts by columns not matching the leading edges of an available B-tree index | Add index matching sort direction, e.g. `(expense_date DESC, id DESC)` |
| `USE TEMP B-TREE FOR GROUP BY` | Aggregation grouping lacks covering or prefix index | Create composite index covering grouping + aggregation fields |
| `SEARCH ... USING AUTOMATIC COVERING INDEX` | Missing index on a foreign key used in a `JOIN` or `LEFT JOIN` (e.g. `project_id`, `tag_id`) | SQLite is building an in-memory index on every run. Add explicit foreign key index |
| `BLOOM FILTER ON e (...)` | SQLite generates a bloom filter to prune scans because the join column lacks an index | Add index on the joined foreign key column |
| `CO-ROUTINE view_<name>` | View subquery executed as a co-routine; inner query performs scans | Optimize the underlying view definition or add indexes to underlying tables |

---

## 🔒 Cryptographic Indexing Constraints (Deterministic AES-GCM)

As specified in `AGENTS.md` (Section 2) and `README.md` (Section 27):
Jizifin uses deterministic AES-GCM encryption with a static IV (`[106, 105, 122, 105, 102, 105, 110, 45, 99, 114, 121, 112]`).

### Column Classification

| Category | Columns | Allowed Index Operations | Forbidden / Invalid Operations |
| :--- | :--- | :--- | :--- |
| **Encrypted (Base64URL)** | `users.name`, `splits.category`, `income_categories.category`, `projects.name`, `expenses.name`, `expenses.who_paid`, `expenses.category`, `expense_overrides.user_name`, `income.name`, `income.who`, `income.category`, `recurring_expenses.name`, `recurring_expenses.who_paid`, `recurring_expenses.category`, `budgets.category`, `split_allocations.category`, `split_allocations.user_name`, `tags.name`, `tags.description`, `joint_account.name`, `joint_account_deposits.user_name`, `joint_account_corrections.note`, `jobs.name`, `jobs.who`, `jobs.notes`, `salary_overrides.user_name`, `salary_overrides.note` | Exact match (`col = ?`), `IN (...)`, `GROUP BY col`, equality join (`ON a.col = b.col`), `COUNT(col)` | Range scans (`<`, `>`, `BETWEEN`), `LIKE '%term%'`, text collation order. |
| **Plaintext** | Amounts (`cost_cents`, `amount_cents`, `target_cents`), dates (`expense_date`, `income_date`, `target_date`, `start_date`, `end_date`), IDs (`id`, `project_id`, `tag_id`, `agreement_id`), flags (`is_joint`, `is_active`, `allow_subcategories`), `settlements` (`month`, `settled_at`, `net_balance_transferred_cents`) | Range queries (`col >= ? AND col <= ?`), sort ordering (`ORDER BY col DESC`), mathematical aggregations (`SUM(cost_cents)`) | None |

### Date Query Optimization Rule (SARGability)
- ❌ **Anti-pattern**: `WHERE strftime('%Y-%m', expense_date) = strftime('%Y-%m', 'now')`
  - Calling a function on the column prevents SQLite from using an index on `expense_date`.
- ✅ **Optimized**: `WHERE expense_date >= 'YYYY-MM-01' AND expense_date <= 'YYYY-MM-31'`
  - Allows SQLite to perform an index range scan using `idx_expenses_date`.

---

## 🗄️ Canonical Jizifin Index Catalog

The following indexes eliminate table scans and temp B-trees across all 7 views and core endpoints:

```sql
-- 1. expenses: Timeline sorting & date range queries
CREATE INDEX IF NOT EXISTS idx_expenses_date_id ON expenses (expense_date DESC, id DESC);

-- 2. expenses: Member spending & payback calculations
CREATE INDEX IF NOT EXISTS idx_expenses_who_date ON expenses (who_paid, expense_date DESC);

-- 3. expenses: Category spending & budget limits
CREATE INDEX IF NOT EXISTS idx_expenses_category_date ON expenses (category, expense_date DESC);

-- 4. expenses: Foreign key indexes for projects and tags (eliminates AUTOMATIC COVERING INDEX)
CREATE INDEX IF NOT EXISTS idx_expenses_project_id ON expenses (project_id) WHERE project_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_expenses_tag_id ON expenses (tag_id) WHERE tag_id IS NOT NULL;

-- 5. expenses: Joint account ledger queries
CREATE INDEX IF NOT EXISTS idx_expenses_joint_date ON expenses (is_joint, expense_date DESC);

-- 6. income: Ledger history (already present in schema)
CREATE INDEX IF NOT EXISTS idx_income_who_date ON income (who, income_date DESC);

-- 7. jobs: Active employment streams (already present in schema)
CREATE INDEX IF NOT EXISTS idx_jobs_who_dates ON jobs (who, start_date, end_date);

-- 8. split_agreements & allocations: (already present in schema)
CREATE INDEX IF NOT EXISTS idx_split_agreements_cat_dates ON split_agreements (category, start_date, end_date);
CREATE INDEX IF NOT EXISTS idx_split_allocations_agreement ON split_allocations (agreement_id);
CREATE INDEX IF NOT EXISTS idx_split_allocations_category ON split_allocations (category);
```

---

## 📊 Database Views Optimization Status

| View | Purpose | Optimization Notes |
| :--- | :--- | :--- |
| `view_monthly_total` | Total month spending & count | Scans `expenses`. Accelerated by date range index on `expense_date`. |
| `view_monthly_by_category` | Category totals for current month | Uses `category` and `cost_cents`. Uses `idx_expenses_category_date`. |
| `view_expenses_by_month_category` | Monthly spending by month & category | Grouped by `strftime('%Y-%m', expense_date), category`. |
| `view_monthly_by_payer` | Member totals for current month | Uses `who_paid`. Uses `idx_expenses_who_date`. |
| `view_project_summary` | Total spent cents per project | `LEFT JOIN expenses e ON e.project_id = p.id`. Requires `idx_expenses_project_id`. |
| `view_tag_totals` | All-time tag spending aggregates | `LEFT JOIN expenses e ON e.tag_id = t.id`. Requires `idx_expenses_tag_id`. |
| `view_joint_account_monthly` | Joint account monthly spending | `INNER JOIN joint_account_categories jac ON jac.category = e.category`. |

---

## 🧪 Zero-Regression Verification Workflow

Before committing any SQL or schema modifications, you **must** execute the verification test suites as mandated by `AGENTS.md` (Section 7):

```bash
# 1. Run backend integration scenarios (12 end-to-end integration tests)
uv run --directory backend pytest tests/test_scenarios_integration.py

# 2. Run full backend test suite (328 tests) with coverage
uv run --directory backend pytest --cov=app --cov-report=term

# 3. Clean Host / Docker fallback (if host lacks local npm/dependencies)
docker run --rm -v $(pwd)/backend/app:/app/app -v $(pwd)/backend/tests:/app/tests jizifin-backend-test pytest
```

### Documentation Sync Invariant
Whenever schema or indexes change in `backend/app/database.py`:
- Update `AGENTS.md` Section 3 (**DATABASE SCHEMA & LOGIC CONSTRAINTS**).
- Update `README.md` Section 3 (**Tech Stack / Database / Testing**).
