# 📋 Jizifin Backend Vulnerability Audit & Correction Plan

This document details all identified architectural shortcomings, calculation discrepancies, and failure modes across the Jizifin backend that could propagate into incorrect balance sheets, distorted cash flows, or runtime errors. Each vulnerability is mapped to a concrete, actionable correction plan.

---

## 1. 🧮 Core Calculation & Balance Sheet Vulnerabilities

### 1.1 Income Proration Flaws in [`IncomeResolver`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-auditor/src/income.rs#L43-L126)
- **File:** [`backend/crates/jizifin-auditor/src/income.rs`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-auditor/src/income.rs#L81-L111)
- **Vulnerabilities:**
  1. **New job starting mid-month:** When a member starts an open-ended job mid-month (e.g. June 15 with `valid_to: None`), `switch_month` is `None` (no prior job replaced) and `valid_to` is `None`. `payout()` returns **100% of the full monthly salary** for June rather than prorating the 16 remaining days worked.
  2. **Bounded jobs starting after the 1st:** For temporary contracts (e.g., June 10 to June 20), `prorate()` computes `worked = end.day()` (i.e. 20 days), falsely assuming the contract began on the 1st of the month, overpaying by 9 days.
- **Balance Sheet Propagation:** Distorts [`SalaryRatios`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-auditor/src/ratios.rs#L14-L19), [`IncomeRatio`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-auditor/src/shares.rs#L30) expense splits, and household [`CashFlow`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-auditor/src/cashflow.rs#L13-L24) income, misallocating liabilities to or from members.
- **Actionable Correction:**
  - Update [`IncomeResolver::payout`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-auditor/src/income.rs#L81-L93) and `prorate` to compute `days_worked` bounded by both `valid_from` and `valid_to` against the calendar month's actual day count:
    $$\text{days\_worked} = \min(\text{valid\_to}, \text{month.last\_day()}) - \max(\text{valid\_from}, \text{month.first\_day()}) + 1$$

---

### 1.2 Offset Rules Silently Ineffective for Personal-Pay Categories
- **File:** [`backend/crates/jizifin-auditor/src/settlement/offsets.rs`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-auditor/src/settlement/offsets.rs#L37-L48)
- **Vulnerability:**
  - `OffsetApplier::offset_amount` looks up spend using `Bucket { category, personal_of: None }`.
  - In [`CategoryLedger::book`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-auditor/src/settlement/ledger.rs#L21-L27), any category with `pay_mode: PayMode::Personal` is stored under `Bucket { category, personal_of: Some(payer) }`.
  - Any offset rule referencing a personal-pay category looks up a non-existent bucket and resolves to `Cents::ZERO`.
- **Balance Sheet Propagation:** Bilateral deductions and compensations (e.g. offsetting a personal cost like fuel or personal hobby against a shared cost) fail silently, leaving peer debt balances uncorrected.
- **Actionable Correction:**
  - In `OffsetApplier::offset_amount`, look up both `Bucket { category, personal_of: None }` and `Bucket { category, personal_of: Some(member) }` so expenses booked in personal-pay categories are properly counted toward offset deductions.

---

### 1.3 Joint Account Project Equity Memory Loss Across Months
- **File:** [`backend/crates/jizifin-auditor/src/project.rs`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-auditor/src/project.rs#L92-L100)
- **Vulnerability:**
  - In [`ProjectSettlementCalculator::equity`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-auditor/src/project.rs#L92-L100), joint account deposits are filtered strictly within `month.period()` (the single calendar month of the entry date).
  - If members saved \$5,000 across preceding months in the joint account specifically for a renovation project and spend \$4,000 in October with zero new deposits made in October, cumulative deposit equity is ignored. The equity falls back to an arbitrary equal split (50/50).
  - Intra-month deposit timing also distorts equity: an expense on the 3rd assigns 100% of funding to a member who deposited on the 1st if another member deposits on the 5th.
- **Balance Sheet Propagation:** Project equity balances, participant net contributions, and project settlement debts do not reflect true historical funding.
- **Actionable Correction:**
  - Compute joint equity based on cumulative historical deposits up to the entry date (`e.date <= entry.date`), or based on the account's configured ownership ratio, rather than a single calendar month.

---

### 1.4 Individual Cash Flow Distortion on Joint Expenses and Transfers
- **File:** [`backend/crates/jizifin-auditor/src/cashflow.rs`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-auditor/src/cashflow.rs#L56-L60)
- **Vulnerability:**
  - In [`CashFlowCalculator::spent`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-auditor/src/cashflow.rs#L56-L60), spend is aggregated solely from `EntryKind::Expense` filtered by `e.member`.
  - The database schema and validation mandate that every expense specify a `member_id`, even when charged to a joint account (`e.account.is_some()`).
  - When a \$1,500 rent debit occurs on the joint account and is assigned to Member A, Member A's individual cashflow is docked \$1,500, while Member B shows \$0 spent.
  - Crucially, actual member cash outflows—deposits made into the joint account—are classified as `EntryKind::Deposit` and excluded from `spent`.
- **Balance Sheet Propagation:** Individual cash flows (`/cashflow?members=<id>`) and savings rates are grossly distorted: members funding the joint account appear to save 100%, while the member named on joint invoices appears to have spent their personal funds.
- **Actionable Correction:**
  - Differentiate member-scoped cash flow: for individual members, include deposits made into joint accounts as cash outflows and exclude direct joint account debits, or reflect liability shares rather than nominal invoice names.

---

### 1.5 Joint Category Expenses Paid Out-of-Pocket Dropped from Settlement
- **File:** [`backend/crates/jizifin-auditor/src/settlement/calculator.rs`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-auditor/src/settlement/calculator.rs#L96-L100)
- **Vulnerability:**
  - [`SettlementCalculator::payback_entries`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-auditor/src/settlement/calculator.rs#L96-L100) filters out any expense where `snap.is_joint_category(c)` is true.
  - If Member A pays out-of-pocket (personal card, `account_id: None`) for an expense under a category linked to an account (e.g. household groceries or emergency repairs), it is omitted from peer settlement.
  - Because `account_id` is `None`, it is also not booked against the joint account balance.
- **Balance Sheet Propagation:** The payer is neither reimbursed by household peers in the peer settlement nor reimbursed by the joint account; the expense vanishes from reconciliation.
- **Actionable Correction:**
  - Only exclude joint expenses when `e.is_joint_expense()` (`account_id IS NOT NULL`). Expenses paid with personal funds (`account_id IS NULL`) must participate in peer settlement or be surfaced as a pending reimbursement from the joint account.

---

### 1.6 Joint Dashboard Invariant Failure on Unbudgeted Expenses or Refunds
- **File:** [`backend/crates/jizifin-auditor/src/joint.rs`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-auditor/src/joint.rs#L187-L196)
- **Vulnerability:**
  - [`JointBalanceCalculator`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-auditor/src/joint.rs#L83-L179) enforces `CategoriesWithinActual`: `sum(categories.actual).abs() <= actual_total.abs()`.
  - `categories` only includes categories explicitly configured in `account_categories`.
  - If an expense or refund is booked on the joint account under an unbudgeted category, a refund in that unbudgeted category can reduce `actual_total` below the sum of the budgeted categories.
- **Balance Sheet Propagation:** The invariant fails with an HTTP 500 error, locking the household out of the joint account dashboard.
- **Actionable Correction:**
  - Update `CategoriesWithinActual` to account for unbudgeted categories and negative refunds, ensuring the invariant evaluates total positive and negative flows without failing on legitimate credit adjustments.

---

### 1.7 Joint Account Income Excluded from Surplus Calculation
- **File:** [`backend/crates/jizifin-auditor/src/joint.rs`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-auditor/src/joint.rs#L143-L147)
- **Vulnerability:**
  - `deposited_total()` only tallies `EntryKind::Deposit` and positive `EntryKind::Correction`.
  - Any direct joint account income (bank interest, merchant cashback, tax credits) increases `balance` via `signed()`, but is omitted from `deposited_total`.
  - `month_surplus` (`deposited_total - actual_total`) therefore reports a false deficit.
- **Balance Sheet Propagation:** Joint account monthly performance and surplus/deficit figures are understated.
- **Actionable Correction:**
  - Include `EntryKind::Income` credited to joint accounts in `deposited_total()` and `month_surplus`.

---

## 2. 🔒 Month-End Close, Sealing, and Drift Hazards

### 2.1 False Drift Flag on Filtered Queries of Sealed Settlements
- **File:** [`backend/crates/jizifin-api/src/routes/settlements.rs`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-api/src/routes/settlements.rs#L52-L58)
- **Vulnerability:**
  - When querying `GET /settlements/{month}?members=1`, `fetch` computes a live settlement filtered to Member 1 and compares it to the stored seal (which contains all members).
  - `live != stored` evaluates to `true`, reporting `drift: true` even though the ledger has not changed.
- **Balance Sheet Propagation:** Erroneous drift warnings undermine confidence in audited month-end closes.
- **Actionable Correction:**
  - When evaluating drift against a stored seal, always recompute with the full household scope (`scope.members = None`), regardless of any query filter passed by the client.

---

### 2.2 Unprotected Upstream Master Data in Sealed Months
- **Files:** [`backend/crates/jizifin-store/src/repo/income.rs`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-store/src/repo/income.rs), [`backend/crates/jizifin-store/src/repo/agreements.rs`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-store/src/repo/agreements.rs)
- **Vulnerability:**
  - [`Seals::guard`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-store/src/repo/seals.rs#L35-L43) is enforced when creating, updating, or deleting `entries`.
  - However, `Jobs`, `JobAdjustment`, `IncomeOverrides`, and `Agreements` (SCD2 splits) can be modified, backdated, or deleted inside sealed months without triggering `Seals::guard`.
- **Balance Sheet Propagation:** Modifying historical contracts retroactively causes sealed months to drift and breaks ledger auditability.
- **Actionable Correction:**
  - Add `Seals::guard` checks to `Jobs::update`, `Jobs::adjust`, `IncomeOverrides::upsert`, and `Agreements::update` for any covered dates that lie in sealed months.

---

### 2.3 Absence of an Unseal Endpoint
- **File:** [`backend/crates/jizifin-api/src/routes/settlements.rs`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-api/src/routes/settlements.rs)
- **Vulnerability:**
  - There is no `DELETE /settlements/{month}/seal` route.
- **Operational Hazard:** If an administrative error is sealed, or if a bank reconciliation requires an adjustment in a sealed month, the user cannot unlock the month via the API.
- **Actionable Correction:**
  - Implement `DELETE /settlements/{month}/seal` in `jizifin-store` and `jizifin-api`, emitting a `settlements.unsealed` event.

---

## 3. 🖥️ Store & Backend Infrastructure Pitfalls

### 3.1 Unhandled `NULL` Conversion Panics in Aggregate Views
- **File:** [`backend/crates/jizifin-store/src/repo/views.rs`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-store/src/repo/views.rs#L256-L258)
- **Vulnerability:**
  - `id_bucket` decodes the grouping key with `r.get(0)?` expecting non-null `i64`.
  - When calling `/views/month?kind=deposit` or `/views/month?kind=correction`, `category_id` is `NULL`.
  - DuckDB returns a `NULL` column, causing `FromSqlConversionFailure` and returning an HTTP 500 error.
- **Operational Hazard:** Summary view endpoints crash when inspecting non-expense entry kinds.
- **Actionable Correction:**
  - Support `Option<i64>` in `id_bucket` or coalesce NULL keys (`COALESCE(category_id, 0)`) in `Views::month`.

---

### 3.2 Recurring Entry Materialization Data Loss (`GRACE_DAYS = 7`)
- **File:** [`backend/crates/jizifin-store/src/repo/recurring.rs`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-store/src/repo/recurring.rs#L47-L53)
- **Vulnerability:**
  - `Recurring::materialize` bounds catch-up to `today - Duration::days(grace_days)`.
  - On a local or home server (such as an Apple Silicon MacBook home server), if the machine is offline or suspended for >7 days (e.g. during a holiday or hardware downtime), recurring bills in an unsealed month are permanently skipped.
- **Balance Sheet Propagation:** Household recurring expenses and income are omitted, skewing cash flow and account balances.
- **Actionable Correction:**
  - Remove the hard 7-day cutoff for recurring materialization in unsealed months, searching back to the earliest unsealed date or the template's last materialization.

---

### 3.3 Unbounded Reads in Snapshot Loader for Account Scopes
- **File:** [`backend/crates/jizifin-store/src/loader.rs`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-store/src/loader.rs#L45-L54)
- **Vulnerability:**
  - In `AuditScope::account_to_date`, the period is set from `0001-01-01` to `month.last_day()`.
  - `loader.rs` does not include `AND account_id = ?` in its SQL query when `scope.account` is present.
  - The loader loads the entire history of all household entries across all accounts and members into memory.
- **Operational Hazard:** Causes high latency and unnecessary memory usage as the ledger grows.
- **Actionable Correction:**
  - Push `account_id` filtering into `DuckSnapshotLoader::entries` when `scope.account` is populated.

---

### 3.4 UTC Date Rollover Desynchronization Around Midnight
- **File:** [`backend/crates/jizifin-api/src/routes/query.rs`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-api/src/routes/query.rs#L38-L40)
- **Vulnerability:**
  - `today()` evaluates `Utc::now().date_naive()`.
  - For households in European (CET/CEST) or American timezones, queries around midnight evaluate to the previous or next calendar day.
- **Operational Hazard:** Causes month boundary glitches and incorrect due-day alerts on the first day of each month.
- **Actionable Correction:**
  - Accept an optional client date header or timezone parameter, falling back to server local date rather than strictly UTC.

---

### 3.5 Discrepancy Between Split Overview and Entry Evaluation (`overlaps` vs `covers`)
- **File:** [`backend/crates/jizifin-auditor/src/splits.rs`](file:///home/jim/Documents/jizifin/backend/crates/jizifin-auditor/src/splits.rs#L77-L81)
- **Vulnerability:**
  - `SplitOverviewCalculator` marks an agreement active if `month.overlaps(valid_from, valid_to)`.
  - An entry on date $D$ resolves agreements via `agreement.covers(D)`.
  - If a split agreement starts mid-month (e.g. October 15), the overview shows the agreement applies to the entire month, whereas expenses booked prior to October 15 use the baseline split.
- **Visual Hazard:** Users observe a disconnect between the split overview percentage and actual expense settlement shares.
- **Actionable Correction:**
  - Indicate partial month coverage or effective date boundaries in `SplitOverview` when an agreement covers only part of the month.

---

# 🚀 Actionable Implementation Matrix

| Priority | Issue | Actionable Fix | Target Crates |
| :---: | :--- | :--- | :--- |
| **P0** | **1.1 Income Proration** | Update `IncomeResolver::payout` to calculate `days_worked` bounded by both `valid_from` and `valid_to` against the calendar month's actual day count. | `jizifin-auditor` |
| **P0** | **1.2 Personal Offset Rules** | In `OffsetApplier`, look up both `Bucket { category, personal_of: None }` and `Bucket { category, personal_of: Some(payer) }`. | `jizifin-auditor` |
| **P0** | **1.3 Project Joint Equity** | Compute joint equity based on cumulative historical deposits up to the entry date, or based on the account's fixed ownership ratio, rather than a single calendar month. | `jizifin-auditor` |
| **P0** | **1.5 Joint Category Reimbursement** | Allow personal payments (`account == None`) for joint categories to be settled via peer payback, or treat them as reimbursable debits against the joint account. | `jizifin-auditor` |
| **P1** | **1.6 Joint Invariant `CategoriesWithinActual`** | Account for unbudgeted category spend and negative refunds in `CategoriesWithinActual` invariant comparison. | `jizifin-auditor` |
| **P1** | **1.7 Joint Income Surplus** | Include `EntryKind::Income` credited to joint accounts in `deposited_total()` and `month_surplus`. | `jizifin-auditor` |
| **P1** | **2.1 False Drift Detection** | Always compare stored seals against an unfiltered (`scope.members = None`) recomputation, regardless of query filter. | `jizifin-api` |
| **P1** | **2.2 Upstream Seal Guards** | Guard `jobs`, `income_overrides`, and `agreements` writes with `Seals::guard` to prevent retroactive drift. | `jizifin-store` |
| **P1** | **3.1 View `NULL` Buckets** | Support `Option<i64>` in `id_bucket` or coalesce NULL keys (`COALESCE(category_id, 0)`) in `Views::month`. | `jizifin-store` |
| **P2** | **1.4 Cash Flow Representation** | Support net personal cash outlay in member-scoped cash flow: include deposits into joint accounts and exclude joint account direct charges. | `jizifin-auditor` |
| **P2** | **2.3 Unseal Endpoint** | Implement `DELETE /settlements/{month}/seal` with appropriate audit event emission (`settlements.unsealed`). | `jizifin-api`, `jizifin-store` |
| **P2** | **3.2 Recurring Catch-up Grace** | Remove the hard 7-day cutoff for recurring materialization in unsealed months so downtime does not lose entries. | `jizifin-store` |
| **P2** | **3.3 Loader Query Scoping** | Push `account_id` filtering into `DuckSnapshotLoader::entries` when `scope.account` is populated. | `jizifin-store` |
| **P3** | **3.5 Split Overview Consistency** | Indicate date boundaries or split transitions in `SplitOverview` when an agreement covers only part of a month. | `jizifin-auditor` |
