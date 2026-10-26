/**
 * api.js — v2 client for the Rust backend.
 *
 * The browser only encrypts input, decrypts labels (memoized per ciphertext)
 * and renders. Every number shown comes from the server: contentious values
 * from the Auditor (`AuditedResult`), aggregates from `/views/*`.
 */

import { get } from 'svelte/store';
import { encryptText, decryptText, deriveKey } from './crypto.js';
import {
  cryptoKey, sessionToken, members, categories, accounts, projects, tags, jobs,
  recurring, budgets, offsetRules, seals,
} from './stores.js';

const BASE = '/api';

/** Client proof plaintext; the server stores only SHA-256 of its ciphertext. */
const AUTH_PROOF = 'FinanceTrackerAuth';

// ---------------------------------------------------------------------------
// HTTP
// ---------------------------------------------------------------------------

/** Non-2xx response; `message` is the server's `detail`. */
export class ApiError extends Error {
  constructor(status, detail) {
    super(detail);
    this.status = status;
  }
}

function authHeaders(extra = {}) {
  const token = get(sessionToken);
  return token ? { ...extra, Authorization: `Bearer ${token}` } : extra;
}

async function failure(res) {
  const text = await res.text();
  try {
    return new ApiError(res.status, JSON.parse(text).detail ?? text);
  } catch {
    return new ApiError(res.status, text || res.statusText);
  }
}

/** JSON request; returns the parsed body, or null for 204. */
export async function request(path, { method = 'GET', body } = {}) {
  const headers = authHeaders(body === undefined ? {} : { 'Content-Type': 'application/json' });
  const res = await fetch(`${BASE}${path}`, { method, headers, body: body === undefined ? undefined : JSON.stringify(body) });
  if (res.status === 401 && path !== '/auth/login') sessionToken.set('');
  if (!res.ok) throw await failure(res);
  return res.status === 204 ? null : res.json();
}

function query(params) {
  const qs = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value === null || value === undefined || value === '') continue;
    qs.set(key, Array.isArray(value) ? value.join(',') : String(value));
  }
  const text = qs.toString();
  return text ? `?${text}` : '';
}

// ---------------------------------------------------------------------------
// Cipher: encrypt input, decrypt labels once per distinct ciphertext
// ---------------------------------------------------------------------------

const opened = new Map();

/** Encrypts `text`; empty stays null. */
export async function seal(text) {
  const trimmed = typeof text === 'string' ? text.trim() : text;
  if (!trimmed) return null;
  return encryptText(trimmed, get(cryptoKey));
}

/** Decrypts `ct`, memoized. */
export async function open(ct) {
  if (!ct) return ct ?? '';
  if (!opened.has(ct)) opened.set(ct, decryptText(ct, get(cryptoKey)));
  return opened.get(ct);
}

/** Copies of `rows` with `fields` decrypted in place. */
export async function openRows(rows, fields) {
  return Promise.all(rows.map((row) => openRow(row, fields)));
}

async function openRow(row, fields) {
  const copy = { ...row };
  await Promise.all(fields.map(async (f) => { copy[f] = await open(row[f]); }));
  return copy;
}

/** Copy of `payload` with `fields` encrypted. */
async function sealRow(payload, fields) {
  const copy = { ...payload };
  await Promise.all(fields.map(async (f) => { copy[f] = await seal(payload[f]); }));
  return copy;
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

async function proofFor(passphrase) {
  const key = await deriveKey(passphrase);
  return { key, proof: await encryptText(AUTH_PROOF, key) };
}

async function startSession(path, passphrase) {
  const { key, proof } = await proofFor(passphrase);
  const { token } = await request(path, { method: 'POST', body: { proof } });
  opened.clear();
  cryptoKey.set(key);
  sessionToken.set(token);
}

export const auth = {
  status: () => request('/auth/status'),
  /** First boot: stores the proof hash and opens a session. */
  init: (passphrase) => startSession('/auth/init', passphrase),
  login: (passphrase) => startSession('/auth/login', passphrase),
  async logout() {
    await request('/auth/logout', { method: 'POST' }).catch(() => null);
    sessionToken.set('');
    cryptoKey.set(null);
    opened.clear();
  },
  /** Wipes every table; requires the passphrase proof. */
  async reset(passphrase) {
    const { proof } = await proofFor(passphrase);
    await request('/auth/reset', { method: 'POST', body: { proof } });
    sessionToken.set('');
    cryptoKey.set(null);
  },
};

// ---------------------------------------------------------------------------
// Catalog resources: list into a store, writes refresh it
// ---------------------------------------------------------------------------

function resource(path, ciphered, store) {
  const api = {
    async list() {
      const rows = await openRows(await request(path), ciphered);
      store?.set(rows);
      return rows;
    },
    async create(payload) {
      const { id } = await request(path, { method: 'POST', body: await sealRow(payload, ciphered) });
      await api.list();
      return id;
    },
    async update(id, payload) {
      await request(`${path}/${id}`, { method: 'PUT', body: await sealRow(payload, ciphered) });
      await api.list();
    },
    async remove(id) {
      await request(`${path}/${id}`, { method: 'DELETE' });
      await api.list();
    },
  };
  return api;
}

export const memberApi = resource('/members', ['name'], members);
export const categoryApi = resource('/categories', ['name'], categories);
export const tagApi = resource('/tags', ['name', 'description'], tags);
export const jobApi = {
  ...resource('/jobs', ['name', 'notes'], jobs),
  /** Pay change from `effective`: ends the job the day before and starts its successor. */
  async adjust(id, { effective, amount, notes }) {
    const body = { effective, amount, notes: await seal(notes) };
    const { id: next } = await request(`/jobs/${id}/adjust`, { method: 'POST', body });
    await jobApi.list();
    return next;
  },
};
export const recurringApi = resource('/recurring', ['name'], recurring);
export const budgetApi = resource('/budgets', [], budgets);
export const accountApi = resource('/accounts', ['name'], accounts);
export const projectApi = resource('/projects', ['name'], projects);
export const offsetRuleApi = resource('/offset-rules', [], offsetRules);

/** SCD2 split agreements of one category. */
export const agreementApi = {
  async list(category) {
    return openRows(await request(`/categories/${category}/agreements`), ['note']);
  },
  async create(category, payload) {
    return (await request(`/categories/${category}/agreements`, { method: 'POST', body: await sealRow(payload, ['note']) })).id;
  },
  async update(id, payload) {
    await request(`/agreements/${id}`, { method: 'PUT', body: await sealRow(payload, ['note']) });
  },
  remove: (id) => request(`/agreements/${id}`, { method: 'DELETE' }),
  /** Replaces the open-ended baseline shares of `category`. */
  setBaseline: (category, shares) => request(`/categories/${category}/baseline`, { method: 'PUT', body: { shares } }),
};

/** Month-specific income replacements. */
export const incomeOverrideApi = {
  async list(month) {
    return openRows(await request(`/income-overrides${query({ month })}`), ['note']);
  },
  async upsert(payload) {
    await request('/income-overrides', { method: 'PUT', body: await sealRow(payload, ['note']) });
  },
  remove: (member, month) => request(`/income-overrides/${member}/${month}`, { method: 'DELETE' }),
};

// ---------------------------------------------------------------------------
// Ledger
// ---------------------------------------------------------------------------

const ENTRY_TEXT = ['name', 'note'];

export const entryApi = {
  /** One keyset page: `{ entries, next_cursor }`. */
  async page(filter = {}) {
    const page = await request(`/entries${query(filter)}`);
    return { ...page, entries: await openRows(page.entries, ENTRY_TEXT) };
  },
  async get(id) {
    return openRow(await request(`/entries/${id}`), ENTRY_TEXT);
  },
  /** Resolved liability of one expense: `{ entry, source, total, shares: { memberId: cents } }`. */
  shares: (id) => request(`/entries/${id}/shares`),
  async create(payload) {
    return (await request('/entries', { method: 'POST', body: await sealRow(payload, ENTRY_TEXT) })).id;
  },
  /** Entries in one transaction; returns their ids. */
  async batch(payloads) {
    const body = await Promise.all(payloads.map((p) => sealRow(p, ENTRY_TEXT)));
    return request('/entries/batch', { method: 'POST', body });
  },
  async update(id, payload) {
    await request(`/entries/${id}`, { method: 'PUT', body: await sealRow(payload, ENTRY_TEXT) });
  },
  remove: (id) => request(`/entries/${id}`, { method: 'DELETE' }),
  /** Same category + amount in the same month: `[{ index, matches }]`. */
  async duplicates(probes) {
    const found = await request('/entries/duplicates', { method: 'POST', body: probes });
    return Promise.all(found.map(async (d) => ({ ...d, matches: await openRows(d.matches, ENTRY_TEXT) })));
  },
};

// ---------------------------------------------------------------------------
// Auditor (contentious values) and settlements
// ---------------------------------------------------------------------------

export const audit = {
  settlement: (month, members = []) => request(`/settlements/${month}${query({ members })}`),
  seal: async (month) => {
    await request(`/settlements/${month}/seal`, { method: 'POST' });
    await audit.seals();
  },
  async seals() {
    const rows = await request('/settlements');
    seals.set(rows);
    return rows;
  },
  /** Proposed deposits; `roundTo` cents also returns them rounded up (`rounded`). */
  depositProposal: (account, month, roundTo = null) => request(`/accounts/${account}/deposit-proposal${query({ month, round_to: roundTo })}`),
  jointDashboard: (account, month) => request(`/accounts/${account}/dashboard${query({ month })}`),
  /** `{ rows, summary }`, optionally for `members` and one joint `account`. */
  budgetStatus: (month, { members = [], account } = {}) => request(`/budgets/status${query({ month, members, account })}`),
  budgetProposals: (month) => request(`/budgets/proposals${query({ month })}`),
  salaryRatios: (month) => request(`/splits/salary-ratios${query({ month })}`),
  /** Effective split of every expense category in `month`. */
  splits: (month) => request(`/splits/effective${query({ month })}`),
  /** Whole-percent proposal: strategy 'even' | 'salary' over `members` (all when empty). */
  splitProposal: (strategy, members = [], month) => request(`/splits/proposal${query({ strategy, members, month })}`),
  income: (month) => request(`/income/effective${query({ month })}`),
  cashFlow: (month, members = []) => request(`/cashflow${query({ month, members })}`),
  projectSettlement: (project) => request(`/projects/${project}/settlement`),
};

// ---------------------------------------------------------------------------
// Views (aggregates keyed by id)
// ---------------------------------------------------------------------------

export const views = {
  /** `opts`: kind, members, and one of category / member / project / account to focus on. */
  month: (month, opts = {}) => request(`/views/month${query({ month, kind: 'expense', ...opts })}`),
  series: ({ kind = 'expense', from, to } = {}) => request(`/views/series${query({ kind, from, to })}`),
  tags: () => request('/views/tags'),
  tag: (id) => request(`/views/tags/${id}`),
  projects: () => request('/views/projects'),
  recurring: (month) => request(`/views/recurring${query({ month })}`),
};

// ---------------------------------------------------------------------------
// Console and backup
// ---------------------------------------------------------------------------

/** Read-only SQL console: `{ columns, rows }`. */
export const runQuery = (sql) => request('/query', { method: 'POST', body: { sql } });

export const backup = {
  /** Ciphertext-only DuckDB export as a Blob. */
  async download() {
    const res = await fetch(`${BASE}/backup`, { headers: authHeaders() });
    if (!res.ok) throw await failure(res);
    return res.blob();
  },
  /** Replaces every table with the uploaded export. */
  async restore(file) {
    const res = await fetch(`${BASE}/backup`, { method: 'POST', headers: authHeaders(), body: file });
    if (!res.ok) throw await failure(res);
  },
};

// ---------------------------------------------------------------------------
// Bootstrap
// ---------------------------------------------------------------------------

/** Loads every catalog store; views and audits are fetched by the tabs that show them. */
export async function loadCatalog() {
  await Promise.all([
    memberApi.list(), categoryApi.list(), accountApi.list(), projectApi.list(), tagApi.list(),
    jobApi.list(), recurringApi.list(), budgetApi.list(), offsetRuleApi.list(), audit.seals(),
  ]);
}
