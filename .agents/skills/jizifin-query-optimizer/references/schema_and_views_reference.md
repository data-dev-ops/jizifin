# Schema & Views Reference for Jizifin Query Optimization

This reference document compiles the complete database topography from [`backend/app/database.py`](file:///home/jim/Documents/jizifin/backend/app/database.py) and [`AGENTS.md`](file:///home/jim/Documents/jizifin/AGENTS.md) (Section 3).

---

## 1. Table Definitions & Encryption Status (v4 Schema)

| Table | Column | Type | Encrypted? | Index Status | Notes / Foreign Keys |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `app_config` | `key` | TEXT | No | PRIMARY KEY | App settings, e.g. `magic_word` |
| `users` | `name` | TEXT | **Yes** | PRIMARY KEY | `CHECK(length(name) <= 256)` |
| `splits` | `category` | TEXT | **Yes** | PRIMARY KEY | Category registry |
| `income_categories` | `category` | TEXT | **Yes** | PRIMARY KEY | Category registry (no FK from `income`) |
| `projects` | `id` | INTEGER | No | PRIMARY KEY | Autoincrement |
| `projects` | `name` | TEXT | **Yes** | UNIQUE | Encrypted goal name |
| `projects` | `target_cents` | INTEGER | No | None | Stored as integer cents |
| `projects` | `target_date` | TEXT | No | None | YYYY-MM-DD |
| `tags` | `id` | INTEGER | No | PRIMARY KEY | Autoincrement |
| `tags` | `name` | TEXT | **Yes** | UNIQUE | Encrypted label |
| `tags` | `start_date` | TEXT | No | None | Nullable YYYY-MM-DD |
| `tags` | `end_date` | TEXT | No | None | Nullable YYYY-MM-DD |
| `expenses` | `id` | INTEGER | No | PRIMARY KEY | Autoincrement |
| `expenses` | `name` | TEXT | **Yes** | None | Encrypted description |
| `expenses` | `cost_cents` | INTEGER | No | None | Positive integer cents |
| `expenses` | `expense_date` | TEXT | No | Target for Index | YYYY-MM-DD |
| `expenses` | `who_paid` | TEXT | **Yes** | Target for Index | `REFERENCES users(name) ON UPDATE CASCADE` |
| `expenses` | `category` | TEXT | **Yes** | Target for Index | `REFERENCES splits(category) ON UPDATE CASCADE` |
| `expenses` | `project_id` | INTEGER | No | Target for Index | `REFERENCES projects(id) ON DELETE SET NULL` |
| `expenses` | `tag_id` | INTEGER | No | Target for Index | `REFERENCES tags(id) ON DELETE SET NULL` |
| `expenses` | `is_joint` | INTEGER | No | Target for Index | 0 or 1 |
| `income` | `id` | INTEGER | No | PRIMARY KEY | Autoincrement |
| `income` | `name` | TEXT | **Yes** | None | Encrypted bonus/gift description |
| `income` | `amount_cents` | INTEGER | No | None | Positive integer cents |
| `income` | `who` | TEXT | **Yes** | `idx_income_who_date` | `REFERENCES users(name)` |
| `income` | `income_date` | TEXT | No | `idx_income_who_date` | YYYY-MM-DD |
| `jobs` | `id` | INTEGER | No | PRIMARY KEY | Autoincrement |
| `jobs` | `name` | TEXT | **Yes** | None | Employment stream |
| `jobs` | `who` | TEXT | **Yes** | `idx_jobs_who_dates` | `REFERENCES users(name)` |
| `jobs` | `amount_cents` | INTEGER | No | None | Stored as integer cents |
| `jobs` | `start_date` | TEXT | No | `idx_jobs_who_dates` | YYYY-MM-DD |
| `jobs` | `end_date` | TEXT | No | `idx_jobs_who_dates` | Nullable YYYY-MM-DD |

---

## 2. Read-Only Views Definition Reference

### `view_monthly_total`
```sql
CREATE VIEW view_monthly_total AS
SELECT
    COALESCE(ROUND(SUM(cost_cents) / 100.0, 2), 0.0) AS total_amount,
    COUNT(*)                                           AS expense_count,
    strftime('%Y-%m', 'now')                          AS month
FROM expenses
WHERE strftime('%Y-%m', expense_date) = strftime('%Y-%m', 'now')
```

### `view_monthly_by_category`
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

### `view_expenses_by_month_category`
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

### `view_monthly_by_payer`
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

### `view_project_summary`
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

### `view_tag_totals`
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

### `view_joint_account_monthly`
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
