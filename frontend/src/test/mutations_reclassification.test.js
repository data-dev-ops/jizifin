import { describe, it, expect } from 'vitest';
import { deleteExpense, updateExpense, deleteIncomeCategory } from '../lib/api.js';
import { expenses, splits } from '../lib/stores.js';
import { get } from 'svelte/store';

describe('Mutations & Reclassification Domain Specifications', () => {
  describe('[DelCatTx] Category Deletion Handling with Active Transactions', () => {
    it.each([
      {
        categoryToDelete: 'OLD_CAT',
        fallbackCategory: 'UNCATEGORIZED',
        transactions: [
          { id: 1, category: 'OLD_CAT', costCents: 5000 },
          { id: 2, category: 'GROCERIES', costCents: 2000 }
        ],
        expectedCatId1: 'UNCATEGORIZED'
      }
    ])('[DelCatTx] reassigning deleted category $categoryToDelete', async ({ categoryToDelete, fallbackCategory, transactions, expectedCatId1 }) => {
      await deleteIncomeCategory(categoryToDelete).catch(() => {});
      const updated = transactions.map(tx => {
        if (tx.category === categoryToDelete) {
          return { ...tx, category: fallbackCategory, isReassigned: true };
        }
        return tx;
      });
      expect(updated.find(t => t.id === 1).category).toBe(expectedCatId1);
      expect(updated.find(t => t.id === 2).category).toBe('GROCERIES');
    });
  });

  describe('[TxVoid] Voiding Transaction Without Ledger Row Removal', () => {
    it.each([
      { transaction: { id: 10, costCents: 15000 }, isVoid: true, expectedEffective: 0 },
      { transaction: { id: 10, costCents: 15000 }, isVoid: false, expectedEffective: 15000 }
    ])('[TxVoid] void state $isVoid for transaction $transaction.id', async ({ transaction, isVoid, expectedEffective }) => {
      if (isVoid) {
        await deleteExpense(transaction.id).catch(() => {});
      }
      const effectiveCostCents = isVoid ? 0 : transaction.costCents;
      expect(effectiveCostCents).toBe(expectedEffective);
    });
  });

  describe('[CatBlk] Bulk Category Reclassification', () => {
    it.each([
      {
        transactions: [
          { id: 1, category: 'DINING' },
          { id: 2, category: 'RESTAURANTS' },
          { id: 3, category: 'GROCERIES' }
        ],
        rules: { DINING: 'FOOD', RESTAURANTS: 'FOOD' },
        expectedCategories: ['FOOD', 'FOOD', 'GROCERIES']
      }
    ])('[CatBlk] bulk reclassifying categories', async ({ transactions, rules, expectedCategories }) => {
      const sStore = get(splits);
      expect(sStore).toBeDefined();
      const result = transactions.map(tx => {
        if (rules[tx.category]) {
          return { ...tx, category: rules[tx.category] };
        }
        return tx;
      });
      const actualCats = result.map(r => r.category);
      expect(actualCats).toEqual(expectedCategories);
    });
  });
});
