/**
 * stores.js — Svelte writable stores for cross-component reactive state.
 *
 * users             → array of UserResponse objects (includes inactive when fetched with flag)
 * expenses          → array of ExpenseResponse objects (newest-first)
 * splits            → array of SplitResponse objects (each has allocations: [{user_name, pct}])
 * analytics         → { monthly_total, by_category, by_payer } from the three views
 * selectedMonth     → currently selected YYYY-MM string used to filter Dashboard & Expenses
 * incomeAnalytics   → array of IncomeByPersonRow for the selected month (with carry-forward)
 * paybacks          → PaybackSummary: { rows, debts, month }
 * budgets           → array of BudgetResponse rows (raw config)
 * recurringExpenses → array of RecurringResponse objects
 * settlements       → array of SettlementResponse rows (locked months)
 * projects          → array of ProjectResponse objects
 * tags              → array of TagTotalRow objects (all-time aggregates per tag)
 */

import { writable } from 'svelte/store';

/**
 * Helper: writable store that reads/writes a boolean to localStorage.
 * Falls back to `defaultValue` when no entry exists yet.
 */
function persistedBoolean(key, defaultValue) {
  const stored = localStorage.getItem(key);
  const initial = stored !== null ? stored === 'true' : defaultValue;
  const store = writable(initial);
  store.subscribe((val) => localStorage.setItem(key, String(val)));
  return store;
}

/**
 * Household users. Each entry: { name, color, is_active, created_at }
 * Populated by fetchUsers(). The store holds ALL users (active + inactive)
 * so deactivated users remain visible in history. Filter by is_active at
 * the UI layer where only active users should be selectable.
 */
export const users = writable([]);

export const expenses = writable([]);

export const splits = writable([]);

export const analytics = writable({
  monthly_total: { total_amount: 0.0, expense_count: 0, month: '' },
  by_category: [],
  by_payer: [],
});

/** Current month in YYYY-MM format, shared across Dashboard and Expenses tabs. */
const now = new Date();
export const selectedMonth = writable(
  `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`
);

/**
 * Income totals per person for the selected month.
 * Each entry: { who: string, total_cents: number, is_carried: boolean }
 */
export const incomeAnalytics = writable([]);

/**
 * Payback/settlement analytics summary.
 * Structure: { rows: PaybackRow[], debts: DebtItem[], month: string }
 * PaybackRow: { category, total_amount, per_user_paid, per_user_share_pct, net_per_user }
 * DebtItem:   { from_user, to_user, amount }
 */
export const paybacks = writable({
  rows: [],
  debts: [],
  month: '',
});

/**
 * Projects list. Each entry: ProjectResponse from /projects.
 * Fields: id, name, target_cents, target_date, total_spent_cents,
 *         avg_monthly_payment_cents, last_payment, estimated_completion_date
 */
export const projects = writable([]);

/**
 * Tags (open-ended event labels). Each entry: TagTotalRow from /tags.
 * Fields: id, name, color, description, total_amount, expense_count, first_date, last_date
 */
export const tags = writable([]);

/**
 * Budget raw config rows. Each entry: { category, month, limit_cents }
 */
export const budgets = writable([]);

/**
 * Recurring expense templates. Each entry:
 * { id, name, cost_cents, who_paid, category, day_of_month }
 */
export const recurringExpenses = writable([]);

/**
 * Locked months. Each entry: { month, settled_at, net_balance_transferred_cents }
 */
export const settlements = writable([]);

/**
 * UI preference: allow editing split percentages on mobile.
 * Default false — mobile view is read-only until the user opts in via Settings.
 * Persisted to localStorage so the choice survives page reload.
 */
export const mobileSplitsEditable = persistedBoolean('mobileSplitsEditable', false);

/**
 * Helper: writable store that reads/writes a string to localStorage.
 */
function persistedString(key, defaultValue) {
  const stored = localStorage.getItem(key);
  const initial = stored !== null ? stored : defaultValue;
  const store = writable(initial);
  store.subscribe((val) => localStorage.setItem(key, val));
  return store;
}

export const defaultPayer = persistedString('defaultPayer', '');
export const defaultCategory = persistedString('defaultCategory', '');
export const showQueryTab = persistedBoolean('showQueryTab', true);
export const currencySymbol = persistedString('currencySymbol', '€');

/**
 * UI display mode: how split percentages are entered.
 * 'inputs' — individual number fields per user (default, works for any user count).
 * 'slider' — single linked range slider when exactly 2 active users; drag one side,
 *            the other auto-complements to 100%. Falls back to inputs if >2 users.
 */
export const splitInputMode = persistedString('splitInputMode', 'inputs');

/**
 * UI display mode: how per-category payback breakdown is rendered.
 * 'cards' — current grid of per-user cards showing paid/share/net (default).
 * 'bar'   — horizontal stacked bars per category, segments proportional to amount paid.
 */
export const paybackDisplayMode = persistedString('paybackDisplayMode', 'cards');

/**
 * UI display mode: category spending chart type on the dashboard.
 * 'doughnut' — current doughnut/donut chart (default).
 * 'bar'      — horizontal bar chart with categories on the Y axis.
 */
export const chartStyle = persistedString('chartStyle', 'doughnut');

export const authSalt = writable('');
export const cryptoKey = writable(null);


