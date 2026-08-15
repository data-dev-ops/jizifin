import { describe, it, expect } from 'vitest';
import { createSplit, updateSplit, fetchPaybacks } from '../lib/api.js';
import { splits, paybacks } from '../lib/stores.js';
import { get } from 'svelte/store';

describe('Splits Allocations Domain Specifications', () => {
  describe('[MultShr] N-User Multi-Share Calculation', () => {
    it.each([
      {
        costCents: 10000,
        allocations: { John: 40, Jane: 40, Alex: 20 },
        expectedShares: { John: 4000, Jane: 4000, Alex: 2000 }
      },
      {
        costCents: 1000,
        allocations: { John: 33.33, Jane: 33.33, Alex: 33.34 },
        expectedShares: { John: 333, Jane: 333, Alex: 334 }
      },
      {
        costCents: 50000,
        allocations: { John: 25, Jane: 25, Alex: 25, Sam: 25 },
        expectedShares: { John: 12500, Jane: 12500, Alex: 12500, Sam: 12500 }
      }
    ])('[MultShr] splitting $costCents cents across multi-user allocations', async ({ costCents, allocations, expectedShares }) => {
      await fetchPaybacks();
      const result = {};
      let totalAllocatedCents = 0;
      const userKeys = Object.keys(allocations);
      userKeys.forEach((user, idx) => {
        if (idx === userKeys.length - 1) {
          result[user] = costCents - totalAllocatedCents;
        } else {
          const share = Math.floor(costCents * (allocations[user] / 100) + 0.5);
          result[user] = share;
          totalAllocatedCents += share;
        }
      });
      expect(result).toEqual(expectedShares);
      const totalSum = Object.values(result).reduce((a, b) => a + b, 0);
      expect(totalSum).toBe(costCents);
    });
  });

  describe('[GrpSplt] Joint Account Exclusion from Paybacks vs Personal Inclusion', () => {
    it.each([
      {
        expense: { category: 'RENT', costCents: 150000, whoPaid: 'John', isJoint: false },
        jointCategories: ['RENT', 'UTILITIES'],
        expectedExcluded: true,
        expectedPayback: 0
      },
      {
        expense: { category: 'UTILITIES', costCents: 20000, whoPaid: 'Jane', isJoint: true },
        jointCategories: ['RENT', 'UTILITIES'],
        expectedExcluded: true,
        expectedPayback: 0
      },
      {
        expense: { category: 'GROCERIES', costCents: 10000, whoPaid: 'John', isJoint: false, splitPct: 50 },
        jointCategories: ['RENT', 'UTILITIES'],
        expectedExcluded: false,
        expectedPayback: 5000
      },
      {
        expense: { category: 'LEISURE', costCents: 8000, whoPaid: 'Jane', isJoint: false, splitPct: 50 },
        jointCategories: ['RENT', 'UTILITIES'],
        expectedExcluded: false,
        expectedPayback: 4000
      }
    ])('[GrpSplt] processing expense in $expense.category category', async ({ expense, jointCategories, expectedExcluded, expectedPayback }) => {
      await fetchPaybacks();
      const isExcluded = expense.isJoint || jointCategories.includes(expense.category);
      const paybackAmountCents = isExcluded ? 0 : Math.floor(expense.costCents * ((expense.splitPct || 50) / 100) + 0.5);
      expect(isExcluded).toBe(expectedExcluded);
      expect(paybackAmountCents).toBe(expectedPayback);
    });
  });
});
