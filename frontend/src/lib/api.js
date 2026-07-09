/**
 * api.js — All HTTP interactions with the FastAPI backend.
 * Functions update the shared Svelte stores after every successful fetch.
 * Integrates client-side encryption/decryption transparently.
 */

import { get } from 'svelte/store';
import { cryptoKey } from './stores.js';
import { encryptText, decryptText } from './crypto.js';
import { expenses, splits, analytics, incomeAnalytics, paybacks, projects, budgets, recurringExpenses, settlements, users, tags } from './stores.js';

const BASE = `/api`;

// ---------------------------------------------------------------------------
// Encryption / Decryption Helpers
// ---------------------------------------------------------------------------

export async function enc(txt) {
  const key = get(cryptoKey);
  if (!key) return txt;
  return encryptText(txt, key);
}

export async function dec(txt) {
  const key = get(cryptoKey);
  if (!key) return txt;
  return decryptText(txt, key);
}

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

export async function fetchUsers(includeDeactivated = false) {
  const qs = includeDeactivated ? '?include_deactivated=true' : '';
  const data = await request(`/users${qs}`);
  
  // Decrypt users
  const decrypted = await Promise.all(
    data.map(async (u) => ({
      ...u,
      name: await dec(u.name)
    }))
  );
  
  users.set(decrypted);
  return decrypted;
}

export async function createUser(payload) {
  const encryptedPayload = {
    ...payload,
    name: await enc(payload.name)
  };
  
  const data = await request('/users', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(encryptedPayload),
  });
  
  const decrypted = {
    ...data,
    name: await dec(data.name)
  };
  
  users.update((prev) => [...prev, decrypted]);
  return decrypted;
}

export async function updateUser(name, payload) {
  const encName = await enc(name);
  const data = await request(`/users/${encodeURIComponent(encName)}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload), // payload only color/is_active, no encryption needed
  });
  
  const decrypted = {
    ...data,
    name: await dec(data.name)
  };
  
  users.update((prev) => prev.map((u) => (u.name === name ? decrypted : u)));
  return decrypted;
}

export async function deleteUser(name) {
  const encName = await enc(name);
  const res = await fetch(`${BASE}/users/${encodeURIComponent(encName)}`, { method: 'DELETE' });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`API DELETE /users/${name} → ${res.status}: ${body}`);
  }
  users.update((prev) => prev.filter((u) => u.name !== name));
}

// ---------------------------------------------------------------------------
// Expenses
// ---------------------------------------------------------------------------

async function decryptExpense(e) {
  return {
    ...e,
    name: await dec(e.name),
    category: await dec(e.category),
    who_paid: await dec(e.who_paid),
    overrides: e.overrides
      ? await Promise.all(
          e.overrides.map(async (o) => ({
            ...o,
            user_name: await dec(o.user_name)
          }))
        )
      : []
  };
}

export async function fetchExpenses() {
  const data = await request('/expenses');
  const decrypted = await Promise.all(data.map(decryptExpense));
  expenses.set(decrypted);
  return decrypted;
}

export async function createExpense(payload, month) {
  const encryptedPayload = {
    ...payload,
    name: await enc(payload.name),
    category: await enc(payload.category),
    who_paid: await enc(payload.who_paid),
    overrides: payload.overrides
      ? await Promise.all(
          payload.overrides.map(async (o) => ({
            ...o,
            user_name: await enc(o.user_name)
          }))
        )
      : []
  };

  const data = await request('/expenses', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(encryptedPayload),
  });
  
  const decrypted = await decryptExpense(data);
  expenses.update((prev) => [decrypted, ...prev]);
  if (month) await fetchAnalytics(month);
  return decrypted;
}

export async function deleteExpense(id, month) {
  const res = await fetch(`${BASE}/expenses/${id}`, { method: 'DELETE' });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`API DELETE /expenses/${id} → ${res.status}: ${body}`);
  }
  expenses.update((prev) => prev.filter((e) => e.id !== id));
  if (month) await fetchAnalytics(month);
}

export async function updateExpense(id, payload, month) {
  const encryptedPayload = { ...payload };
  if (payload.name !== undefined) encryptedPayload.name = await enc(payload.name);
  if (payload.category !== undefined) encryptedPayload.category = await enc(payload.category);
  if (payload.who_paid !== undefined) encryptedPayload.who_paid = await enc(payload.who_paid);
  if (payload.overrides !== undefined) {
    encryptedPayload.overrides = await Promise.all(
      payload.overrides.map(async (o) => ({
        ...o,
        user_name: await enc(o.user_name)
      }))
    );
  }

  const data = await request(`/expenses/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(encryptedPayload),
  });
  
  const decrypted = await decryptExpense(data);
  expenses.update((prev) => prev.map((e) => (e.id === id ? decrypted : e)));
  if (month) await fetchAnalytics(month);
  return decrypted;
}

// ---------------------------------------------------------------------------
// Splits
// ---------------------------------------------------------------------------

async function decryptSplit(s) {
  return {
    ...s,
    category: await dec(s.category),
    allocations: s.allocations
      ? await Promise.all(
          s.allocations.map(async (a) => ({
            ...a,
            user_name: await dec(a.user_name)
          }))
        )
      : []
  };
}

export async function fetchSplits() {
  const data = await request('/splits');
  const decrypted = await Promise.all(data.map(decryptSplit));
  splits.set(decrypted);
  return decrypted;
}

export async function createSplit(payload) {
  const encryptedPayload = {
    category: await enc(payload.category),
    allocations: await Promise.all(
      payload.allocations.map(async (a) => ({
        ...a,
        user_name: await enc(a.user_name)
      }))
    )
  };

  const data = await request('/splits', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(encryptedPayload),
  });
  
  const decrypted = await decryptSplit(data);
  splits.update((prev) => [...prev, decrypted]);
  return decrypted;
}

export async function updateSplit(category, payload) {
  const encCategory = await enc(category);
  const encryptedPayload = {
    allocations: await Promise.all(
      payload.allocations.map(async (a) => ({
        ...a,
        user_name: await enc(a.user_name)
      }))
    )
  };

  const data = await request(`/splits/${encodeURIComponent(encCategory)}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(encryptedPayload),
  });
  
  const decrypted = await decryptSplit(data);
  splits.update((prev) => prev.map((s) => (s.category === category ? decrypted : s)));
  return decrypted;
}

// ---------------------------------------------------------------------------
// Analytics
// ---------------------------------------------------------------------------

export async function fetchAnalytics(month) {
  const qs = month ? `?month=${encodeURIComponent(month)}` : '';
  const [monthly_total, by_category, by_payer] = await Promise.all([
    request(`/analytics/monthly-total${qs}`),
    request(`/analytics/by-category${qs}`),
    request(`/analytics/by-payer${qs}`),
  ]);

  const decryptedByCategory = await Promise.all(
    by_category.map(async (c) => ({
      ...c,
      category: await dec(c.category)
    }))
  );

  const decryptedByPayer = await Promise.all(
    by_payer.map(async (p) => ({
      ...p,
      who_paid: await dec(p.who_paid)
    }))
  );

  const finalData = {
    monthly_total,
    by_category: decryptedByCategory,
    by_payer: decryptedByPayer
  };

  analytics.set(finalData);
  return finalData;
}

export async function fetchPaybacks(month) {
  const encPersonal = [await enc('PERSONAL COST'), await enc('LEISURE'), await enc('GIFT')].join(',');
  const encCombinedFixed = await enc('Combined Fixed');
  const encApartment = await enc('Apartment');
  const encZina = await enc('Zina');
  const encJim = await enc('Jim');

  const params = new URLSearchParams({
    personal_cats: encPersonal,
    combined_fixed_cat: encCombinedFixed,
    apartment_cat: encApartment,
    zina_name: encZina,
    jim_name: encJim
  });
  if (month) params.append('month', month);

  const qs = `?${params.toString()}`;
  const data = await request(`/analytics/paybacks${qs}`);
  
  const decrypted = {
    ...data,
    rows: data.rows
      ? await Promise.all(
          data.rows.map(async (r) => ({
            ...r,
            category: await dec(r.category)
          }))
        )
      : [],
    debts: data.debts
      ? await Promise.all(
          data.debts.map(async (d) => ({
            ...d,
            from_user: await dec(d.from_user),
            to_user: await dec(d.to_user)
          }))
        )
      : []
  };

  paybacks.set(decrypted);
  return decrypted;
}

// ---------------------------------------------------------------------------
// Income
// ---------------------------------------------------------------------------

export async function fetchIncomeByPerson(month) {
  const encSalary = await enc('SALARY');
  const params = new URLSearchParams({ salary_cat: encSalary });
  if (month) params.append('month', month);

  const qs = `?${params.toString()}`;
  const data = await request(`/analytics/income-by-person${qs}`);
  
  const decrypted = await Promise.all(
    data.map(async (i) => ({
      ...i,
      who: await dec(i.who)
    }))
  );

  incomeAnalytics.set(decrypted);
  return decrypted;
}

export async function fetchLatestSalaries() {
  const encSalary = await enc('SALARY');
  const qs = `?salary_cat=${encodeURIComponent(encSalary)}`;
  const data = await request(`/income/latest-salary${qs}`);
  return Promise.all(
    data.map(async (s) => ({
      ...s,
      who: await dec(s.who),
      name: await dec(s.name)
    }))
  );
}

export async function createIncome(entries, month) {
  const encryptedEntries = await Promise.all(
    entries.map(async (e) => ({
      ...e,
      name: await enc(e.name),
      who: await enc(e.who),
      category: await enc(e.category)
    }))
  );

  const data = await request('/income', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(encryptedEntries),
  });
  
  if (month) await fetchIncomeByPerson(month);
  return data;
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

async function decryptProject(p) {
  return {
    ...p,
    name: await dec(p.name),
    last_payment: p.last_payment
      ? {
          ...p.last_payment,
          who_paid: await dec(p.last_payment.who_paid),
          name: await dec(p.last_payment.name)
        }
      : null
  };
}

export async function fetchProjects() {
  const data = await request('/projects');
  const decrypted = await Promise.all(data.map(decryptProject));
  projects.set(decrypted);
  return decrypted;
}

export async function createProject(payload) {
  const encryptedPayload = {
    ...payload,
    name: await enc(payload.name)
  };

  const data = await request('/projects', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(encryptedPayload),
  });
  
  const decrypted = await decryptProject(data);
  projects.update((prev) => [...prev, decrypted]);
  return decrypted;
}

export async function updateProject(id, payload) {
  const encryptedPayload = { ...payload };
  if (payload.name !== undefined) encryptedPayload.name = await enc(payload.name);

  const data = await request(`/projects/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(encryptedPayload),
  });
  
  const decrypted = await decryptProject(data);
  projects.update((prev) => prev.map((p) => (p.id === id ? decrypted : p)));
  return decrypted;
}

export async function deleteProject(id) {
  const res = await fetch(`${BASE}/projects/${id}`, { method: 'DELETE' });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`API DELETE /projects/${id} → ${res.status}: ${body}`);
  }
  projects.update((prev) => prev.filter((p) => p.id !== id));
}

// ---------------------------------------------------------------------------
// Tags
// ---------------------------------------------------------------------------

async function decryptTag(t) {
  return {
    ...t,
    name: await dec(t.name),
    description: t.description ? await dec(t.description) : null,
  };
}

export async function fetchTags() {
  const data = await request('/tags');
  const decrypted = await Promise.all(data.map(decryptTag));
  tags.set(decrypted);
  return decrypted;
}

export async function createTag(payload) {
  const encryptedPayload = {
    ...payload,
    name: await enc(payload.name),
    description: payload.description ? await enc(payload.description) : null,
  };
  const data = await request('/tags', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(encryptedPayload),
  });
  const decrypted = await decryptTag(data);
  tags.update((prev) => [...prev, decrypted]);
  return decrypted;
}

export async function updateTag(id, payload) {
  const encryptedPayload = { ...payload };
  if (payload.name !== undefined)        encryptedPayload.name = await enc(payload.name);
  if (payload.description !== undefined) {
    encryptedPayload.description = payload.description ? await enc(payload.description) : null;
  }
  const data = await request(`/tags/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(encryptedPayload),
  });
  const decrypted = await decryptTag(data);
  tags.update((prev) => prev.map((t) => (t.id === id ? { ...decrypted, total_amount: t.total_amount, expense_count: t.expense_count, first_date: t.first_date, last_date: t.last_date } : t)));
  return decrypted;
}

export async function deleteTag(id) {
  const res = await fetch(`${BASE}/tags/${id}`, { method: 'DELETE' });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`API DELETE /tags/${id} → ${res.status}: ${body}`);
  }
  tags.update((prev) => prev.filter((t) => t.id !== id));
}

export async function fetchTagAnalytics(tagId) {
  const data = await request(`/analytics/tags/${tagId}`);
  // Decrypt category names in by_category breakdown
  const decryptedByCategory = await Promise.all(
    (data.by_category ?? []).map(async (c) => ({
      ...c,
      category: await dec(c.category),
    }))
  );
  return {
    tag: await decryptTag(data.tag),
    by_month: data.by_month ?? [],
    by_category: decryptedByCategory,
  };
}

// ---------------------------------------------------------------------------
// Bootstrap / Import / Export
// ---------------------------------------------------------------------------

export async function fetchAllData(month) {
  await Promise.all([
    fetchUsers(true),
    fetchExpenses(),
    fetchSplits(),
    fetchAnalytics(month),
    fetchIncomeByPerson(month),
    fetchPaybacks(month),
    fetchProjects(),
    fetchTags(),
    fetchBudgets(),
    fetchRecurring(),
    fetchSettlements(),
  ]);
}

export async function exportDatabase(saltText) {
  const res = await fetch(`${BASE}/auth/export`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ value: saltText })
  });
  if (!res.ok) {
    const errText = await res.text();
    throw new Error(`Export failed: ${errText}`);
  }
  const blob = await res.blob();
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = 'jizifin_decrypted.db';
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

// ---------------------------------------------------------------------------
// Recurring expenses
// ---------------------------------------------------------------------------

async function decryptRecurring(r) {
  return {
    ...r,
    name: await dec(r.name),
    who_paid: await dec(r.who_paid),
    category: await dec(r.category)
  };
}

export async function fetchRecurring() {
  const data = await request('/recurring');
  const decrypted = await Promise.all(data.map(decryptRecurring));
  recurringExpenses.set(decrypted);
  return decrypted;
}

export async function createRecurring(payload) {
  const encryptedPayload = {
    ...payload,
    name: await enc(payload.name),
    who_paid: await enc(payload.who_paid),
    category: await enc(payload.category)
  };

  const data = await request('/recurring', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(encryptedPayload),
  });
  
  const decrypted = await decryptRecurring(data);
  recurringExpenses.update((prev) => [...prev, decrypted]);
  return decrypted;
}

export async function deleteRecurring(id) {
  const res = await fetch(`${BASE}/recurring/${id}`, { method: 'DELETE' });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`API DELETE /recurring/${id} → ${res.status}: ${body}`);
  }
  recurringExpenses.update((prev) => prev.filter((r) => r.id !== id));
}

// ---------------------------------------------------------------------------
// Budgets
// ---------------------------------------------------------------------------

async function decryptBudget(b) {
  return {
    ...b,
    category: await dec(b.category)
  };
}

export async function fetchBudgets() {
  const data = await request('/budgets');
  const decrypted = await Promise.all(data.map(decryptBudget));
  budgets.set(decrypted);
  return decrypted;
}

export async function upsertBudget(payload) {
  const encryptedPayload = {
    ...payload,
    category: await enc(payload.category)
  };

  const data = await request('/budgets', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(encryptedPayload),
  });
  
  const decrypted = await decryptBudget(data);
  budgets.update((prev) => {
    const idx = prev.findIndex((b) => b.category === decrypted.category && b.month === decrypted.month);
    return idx >= 0 ? prev.map((b, i) => (i === idx ? decrypted : b)) : [...prev, decrypted];
  });
  return decrypted;
}

export async function deleteBudget(category, month) {
  const encCategory = await enc(category);
  const res = await fetch(`${BASE}/budgets/${encodeURIComponent(encCategory)}/${encodeURIComponent(month)}`, { method: 'DELETE' });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`API DELETE /budgets → ${res.status}: ${body}`);
  }
  budgets.update((prev) => prev.filter((b) => !(b.category === category && b.month === month)));
}

export async function fetchBudgetAnalytics(month) {
  const qs = month ? `?month=${encodeURIComponent(month)}` : '';
  const data = await request(`/analytics/budgets${qs}`);
  return Promise.all(
    data.map(async (r) => ({
      ...r,
      category: await dec(r.category)
    }))
  );
}

// ---------------------------------------------------------------------------
// Settlements
// ---------------------------------------------------------------------------

export async function fetchSettlements() {
  const data = await request('/settlements');
  settlements.set(data);
  return data;
}

export async function createSettlement(payload) {
  const data = await request('/settlements', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  settlements.update((prev) => [data, ...prev]);
  return data;
}
