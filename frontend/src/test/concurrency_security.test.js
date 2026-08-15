import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { deriveKey, encryptText, decryptText } from '../lib/crypto.js';
import { createExpense, createJointCorrection } from '../lib/api.js';
import { cryptoKey, expenses, jointCorrections, jointAccount } from '../lib/stores.js';

describe('Concurrency & Security Domain Specifications', () => {
  let masterKey;

  beforeEach(async () => {
    masterKey = await deriveKey('correct-household-passphrase');
    cryptoKey.set(masterKey);
    expenses.set([]);
    jointCorrections.set([]);
    jointAccount.set({ balance_cents: 0 });
  });

  describe('[ShrAuth] Auth Salt Initialization & Passphrase Validation', () => {
    it('verifies master key derivation and static IV AES-GCM decryption', async () => {
      const magicWord = 'FinanceTrackerAuth';
      const encryptedMagic = await encryptText(magicWord, masterKey);

      // Decryption with correct key succeeds
      const decrypted = await decryptText(encryptedMagic, masterKey);
      expect(decrypted).toBe(magicWord);

      // Decryption with wrong key fails
      const wrongKey = await deriveKey('wrong-passphrase');
      const wrongDecrypted = await decryptText(encryptedMagic, wrongKey);
      expect(wrongDecrypted).not.toBe(magicWord);
    });
  });

  describe('[Concurrency] Parallel State Store Mutations', () => {
    it('handles concurrent Promise.all API mutations and updates Svelte stores without race conditions', async () => {
      // Dispatch 5 concurrent expenses in parallel
      const expensePromises = Array.from({ length: 5 }, (_, i) => 
        createExpense({
          name: `Concurrent Expense ${i + 1}`,
          cost_cents: (i + 1) * 1000,
          expense_date: '2026-07-25',
          who_paid: 'John',
          category: 'GROCERIES'
        })
      );

      await Promise.all(expensePromises);

      const currentExpenses = get(expenses);
      expect(currentExpenses.length).toBe(5);

      const totalCostCents = currentExpenses.reduce((sum, e) => sum + e.cost_cents, 0);
      expect(totalCostCents).toBe(15000);
    });

    it('processes concurrent joint account balance corrections atomically', async () => {
      // Post positive top-up and negative withdrawal concurrently
      await Promise.all([
        createJointCorrection({ amount_cents: 10000, correction_date: '2026-07-25', note: 'Top up' }),
        createJointCorrection({ amount_cents: -3000, correction_date: '2026-07-25', note: 'Withdrawal' })
      ]);

      const corrections = get(jointCorrections);
      expect(corrections.length).toBe(2);

      const accountState = get(jointAccount);
      expect(accountState.balance_cents).toBe(7000);
    });
  });
});
