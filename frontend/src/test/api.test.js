import { describe, it, expect, beforeEach, vi } from 'vitest';
import { enc, dec, fetchUsers, createUser, fetchIncomeCategories, fetchJobs, createJob } from '../lib/api.js';
import { jobs } from '../lib/stores.js';
import { cryptoKey, users, incomeCategories } from '../lib/stores.js';
import { deriveKey } from '../lib/crypto.js';
import { get } from 'svelte/store';

describe('api.js — API Service Layer & Encryption Wrappers', () => {
  let mockKey;

  beforeEach(async () => {
    mockKey = await deriveKey('api-test-pass');
    cryptoKey.set(mockKey);
    users.set([]);
    vi.restoreAllMocks();
  });

  describe('enc & dec helpers', () => {
    it.each([
      { plaintext: 'John' },
      { plaintext: 'Household Expenses' },
    ])('encrypts and decrypts values when cryptoKey is set ($plaintext)', async ({ plaintext }) => {
      const encrypted = await enc(plaintext);
      expect(encrypted).not.toBe(plaintext);

      const decrypted = await dec(encrypted);
      expect(decrypted).toBe(plaintext);
    });

    it.each([
      { plaintext: 'Jane' },
      { plaintext: 'Groceries' },
    ])('returns text as-is when cryptoKey is null ($plaintext)', async ({ plaintext }) => {
      cryptoKey.set(null);

      const encrypted = await enc(plaintext);
      expect(encrypted).toBe(plaintext);

      const decrypted = await dec(plaintext);
      expect(decrypted).toBe(plaintext);
    });

    it.each([
      { w1Text: 'John', w2Text: 'Jane', expected: 'John Jane' },
      { w1Text: 'Food', w2Text: 'Dining', expected: 'Food Dining' },
    ])('handles space-separated string decryption (multi-word text: $expected)', async ({ w1Text, w2Text, expected }) => {
      const w1 = await enc(w1Text);
      const w2 = await enc(w2Text);
      const combined = `${w1} ${w2}`;

      const decrypted = await dec(combined);
      expect(decrypted).toBe(expected);
    });

    it.each([
      { val: '' },
      { val: null },
    ])('handles empty/falsy values gracefully ($val)', async ({ val }) => {
      expect(await enc(val)).toBe(val);
      expect(await dec(val)).toBe(val);
    });
  });

  describe('API requests & User service endpoints', () => {
    it.each([
      { name: 'John', color: '#6366f1' },
      { name: 'Jane', color: '#ec4899' },
    ])('fetchUsers retrieves users, decrypts names, and updates users store ($name)', async ({ name, color }) => {
      const encName = await enc(name);
      const fakeUsers = [
        { name: encName, color: color, is_active: 1, created_at: '2026-01-01' }
      ];

      vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
        ok: true,
        json: async () => fakeUsers,
      }));

      const res = await fetchUsers(true);
      expect(res.length).toBe(1);
      expect(res[0].name).toBe(name);
      expect(get(users)).toEqual(res);
    });

    it.each([
      { name: 'Jane', color: '#ec4899' },
      { name: 'Alex', color: '#10b981' },
    ])('createUser sends encrypted payload and updates users store ($name)', async ({ name, color }) => {
      const mockUserRes = {
        name: await enc(name),
        color: color,
        is_active: 1,
        created_at: '2026-01-01'
      };

      vi.stubGlobal('fetch', vi.fn().mockImplementation(async (url, options) => {
        expect(options.method).toBe('POST');
        const body = JSON.parse(options.body);
        expect(body.name).not.toBe(name); // encrypted in transit
        return {
          ok: true,
          json: async () => mockUserRes,
        };
      }));

      const newUser = await createUser({ name: name, color: color });
      expect(newUser.name).toBe(name);
      expect(get(users)).toContainEqual(newUser);
    });

    it.each([
      { name: 'Duplicate', status: 400, body: 'User already exists', expectedErr: 'API POST /users → 400: User already exists' },
      { name: 'Invalid', status: 422, body: 'Invalid payload', expectedErr: 'API POST /users → 422: Invalid payload' },
    ])('throws error with status and body when API returns non-2xx status ($status)', async ({ name, status, body, expectedErr }) => {
      vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
        ok: false,
        status: status,
        text: async () => body,
      }));

      await expect(createUser({ name })).rejects.toThrow(expectedErr);
    });

    it.each([
      { defaults: ['SALARY', 'BONUS', 'GIFT'] },
    ])('fetchIncomeCategories seeds default categories ($defaults) if missing from DB', async ({ defaults }) => {
      incomeCategories.set([]);
      vi.stubGlobal('fetch', vi.fn().mockImplementation(async (url, options) => {
        if (options?.method === 'POST') {
          const body = JSON.parse(options.body);
          return {
            ok: true,
            json: async () => ({ category: body.category }),
          };
        }
        return {
          ok: true,
          json: async () => [],
        };
      }));

      const res = await fetchIncomeCategories();
      const categories = res.map((c) => c.category);
      defaults.forEach((def) => {
        expect(categories).toContain(def);
      });
    });

    it("createJob encrypts name, who, and notes before sending to API and updates jobs store", async () => {
      jobs.set([]);
      const mockJobRes = {
        id: 1,
        name: await enc("Lead Engineer"),
        who: await enc("John"),
        amount_cents: 500000,
        frequency: "monthly",
        start_date: "2026-01-01",
        end_date: null,
        notes: await enc("Lead role"),
        is_active: 1,
        monthly_equivalent_cents: 500000,
      };

      vi.stubGlobal("fetch", vi.fn().mockImplementation(async (url, options) => {
        expect(options.method).toBe("POST");
        const body = JSON.parse(options.body);
        expect(body.name).not.toBe("Lead Engineer"); // encrypted
        expect(body.who).not.toBe("John");
        expect(body.notes).not.toBe("Lead role");
        return {
          ok: true,
          json: async () => mockJobRes,
        };
      }));

      const created = await createJob({
        name: "Lead Engineer",
        who: "John",
        amount_cents: 500000,
        frequency: "monthly",
        start_date: "2026-01-01",
        notes: "Lead role",
        is_active: true,
      });

      expect(created.name).toBe("Lead Engineer");
      expect(created.who).toBe("John");
      expect(created.notes).toBe("Lead role");
      expect(get(jobs)).toContainEqual(created);
    });

    it("authFetch attaches Authorization Bearer header when sessionToken is set", async () => {
      const { authFetch } = await import("../lib/api.js");
      const { sessionToken } = await import("../lib/stores.js");

      sessionToken.set("test-bearer-token-12345");

      let capturedHeaders = null;
      vi.stubGlobal("fetch", vi.fn().mockImplementation(async (url, options) => {
        capturedHeaders = options.headers;
        return {
          ok: true,
          json: async () => ({ status: "ok" }),
        };
      }));

      await authFetch("/users");
      expect(capturedHeaders).toBeDefined();
      expect(capturedHeaders.Authorization).toBe("Bearer test-bearer-token-12345");

      // Reset session token
      sessionToken.set("");
      await authFetch("/users");
      expect(capturedHeaders.Authorization).toBeUndefined();
    });
  });
});

