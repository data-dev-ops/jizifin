import { describe, it, expect, beforeEach, vi } from 'vitest';
import { enc, dec, fetchUsers, createUser } from '../lib/api.js';
import { cryptoKey, users } from '../lib/stores.js';
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
    it('encrypts and decrypts values when cryptoKey is set', async () => {
      const plaintext = 'Jim';
      const encrypted = await enc(plaintext);
      expect(encrypted).not.toBe(plaintext);

      const decrypted = await dec(encrypted);
      expect(decrypted).toBe(plaintext);
    });

    it('returns text as-is when cryptoKey is null', async () => {
      cryptoKey.set(null);
      const plaintext = 'Zina';

      const encrypted = await enc(plaintext);
      expect(encrypted).toBe(plaintext);

      const decrypted = await dec(plaintext);
      expect(decrypted).toBe(plaintext);
    });

    it('handles space-separated string decryption (multi-word text)', async () => {
      const w1 = await enc('Jim');
      const w2 = await enc('Zina');
      const combined = `${w1} ${w2}`;

      const decrypted = await dec(combined);
      expect(decrypted).toBe('Jim Zina');
    });

    it('handles empty/falsy values gracefully', async () => {
      expect(await enc('')).toBe('');
      expect(await enc(null)).toBe(null);
      expect(await dec('')).toBe('');
      expect(await dec(null)).toBe(null);
    });
  });

  describe('API requests & User service endpoints', () => {
    it('fetchUsers retrieves users, decrypts names, and updates users store', async () => {
      const encName = await enc('Jim');
      const fakeUsers = [
        { name: encName, color: '#6366f1', is_active: 1, created_at: '2026-01-01' }
      ];

      vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
        ok: true,
        json: async () => fakeUsers,
      }));

      const res = await fetchUsers(true);
      expect(res.length).toBe(1);
      expect(res[0].name).toBe('Jim');
      expect(get(users)).toEqual(res);
    });

    it('createUser sends encrypted payload and updates users store', async () => {
      const mockUserRes = {
        name: await enc('Zina'),
        color: '#ec4899',
        is_active: 1,
        created_at: '2026-01-01'
      };

      vi.stubGlobal('fetch', vi.fn().mockImplementation(async (url, options) => {
        expect(options.method).toBe('POST');
        const body = JSON.parse(options.body);
        expect(body.name).not.toBe('Zina'); // encrypted in transit
        return {
          ok: true,
          json: async () => mockUserRes,
        };
      }));

      const newUser = await createUser({ name: 'Zina', color: '#ec4899' });
      expect(newUser.name).toBe('Zina');
      expect(get(users)).toContainEqual(newUser);
    });

    it('throws error with status and body when API returns non-2xx status', async () => {
      vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
        ok: false,
        status: 400,
        text: async () => 'User already exists',
      }));

      await expect(createUser({ name: 'Duplicate' })).rejects.toThrow('API POST /users → 400: User already exists');
    });
  });
});
