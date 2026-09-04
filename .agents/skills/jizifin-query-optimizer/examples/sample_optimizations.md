# Jizifin Query Optimization Examples

Practical examples demonstrating how to detect, analyze, and resolve execution plan bottlenecks in Jizifin.

---

## Example 1: Foreign Key JOIN in Project Summary

### ❌ Before (Unindexed Foreign Key)
View: `view_project_summary`
```sql
SELECT p.id, p.name, p.target_cents, p.target_date, COALESCE(SUM(e.cost_cents), 0) AS total_spent_cents
FROM projects p
LEFT JOIN expenses e ON e.project_id = p.id
GROUP BY p.id, p.name, p.target_cents, p.target_date;
```
**Execution Plan:**
```
• [2->0] CO-ROUTINE view_project_summary
  • [10->2] SCAN p USING INDEX sqlite_autoindex_projects_1
  • [15->2] BLOOM FILTER ON e (project_id=?)
  • [25->2] SEARCH e USING AUTOMATIC COVERING INDEX (project_id=?) LEFT-JOIN
• [78->0] SCAN view_project_summary
```
*Diagnosis*: `e.project_id` has no index. SQLite dynamically generates an ephemeral index and bloom filter on every query execution.

### ✅ After (Filtered Foreign Key Index)
Add index to `_init_db_schema` in `backend/app/database.py`:
```sql
CREATE INDEX IF NOT EXISTS idx_expenses_project_id ON expenses (project_id) WHERE project_id IS NOT NULL;
```
**Optimized Execution Plan:**
```
• [2->0] CO-ROUTINE view_project_summary
  • [10->2] SCAN p USING INDEX sqlite_autoindex_projects_1
  • [25->2] SEARCH e USING INDEX idx_expenses_project_id (project_id=?) LEFT-JOIN
• [78->0] SCAN view_project_summary
```
*Result*: Ephemeral index and bloom filter eliminated; replaced with persistent B-Tree search.

---

## Example 2: SARGable Date Range vs strftime Function

### ❌ Before (Non-SARGable Expression)
```sql
SELECT id, name, cost_cents, expense_date, who_paid, category
FROM expenses
WHERE strftime('%Y-%m', expense_date) = '2025-01'
ORDER BY expense_date DESC;
```
**Execution Plan:**
```
• [2->0] SCAN expenses
• [12->0] USE TEMP B-TREE FOR ORDER BY
```
*Diagnosis*: Evaluating `strftime()` on every row forces a full table scan and temporary B-tree sorting.

### ✅ After (SARGable Date Range + Index)
Index:
```sql
CREATE INDEX IF NOT EXISTS idx_expenses_date_id ON expenses (expense_date DESC, id DESC);
```
Query Rewrite:
```sql
SELECT id, name, cost_cents, expense_date, who_paid, category
FROM expenses
WHERE expense_date >= '2025-01-01' AND expense_date <= '2025-01-31'
ORDER BY expense_date DESC, id DESC;
```
**Optimized Execution Plan:**
```
• [2->0] SEARCH expenses USING INDEX idx_expenses_date_id (expense_date>? AND expense_date<?)
```
*Result*: Full table scan and temporary B-tree sorting completely removed.
