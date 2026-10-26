# Reconciliation identities and queries

Every query is a single plain `SELECT` so it runs through the console: `POST /query` with `{"sql": "..."}` (1000-row cap, no table functions, no writes). Replace the month bounds `DATE '2026-09-01'` / `DATE '2026-09-30'`, the other literal dates (use today) and the ids. Never use `current_date`, `now()` or time-zone functions: they need the ICU extension, which the bundled DuckDB dev build cannot download, and the query fails with HTTP 500. Amounts are cents; `SUM` must be cast (`CAST(SUM(x) AS BIGINT)`) because DuckDB sums to `HUGEINT`.

```bash
q() { curl -s -XPOST "$BASE/query" -H "authorization: Bearer $tok" -H 'content-type: application/json' --data-binary @- ; }
python3 -c 'import json,sys;print(json.dumps({"sql":sys.stdin.read()}))' <<'SQL' | q
SELECT kind, COUNT(*) AS n, CAST(SUM(amount_cents) AS BIGINT) AS cents FROM entries
WHERE entry_date BETWEEN DATE '2026-09-01' AND DATE '2026-09-30' GROUP BY kind ORDER BY kind
SQL
```

## 1. Ledger and plain totals

**L1 — month totals per kind.**

```sql
SELECT kind, COUNT(*) AS n, CAST(SUM(amount_cents) AS BIGINT) AS cents FROM entries
WHERE entry_date BETWEEN DATE '2026-09-01' AND DATE '2026-09-30' GROUP BY kind ORDER BY kind
```

Ties to: `expense` cents = `/views/month?month=2026-09` `total` = `/cashflow?month=2026-09` `spent` (no member filter; every expense has a payer by CHECK constraint). `count` = n.

**L2 — expenses per member and per category** (ties to `/views/month` `by_member` / `by_category`, and `by_member` to `/cashflow?members=<id>` `spent`).

```sql
SELECT member_id, category_id, CAST(SUM(amount_cents) AS BIGINT) AS cents, COUNT(*) AS n FROM entries
WHERE kind = 'expense' AND entry_date BETWEEN DATE '2026-09-01' AND DATE '2026-09-30'
GROUP BY ROLLUP (member_id, category_id) ORDER BY member_id NULLS LAST, category_id NULLS LAST
```

## 2. Settlement

The settlement books only *payable* expenses: kind `expense`, no `account_id`, and a category not charged to any joint account.

**S1 — payable total** = Σ `value.rows[].total`.

```sql
SELECT CAST(SUM(amount_cents) AS BIGINT) AS cents, COUNT(*) AS n FROM entries
WHERE kind = 'expense' AND account_id IS NULL
  AND category_id NOT IN (SELECT category_id FROM account_categories)
  AND entry_date BETWEEN DATE '2026-09-01' AND DATE '2026-09-30'
```

**S2 — paid per member** = Σ over rows of `paid[member]`. A mismatch here while S1 ties means a bucket was dropped with a non-zero net (see SKILL.md, known findings).

```sql
SELECT member_id, CAST(SUM(amount_cents) AS BIGINT) AS paid FROM entries
WHERE kind = 'expense' AND account_id IS NULL
  AND category_id NOT IN (SELECT category_id FROM account_categories)
  AND entry_date BETWEEN DATE '2026-09-01' AND DATE '2026-09-30'
GROUP BY member_id ORDER BY member_id
```

**S3 — per bucket** (category; personal-pay categories also split by payer) = each row's `total` and `paid`.

```sql
SELECT e.category_id, CASE WHEN c.pay_mode = 'personal' THEN e.member_id END AS personal_of,
       e.member_id, CAST(SUM(e.amount_cents) AS BIGINT) AS paid
FROM entries e JOIN categories c ON c.id = e.category_id
WHERE e.kind = 'expense' AND e.account_id IS NULL
  AND e.category_id NOT IN (SELECT category_id FROM account_categories)
  AND e.entry_date BETWEEN DATE '2026-09-01' AND DATE '2026-09-30'
GROUP BY ALL ORDER BY ALL
```

**Expected shares** depend on precedence and cannot be rebuilt in SQL. Check them per entry: `GET /entries/{id}/shares` (`source`, `shares`, Σ shares = amount) or read the `?explain=1` trail. Spot-check at least the largest entry of each bucket and every entry with an override:

```sql
SELECT entry_id, SUM(pct) AS pct_total, COUNT(*) AS members FROM entry_share_overrides GROUP BY entry_id ORDER BY entry_id
```

**Settlement arithmetic by hand:** per row `net[m] = paid[m] − expected[m]`; household `net` = Σ rows (before offsets) adjusted by active offset rules; Σ net = 0; Σ `debts[].amount` = `transfer_total`; applying debts zeroes every net; no debt crosses `components`.

## 3. Joint accounts

**J1 — balance to month end** = dashboard `balance` (expenses negative; deposits, corrections and income positive).

```sql
SELECT CAST(SUM(CASE kind WHEN 'expense' THEN -amount_cents ELSE amount_cents END) AS BIGINT) AS balance
FROM entries WHERE account_id = 1 AND entry_date <= DATE '2026-09-30'
```

**J2 — month activity** = `actual_total` (expense), `deposited_total` (deposits + positive corrections), `members[].deposited`, `categories[].actual`.

```sql
SELECT kind, member_id, category_id, CAST(SUM(amount_cents) AS BIGINT) AS cents FROM entries
WHERE account_id = 1 AND entry_date BETWEEN DATE '2026-09-01' AND DATE '2026-09-30'
GROUP BY ROLLUP (kind, member_id, category_id) ORDER BY ALL
```

**J3 — deposit target** = `round_ties_even(expected_total × (1 + safety_margin_pct / 100))`, where `expected_total` is the account's `expected_total_cents` when set, otherwise Σ `account_categories.expected_cents`. Σ `per_member` = target; every `rounded` amount ≥ its unrounded amount and a multiple of `round_to`. `top_up = max(0, target − balance)`.

```sql
SELECT a.id, a.expected_total_cents, a.safety_margin_pct, a.deposit_mode,
       CAST(SUM(ac.expected_cents) AS BIGINT) AS categories_expected
FROM accounts a LEFT JOIN account_categories ac ON ac.account_id = a.id GROUP BY ALL ORDER BY a.id
```

## 4. Budgets, income, cash flow

**B1 — spent per budgeted category** = `/budgets/status` `rows[].spent` (no member/account filter); `remaining = limit − spent`; summary totals are Σ rows.

```sql
SELECT category_id, CAST(SUM(amount_cents) AS BIGINT) AS spent FROM entries
WHERE kind = 'expense' AND entry_date BETWEEN DATE '2026-09-01' AND DATE '2026-09-30'
  AND category_id IN (SELECT category_id FROM budgets)
GROUP BY category_id ORDER BY category_id
```

**I1 — income inputs** for `/income/effective`: contract per job = `round_ties_even(amount × periods / 12)` (monthly 12, weekly 52, biweekly 26, annual 1), active when the month overlaps `[valid_from, valid_to]`; an override for the month replaces the contract as `base`; `one_off` = income entries; `total = base + one_off`.

```sql
SELECT 'job' AS src, member_id, amount_cents, frequency, valid_from, valid_to FROM jobs
UNION ALL SELECT 'override', member_id, amount_cents, NULL, month, NULL FROM income_overrides WHERE month = DATE '2026-09-01'
UNION ALL SELECT 'one_off', member_id, CAST(SUM(amount_cents) AS BIGINT), NULL, NULL, NULL FROM entries
  WHERE kind = 'income' AND entry_date BETWEEN DATE '2026-09-01' AND DATE '2026-09-30' GROUP BY member_id
ORDER BY member_id, src
```

**C1 — cash flow**: `income` = Σ I1 totals of scoped members, `spent` = L2 per member, `net = income − spent`, `savings_rate_pct = net / income × 100` (absent without income).

## 5. Hygiene

**D1 — implausible dates.**

```sql
SELECT id, kind, entry_date, amount_cents FROM entries
WHERE entry_date < DATE '2000-01-01' OR entry_date > DATE '2026-12-31' ORDER BY entry_date
```

**D2 — likely duplicates** (same payer, category, date and amount).

```sql
SELECT member_id, category_id, entry_date, amount_cents, COUNT(*) AS n, list(id ORDER BY id) AS ids
FROM entries WHERE kind = 'expense' GROUP BY ALL HAVING COUNT(*) > 1 ORDER BY entry_date
```

**D3 — orphans** (no foreign keys exist, so the repositories are the only guard).

```sql
SELECT 'member' AS ref, e.id FROM entries e WHERE e.member_id IS NOT NULL AND e.member_id NOT IN (SELECT id FROM members)
UNION ALL SELECT 'category', e.id FROM entries e WHERE e.category_id IS NOT NULL AND e.category_id NOT IN (SELECT id FROM categories)
UNION ALL SELECT 'account', e.id FROM entries e WHERE e.account_id IS NOT NULL AND e.account_id NOT IN (SELECT id FROM accounts)
UNION ALL SELECT 'project', e.id FROM entries e WHERE e.project_id IS NOT NULL AND e.project_id NOT IN (SELECT id FROM projects)
UNION ALL SELECT 'tag', e.id FROM entries e WHERE e.tag_id IS NOT NULL AND e.tag_id NOT IN (SELECT id FROM tags)
UNION ALL SELECT 'share_member', s.agreement_id FROM agreement_shares s WHERE s.member_id NOT IN (SELECT id FROM members)
```

**D4 — kind mismatch** (an expense booked on an income category or vice versa).

```sql
SELECT e.id, e.kind, c.kind AS category_kind FROM entries e JOIN categories c ON c.id = e.category_id
WHERE e.kind IN ('expense', 'income') AND e.kind <> c.kind
```

**D5 — agreements**: shares not summing to 100, bounded agreements overlapping in one category, and inactive members still holding a share in an agreement that is in force.

```sql
SELECT 'not_100' AS issue, a.id, a.category_id, SUM(s.pct) AS detail FROM agreements a
  JOIN agreement_shares s ON s.agreement_id = a.id GROUP BY a.id, a.category_id HAVING abs(SUM(s.pct) - 100) > 1e-6
UNION ALL SELECT 'overlap', a.id, a.category_id, b.id FROM agreements a JOIN agreements b
  ON a.category_id = b.category_id AND a.id < b.id AND a.valid_to IS NOT NULL AND b.valid_to IS NOT NULL
  AND a.valid_from <= b.valid_to AND b.valid_from <= a.valid_to
UNION ALL SELECT 'inactive_member', a.id, a.category_id, s.member_id FROM agreements a
  JOIN agreement_shares s ON s.agreement_id = a.id JOIN members m ON m.id = s.member_id
  WHERE NOT m.is_active AND (a.valid_to IS NULL OR a.valid_to >= DATE '2026-10-26')
```

**D6 — sealed months**: `GET /settlements` lists seals; every sealed month must return `drift: false` from `GET /settlements/{m}`. A drift means a rule, agreement, member or category changed after sealing; find which (compare the stored `result_json` rows with the live ones) and book the correction in the current month.
