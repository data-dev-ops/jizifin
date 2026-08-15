import '@testing-library/jest-dom/vitest';
import { vi, beforeEach } from 'vitest';
import { webcrypto } from 'node:crypto';

// Polyfill/bind WebCrypto for jsdom environment if crypto.subtle is missing
if (!globalThis.crypto || !globalThis.crypto.subtle) {
  Object.defineProperty(globalThis, 'crypto', {
    value: webcrypto,
    writable: true,
  });
}
if (!window.crypto || !window.crypto.subtle) {
  Object.defineProperty(window, 'crypto', {
    value: webcrypto,
    writable: true,
  });
}

// Stateful mock in-memory database store
export let dbState = {};

export function resetDbState() {
  dbState = {
    users: [
      { name: 'John', color: '#6366f1', is_active: 1, created_at: '2026-01-01' },
      { name: 'Jane', color: '#ec4899', is_active: 1, created_at: '2026-01-01' }
    ],
    splits: [
      { category: 'GROCERIES', allocations: [{ user_name: 'John', pct: 50.0 }, { user_name: 'Jane', pct: 50.0 }] }
    ],
    income_categories: [
      { category: 'SALARY' }
    ],
    expenses: [],
    income: [],
    budgets: [],
    projects: [],
    tags: [],
    recurring: [],
    settlements: [],
    joint_account: { id: 1, name: 'Joint Vault', balance_cents: 0, safety_margin_pct: 10, deposit_split_mode: 'even', expected_total_cents: null },
    joint_categories: [],
    joint_deposits: [],
    joint_expected_costs: [],
    joint_corrections: [],
    auth_salt: 'mocked-salt',
    nextId: { expense: 1, income: 1, project: 1, tag: 1, recurring: 1, correction: 1 }
  };
}

resetDbState();

async function mockFetchRouter(input, init) {
  const urlStr = typeof input === 'string' ? input : (input?.url || String(input));
  const method = (init?.method || 'GET').toUpperCase();
  const body = init?.body ? JSON.parse(init.body) : null;

  // Salt & Auth
  if (urlStr.includes('/auth/salt')) {
    if (method === 'GET') {
      return new Response(JSON.stringify({ value: dbState.auth_salt }), { status: 200, headers: { 'Content-Type': 'application/json' } });
    }
    if (method === 'POST') {
      dbState.auth_salt = body.value;
      return new Response(JSON.stringify({ status: 'ok' }), { status: 200, headers: { 'Content-Type': 'application/json' } });
    }
  }

  // Users
  if (urlStr.includes('/users')) {
    if (method === 'GET') {
      return new Response(JSON.stringify(dbState.users), { status: 200, headers: { 'Content-Type': 'application/json' } });
    }
    if (method === 'POST') {
      const newUser = { name: body.name, color: body.color || '#6366f1', is_active: 1, created_at: new Date().toISOString() };
      dbState.users.push(newUser);
      return new Response(JSON.stringify(newUser), { status: 201, headers: { 'Content-Type': 'application/json' } });
    }
  }

  // Splits
  if (urlStr.includes('/splits')) {
    if (method === 'GET') {
      return new Response(JSON.stringify(dbState.splits), { status: 200, headers: { 'Content-Type': 'application/json' } });
    }
    if (method === 'POST') {
      const newSplit = { category: body.category, allocations: body.allocations || [] };
      dbState.splits.push(newSplit);
      return new Response(JSON.stringify(newSplit), { status: 201, headers: { 'Content-Type': 'application/json' } });
    }
  }

  // Expenses
  if (urlStr.includes('/expenses')) {
    if (method === 'GET') {
      return new Response(JSON.stringify(dbState.expenses), { status: 200, headers: { 'Content-Type': 'application/json' } });
    }
    if (method === 'POST') {
      const newExp = {
        id: dbState.nextId.expense++,
        name: body.name,
        cost_cents: body.cost_cents,
        expense_date: body.expense_date,
        who_paid: body.who_paid,
        category: body.category,
        project_id: body.project_id || null,
        tag_id: body.tag_id || null,
        is_joint: body.is_joint || 0,
        overrides: body.overrides || []
      };
      dbState.expenses.unshift(newExp);
      return new Response(JSON.stringify(newExp), { status: 201, headers: { 'Content-Type': 'application/json' } });
    }
    if (method === 'DELETE') {
      const idMatch = urlStr.match(/\/expenses\/(\d+)/);
      if (idMatch) {
        const id = parseInt(idMatch[1]);
        dbState.expenses = dbState.expenses.filter(e => e.id !== id);
        return new Response(null, { status: 204 });
      }
    }
  }

  // Income categories
  if (urlStr.includes('/income/categories')) {
    if (method === 'GET') {
      return new Response(JSON.stringify(dbState.income_categories), { status: 200, headers: { 'Content-Type': 'application/json' } });
    }
    if (method === 'POST') {
      const newCat = { category: body.category };
      dbState.income_categories.push(newCat);
      return new Response(JSON.stringify(newCat), { status: 201, headers: { 'Content-Type': 'application/json' } });
    }
  }

  // Latest salary
  if (urlStr.includes('/salaries') || urlStr.includes('/income/latest-salary')) {
    return new Response(JSON.stringify([
      { who: 'John', amount_cents: 300000 },
      { who: 'Jane', amount_cents: 200000 },
    ]), { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Income
  if (urlStr.includes('/income')) {
    if (method === 'GET') {
      return new Response(JSON.stringify(dbState.income), { status: 200, headers: { 'Content-Type': 'application/json' } });
    }
    if (method === 'POST') {
      const newInc = {
        id: dbState.nextId.income++,
        name: body.name,
        amount_cents: body.amount_cents,
        who: body.who,
        category: body.category,
        income_date: body.income_date,
        is_joint: body.is_joint || 0
      };
      dbState.income.unshift(newInc);
      return new Response(JSON.stringify(newInc), { status: 201, headers: { 'Content-Type': 'application/json' } });
    }
    if (method === 'DELETE') {
      const idMatch = urlStr.match(/\/income\/(\d+)/);
      if (idMatch) {
        const id = parseInt(idMatch[1]);
        dbState.income = dbState.income.filter(i => i.id !== id);
        return new Response(null, { status: 204 });
      }
    }
  }

  // Budgets
  if (urlStr.includes('/budgets') || urlStr.includes('/analytics/budget')) {
    if (method === 'GET') {
      return new Response(JSON.stringify(dbState.budgets), { status: 200, headers: { 'Content-Type': 'application/json' } });
    }
    if (method === 'POST') {
      const newBdg = { category: body.category, month: body.month, limit_cents: body.limit_cents };
      dbState.budgets = dbState.budgets.filter(b => !(b.category === body.category && b.month === body.month));
      dbState.budgets.push(newBdg);
      return new Response(JSON.stringify(newBdg), { status: 201, headers: { 'Content-Type': 'application/json' } });
    }
  }

  // Projects
  if (urlStr.includes('/projects')) {
    if (method === 'GET') {
      return new Response(JSON.stringify(dbState.projects), { status: 200, headers: { 'Content-Type': 'application/json' } });
    }
    if (method === 'POST') {
      const newProj = {
        id: dbState.nextId.project++,
        name: body.name,
        target_cents: body.target_cents,
        target_date: body.target_date,
        total_spent_cents: 0,
        expense_count: 0
      };
      dbState.projects.push(newProj);
      return new Response(JSON.stringify(newProj), { status: 201, headers: { 'Content-Type': 'application/json' } });
    }
  }

  // Tags
  if (urlStr.includes('/tags')) {
    if (method === 'GET') {
      return new Response(JSON.stringify(dbState.tags), { status: 200, headers: { 'Content-Type': 'application/json' } });
    }
    if (method === 'POST') {
      const newTag = {
        id: dbState.nextId.tag++,
        name: body.name,
        color: body.color || '#f59e0b',
        description: body.description || null,
        total_amount: 0.0,
        expense_count: 0
      };
      dbState.tags.push(newTag);
      return new Response(JSON.stringify(newTag), { status: 201, headers: { 'Content-Type': 'application/json' } });
    }
  }

  // Paybacks
  if (urlStr.includes('/paybacks')) {
    return new Response(JSON.stringify({ rows: [], debts: [], month: '' }), { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Analytics endpoints
  if (urlStr.includes('/analytics/monthly-total')) {
    const totalCents = dbState.expenses.reduce((acc, e) => acc + e.cost_cents, 0);
    return new Response(JSON.stringify({ total_amount: totalCents / 100.0, expense_count: dbState.expenses.length, month: '2026-07' }), { status: 200, headers: { 'Content-Type': 'application/json' } });
  }
  if (urlStr.includes('/analytics/by-category') || urlStr.includes('/analytics/by-payer') || urlStr.includes('/analytics/income-by-person')) {
    return new Response(JSON.stringify([]), { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Query console
  if (urlStr.includes('/query')) {
    return new Response(JSON.stringify({ columns: [], rows: [], row_count: 0, truncated: false }), { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Joint account routes
  if (urlStr.includes('/joint-account/categories')) {
    if (method === 'GET') return new Response(JSON.stringify(dbState.joint_categories), { status: 200, headers: { 'Content-Type': 'application/json' } });
    if (method === 'POST') {
      dbState.joint_categories.push(body.category);
      return new Response(JSON.stringify({ status: 'ok' }), { status: 200, headers: { 'Content-Type': 'application/json' } });
    }
  }
  if (urlStr.includes('/joint-account/deposits')) {
    if (method === 'GET') return new Response(JSON.stringify(dbState.joint_deposits), { status: 200, headers: { 'Content-Type': 'application/json' } });
    if (method === 'POST') {
      dbState.joint_deposits = dbState.joint_deposits.filter(d => d.user_name !== body.user_name);
      dbState.joint_deposits.push(body);
      return new Response(JSON.stringify(body), { status: 200, headers: { 'Content-Type': 'application/json' } });
    }
  }
  if (urlStr.includes('/joint-account/monthly-deposits')) {
    return new Response(JSON.stringify([]), { status: 200, headers: { 'Content-Type': 'application/json' } });
  }
  if (urlStr.includes('/joint-account/expected-costs')) {
    if (method === 'GET') return new Response(JSON.stringify(dbState.joint_expected_costs), { status: 200, headers: { 'Content-Type': 'application/json' } });
  }
  if (urlStr.includes('/joint-account/corrections')) {
    if (method === 'GET') return new Response(JSON.stringify(dbState.joint_corrections), { status: 200, headers: { 'Content-Type': 'application/json' } });
    if (method === 'POST') {
      const newCorr = { id: dbState.nextId.correction++, amount_cents: body.amount_cents, correction_date: body.correction_date, note: body.note || null };
      dbState.joint_corrections.push(newCorr);
      dbState.joint_account.balance_cents += body.amount_cents;
      return new Response(JSON.stringify(newCorr), { status: 201, headers: { 'Content-Type': 'application/json' } });
    }
  }
  if (urlStr.includes('/joint-account/dashboard')) {
    return new Response(null, { status: 404 });
  }
  if (urlStr.includes('/joint-account')) {
    return new Response(JSON.stringify(dbState.joint_account), { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  return new Response(JSON.stringify([]), { status: 200, headers: { 'Content-Type': 'application/json' } });
}

// Mock Canvas 2D context for Chart.js
if (typeof HTMLCanvasElement !== 'undefined') {
  HTMLCanvasElement.prototype.getContext = vi.fn(() => ({
    fillRect: vi.fn(),
    clearRect: vi.fn(),
    getImageData: vi.fn(() => ({ data: new Array(4) })),
    putImageData: vi.fn(),
    createImageData: vi.fn(() => []),
    setTransform: vi.fn(),
    drawImage: vi.fn(),
    save: vi.fn(),
    fillText: vi.fn(),
    restore: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    closePath: vi.fn(),
    stroke: vi.fn(),
    translate: vi.fn(),
    scale: vi.fn(),
    rotate: vi.fn(),
    arc: vi.fn(),
    fill: vi.fn(),
    measureText: vi.fn(() => ({ width: 0 })),
    transform: vi.fn(),
    rect: vi.fn(),
    clip: vi.fn(),
  }));
}

// Mock ResizeObserver
if (typeof window !== 'undefined' && !window.ResizeObserver) {
  window.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
}

// Mock matchMedia
if (typeof window !== 'undefined' && !window.matchMedia) {
  window.matchMedia = vi.fn().mockImplementation((query) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }));
}

// Setup fetch router on both globalThis and window, and clear localStorage / reset state before each test
beforeEach(() => {
  localStorage.clear();
  resetDbState();
  globalThis.fetch = mockFetchRouter;
  if (typeof window !== 'undefined') {
    window.fetch = mockFetchRouter;
  }
});
