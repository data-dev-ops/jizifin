/**
 * api.js — All HTTP interactions with the FastAPI backend.
 * Functions update the shared Svelte stores after every successful fetch.
 */

import { expenses, splits, analytics, incomeAnalytics, paybacks, projects, budgets, recurringExpenses, settlements, users } from './stores.js';

const BASE = `http://${window.location.hostname}:8000`;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

async function request(path, options = {}) {
  const res = await fetch(`${BASE}${path}`, options);
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`API ${options.method ?? 'GET'} ${path} → ${res.status}: ${body}`);
  }
  return res.json();
}

// ---------------------------------------------------------------------------
// Users
// ---------------------------------------------------------------------------

/**
 * Fetch household users and update the users store.
 * Pass includeDeactivated=true to load inactive users as well (used by QueryConsole).
 * @param {boolean} [includeDeactivated=false]
 */
export async function fetchUsers(includeDeactivated = false) {
  const qs = includeDeactivated ? '?include_deactivated=true' : '';
  const data = await request(`/users${qs}`);
  users.set(data);
  return data;
}

/**
 * Create a new household user.
 * @param {{ name: string, color: string, is_active?: boolean }} payload
 */
export async function createUser(payload) {
  const data = await request('/users', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  users.update((prev) => [...prev, data]);
  return data;
}

/**
 * Update a user's color or active flag.
 * @param {string} name
 * @param {{ color?: string, is_active?: boolean }} payload
 */
export async function updateUser(name, payload) {
  const data = await request(`/users/${encodeURIComponent(name)}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  users.update((prev) => prev.map((u) => (u.name === name ? data : u)));
  return data;
}

/**
 * Hard-delete a user (only succeeds if they have no expense/income history).
 * @param {string} name
 */
export async function deleteUser(name) {
  const res = await fetch(`${BASE}/users/${encodeURIComponent(name)}`, { method: 'DELETE' });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`API DELETE /users/${name} → ${res.status}: ${body}`);
  }
  users.update((prev) => prev.filter((u) => u.name !== name));
}

// ---------------------------------------------------------------------------
// Expenses
// ---------------------------------------------------------------------------

/** Fetch all expenses and push to the expenses store. */
export async function fetchExpenses() {
  const data = await request('/expenses');
  expenses.set(data);
  return data;
}

/**
 * Create a new expense, update the store, then refresh analytics for the
 * current selected month so cards and charts update immediately.
 * @param {{ name: string, cost_cents: number, expense_date: string, who_paid: string, category: string }} payload
 * @param {string} month  YYYY-MM used to refresh analytics after creation
 */
export async function createExpense(payload, month) {
  const data = await request('/expenses', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  // Prepend to store so the list stays newest-first
  expenses.update((prev) => [data, ...prev]);
  // Refresh analytics so the dashboard reflects the new expense
  if (month) await fetchAnalytics(month);
  return data;
}

/**
 * Delete an expense by id, remove it from the local store, and refresh
 * analytics for the current month so cards update immediately.
 * @param {number} id
 * @param {string} month  YYYY-MM used to refresh analytics after deletion
 */
export async function deleteExpense(id, month) {
  const res = await fetch(`${BASE}/expenses/${id}`, { method: 'DELETE' });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`API DELETE /expenses/${id} \u2192 ${res.status}: ${body}`);
  }
  expenses.update((prev) => prev.filter((e) => e.id !== id));
  // Refresh analytics so the dashboard reflects the removed expense
  if (month) await fetchAnalytics(month);
}

/**
 * Update an existing expense by id.
 * @param {number} id
 * @param {object} payload  Partial ExpenseUpdate fields
 * @param {string} [month]  YYYY-MM to refresh analytics after update
 */
export async function updateExpense(id, payload, month) {
  const data = await request(`/expenses/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  expenses.update((prev) => prev.map((e) => (e.id === id ? data : e)));
  if (month) await fetchAnalytics(month);
  return data;
}

// ---------------------------------------------------------------------------
// Splits
// ---------------------------------------------------------------------------

/** Fetch all split categories and push to the splits store. */
export async function fetchSplits() {
  const data = await request('/splits');
  splits.set(data);
  return data;
}

/**
 * Create a new split category with allocations.
 * @param {{ category: string, allocations: Array<{user_name:string, pct:number}> }} payload
 */
export async function createSplit(payload) {
  const data = await request('/splits', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  splits.update((prev) => [...prev, data]);
  return data;
}

/**
 * Replace split allocations for an existing category.
 * @param {string} category
 * @param {{ allocations: Array<{user_name:string, pct:number}> }} payload
 */
export async function updateSplit(category, payload) {
  const data = await request(`/splits/${encodeURIComponent(category)}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  splits.update((prev) => prev.map((s) => (s.category === category ? data : s)));
  return data;
}


// ---------------------------------------------------------------------------
// Analytics
// ---------------------------------------------------------------------------

/**
 * Fetch all three analytics views for the given month and merge into the
 * analytics store.  Pass `month` as YYYY-MM; omit for the current month.
 * @param {string} [month]
 */
export async function fetchAnalytics(month) {
  const qs = month ? `?month=${encodeURIComponent(month)}` : '';
  const [monthly_total, by_category, by_payer] = await Promise.all([
    request(`/analytics/monthly-total${qs}`),
    request(`/analytics/by-category${qs}`),
    request(`/analytics/by-payer${qs}`),
  ]);
  analytics.set({ monthly_total, by_category, by_payer });
  return { monthly_total, by_category, by_payer };
}

/**
 * Fetch payback and settlement analytics for `month` and update the paybacks store.
 * @param {string} [month]  YYYY-MM
 */
export async function fetchPaybacks(month) {
  const qs = month ? `?month=${encodeURIComponent(month)}` : '';
  const data = await request(`/analytics/paybacks${qs}`);
  paybacks.set(data);
  return data;
}

// ---------------------------------------------------------------------------
// Income
// ---------------------------------------------------------------------------

/**
 * Fetch income-by-person analytics (with salary carry-forward) for `month`.
 * Updates the incomeAnalytics store.
 * @param {string} [month]  YYYY-MM; defaults to current month on the server
 */
export async function fetchIncomeByPerson(month) {
  const qs = month ? `?month=${encodeURIComponent(month)}` : '';
  const data = await request(`/analytics/income-by-person${qs}`);
  incomeAnalytics.set(data);
  return data;
}

/**
 * Fetch the latest 'Salary' income entry for each person.
 * Returns an array of { who, amount_cents, income_date, name }.
 */
export async function fetchLatestSalaries() {
  return request('/income/latest-salary');
}

/**
 * POST one or more income entries in a single batch call.
 * @param {Array<{ name, amount_cents, who, category, income_date }>} entries
 * @param {string} [month]  YYYY-MM to refresh income analytics after save
 */
export async function createIncome(entries, month) {
  const data = await request('/income', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(entries),
  });
  // Refresh income analytics so the chart updates immediately
  if (month) await fetchIncomeByPerson(month);
  return data;
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

/** Fetch all projects with computed stats and update projects store. */
export async function fetchProjects() {
  const data = await request('/projects');
  projects.set(data);
  return data;
}

/**
 * Create a new project.
 * @param {{ name: string, target_cents: number, target_date: string }} payload
 */
export async function createProject(payload) {
  const data = await request('/projects', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  projects.update((prev) => [...prev, data]);
  return data;
}

/**
 * Update a project by id.
 * @param {number} id
 * @param {{ name?: string, target_cents?: number, target_date?: string }} payload
 */
export async function updateProject(id, payload) {
  const data = await request(`/projects/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  projects.update((prev) => prev.map((p) => (p.id === id ? data : p)));
  return data;
}

/**
 * Delete a project. Associated expenses keep their history (project_id → NULL).
 * @param {number} id
 */
export async function deleteProject(id) {
  const res = await fetch(`${BASE}/projects/${id}`, { method: 'DELETE' });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`API DELETE /projects/${id} → ${res.status}: ${body}`);
  }
  projects.update((prev) => prev.filter((p) => p.id !== id));
}

// ---------------------------------------------------------------------------
// Bootstrap
// ---------------------------------------------------------------------------

/** Convenience — bootstrap all store data in one call (used in App.svelte onMount). */
export async function fetchAllData(month) {
  await Promise.all([
    fetchUsers(true),  // load all users (active + inactive) — UI filters as needed
    fetchExpenses(),
    fetchSplits(),
    fetchAnalytics(month),
    fetchIncomeByPerson(month),
    fetchPaybacks(month),
    fetchProjects(),
    fetchBudgets(),
    fetchRecurring(),
    fetchSettlements(),
  ]);
}

// ---------------------------------------------------------------------------
// Recurring expenses
// ---------------------------------------------------------------------------

/** Fetch all recurring expense templates and update the store. */
export async function fetchRecurring() {
  const data = await request('/recurring');
  recurringExpenses.set(data);
  return data;
}

/**
 * Create a recurring expense template.
 * @param {{ name, cost_cents, who_paid, category, day_of_month }} payload
 */
export async function createRecurring(payload) {
  const data = await request('/recurring', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  recurringExpenses.update((prev) => [...prev, data]);
  return data;
}

/**
 * Delete a recurring expense template by id.
 * @param {number} id
 */
export async function deleteRecurring(id) {
  const res = await fetch(`${BASE}/recurring/${id}`, { method: 'DELETE' });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`API DELETE /recurring/${id} \u2192 ${res.status}: ${body}`);
  }
  recurringExpenses.update((prev) => prev.filter((r) => r.id !== id));
}

// ---------------------------------------------------------------------------
// Budgets
// ---------------------------------------------------------------------------

/** Fetch all raw budget config rows and update the store. */
export async function fetchBudgets() {
  const data = await request('/budgets');
  budgets.set(data);
  return data;
}

/**
 * Upsert a budget limit.
 * @param {{ category, month, limit_cents }} payload
 */
export async function upsertBudget(payload) {
  const data = await request('/budgets', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  budgets.update((prev) => {
    const idx = prev.findIndex((b) => b.category === data.category && b.month === data.month);
    return idx >= 0 ? prev.map((b, i) => (i === idx ? data : b)) : [...prev, data];
  });
  return data;
}

/**
 * Delete a budget row.
 * @param {string} category
 * @param {string} month  YYYY-MM or 'ALL'
 */
export async function deleteBudget(category, month) {
  const res = await fetch(`${BASE}/budgets/${encodeURIComponent(category)}/${encodeURIComponent(month)}`, { method: 'DELETE' });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`API DELETE /budgets \u2192 ${res.status}: ${body}`);
  }
  budgets.update((prev) => prev.filter((b) => !(b.category === category && b.month === month)));
}

/**
 * Fetch budget status (actuals vs. limits) for a month.
 * Returns BudgetStatusRow[]: { category, limit_cents, actual_cents, pct_used }
 * @param {string} [month]  YYYY-MM
 */
export async function fetchBudgetAnalytics(month) {
  const qs = month ? `?month=${encodeURIComponent(month)}` : '';
  return request(`/analytics/budgets${qs}`);
}

// ---------------------------------------------------------------------------
// Settlements
// ---------------------------------------------------------------------------

/** Fetch all locked months and update the settlements store. */
export async function fetchSettlements() {
  const data = await request('/settlements');
  settlements.set(data);
  return data;
}

/**
 * Lock a month.
 * @param {{ month: string, net_balance_transferred_cents: number }} payload
 */
export async function createSettlement(payload) {
  const data = await request('/settlements', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  settlements.update((prev) => [data, ...prev]);
  return data;
}
