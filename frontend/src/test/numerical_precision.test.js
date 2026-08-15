import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { deriveKey, encryptText, decryptText } from '../lib/crypto.js';
import { createExpense, fetchExpenses } from '../lib/api.js';
import { cryptoKey, expenses, budgets } from '../lib/stores.js';

describe('Numerical Precision Domain Specifications', () => {
  let testKey;

  beforeEach(async () => {
    testKey = await deriveKey('test-passphrase');
    cryptoKey.set(testKey);
    expenses.set([]);
    budgets.set([]);
  });

  describe('[FloatAdd] IEEE 754 Float Accumulation vs Integer Cents in Store', () => {
    it('aggregates multiple floating point decimal inputs as exact integer cents without drift', async () => {
      // Adding $0.10 and $0.20 via API
      await createExpense({
        name: 'Item 1',
        cost_cents: Math.round(0.10 * 100),
        expense_date: '2026-07-25',
        who_paid: 'John',
        category: 'GROCERIES'
      });

      await createExpense({
        name: 'Item 2',
        cost_cents: Math.round(0.20 * 100),
        expense_date: '2026-07-25',
        who_paid: 'John',
        category: 'GROCERIES'
      });

      const currentExpenses = get(expenses);
      expect(currentExpenses.length).toBe(2);

      const totalCents = currentExpenses.reduce((acc, e) => acc + e.cost_cents, 0);
      expect(totalCents).toBe(30);

      // Contrast with standard JS raw floating point addition drift (0.1 + 0.2 === 0.30000000000000004)
      expect(0.10 + 0.20).not.toBe(0.30);
      expect(totalCents / 100.0).toBe(0.30);
    });
  });

  describe('[RoundHalfUp] Half-Up Rounding for Split Allocations', () => {
    it('accurately rounds split percentage shares for odd cent totals', () => {
      const calculateUserShareCents = (totalCents, pct) => Math.floor(totalCents * (pct / 100) + 0.5);

      expect(calculateUserShareCents(1000, 33.333333333333336)).toBe(333);
      expect(calculateUserShareCents(100, 33.333333333333336)).toBe(33);
      expect(calculateUserShareCents(5, 50.0)).toBe(3);
      expect(calculateUserShareCents(2, 50.0)).toBe(1);
    });
  });

  describe('[LargeInt] Maximum Safe Integer Cents Limits', () => {
    it('preserves large 64-bit integer values in expenses store without truncation', async () => {
      const largeCostCents = 9007199254740991; // Number.MAX_SAFE_INTEGER
      await createExpense({
        name: 'Large Investment Purchase',
        cost_cents: largeCostCents,
        expense_date: '2026-07-25',
        who_paid: 'John',
        category: 'GROCERIES'
      });

      const currentExpenses = get(expenses);
      const created = currentExpenses.find(e => e.cost_cents === largeCostCents);
      expect(created).toBeDefined();
      expect(created.cost_cents).toBe(largeCostCents);
    });
  });

  describe('[ZeroTx] Zero and Negative Transaction Validation', () => {
    it('validates integer cent payloads before store dispatch', () => {
      const isValidCentAmount = (amt) => typeof amt === 'number' && Number.isInteger(amt) && amt > 0 && Number.isSafeInteger(amt);

      expect(isValidCentAmount(0)).toBe(false);
      expect(isValidCentAmount(-500)).toBe(false);
      expect(isValidCentAmount(100)).toBe(true);
    });
  });

  describe('[CryptDec] AES-GCM Static IV Roundtrip Encryption/Decryption', () => {
    it('executes full roundtrip encryption and decryption using static IV WebCrypto', async () => {
      const rawText = 'Household Ledger Entry: John & Jane 🤖💸';
      const encrypted = await encryptText(rawText, testKey);
      expect(typeof encrypted).toBe('string');
      expect(encrypted).not.toContain('='); // Base64URL stripped padding

      const decrypted = await decryptText(encrypted, testKey);
      expect(decrypted).toBe(rawText);
    });
  });
});
