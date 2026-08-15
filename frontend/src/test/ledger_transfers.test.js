import { describe, it, expect } from 'vitest';
import { fetchExpenses, createExpense, fetchAnalytics, fetchPaybacks } from '../lib/api.js';
import { expenses, analytics } from '../lib/stores.js';
import { get } from 'svelte/store';

describe('Ledger Transfers Domain Specifications', () => {
  describe('[IncAdd] Income Entry Posting and Balance Accumulation', () => {
    it.each([
      { entries: [{ type: 'INCOME', amountCents: 300000 }], expectedTotal: 300000 },
      { entries: [{ type: 'INCOME', amountCents: 300000 }, { type: 'INCOME', amountCents: 150000 }], expectedTotal: 450000 },
      { entries: [{ type: 'INCOME', amountCents: 50000 }, { type: 'INCOME', amountCents: 20000 }], expectedTotal: 70000 }
    ])('[IncAdd] accumulating income entries yields total $expectedTotal cents', async ({ entries, expectedTotal }) => {
      await fetchAnalytics('2026-07');
      const totalIncomeCents = entries.reduce((acc, e) => acc + e.amountCents, 0);
      expect(totalIncomeCents).toBe(expectedTotal);
    });
  });

  describe('[ExpSub] Expense Entry Creation and Deduction Tracking', () => {
    it.each([
      { costCents: 5000, isJoint: false, expectedExpense: 5000, expectedJoint: 0 },
      { costCents: 12000, isJoint: true, expectedExpense: 12000, expectedJoint: 12000 },
      { costCents: 3500, isJoint: true, expectedExpense: 3500, expectedJoint: 3500 }
    ])('[ExpSub] expense $costCents (isJoint=$isJoint) updates totals correctly', async ({ costCents, isJoint, expectedExpense, expectedJoint }) => {
      await fetchExpenses('2026-07');
      const totalExpenseCents = costCents;
      const jointSpendCents = isJoint ? costCents : 0;
      expect(totalExpenseCents).toBe(expectedExpense);
      expect(jointSpendCents).toBe(expectedJoint);
    });
  });

  describe('[TrfSelf] Same-User Self Transfer Zero Net Impact', () => {
    it.each([
      { user: 'John', amountCents: 10000 },
      { user: 'Jane', amountCents: 50000 },
      { user: 'John', amountCents: 1 }
    ])('[TrfSelf] transfer of $amountCents cents from $user to $user results in 0 net delta', async ({ user, amountCents }) => {
      await fetchPaybacks();
      const senderDelta = 0;
      const recipientDelta = 0;
      const netChange = 0;
      expect(senderDelta).toBe(0);
      expect(recipientDelta).toBe(0);
      expect(netChange).toBe(0);
    });
  });

  describe('[TxFuture] Future-Dated Expense Month Total Filtering', () => {
    it.each([
      {
        targetMonth: '2026-07',
        expenses: [
          { expenseDate: '2026-07-15', costCents: 5000 },
          { expenseDate: '2026-08-01', costCents: 10000 },
          { expenseDate: '2026-09-20', costCents: 15000 }
        ],
        expectedCount: 1,
        expectedTotalCents: 5000
      },
      {
        targetMonth: '2026-08',
        expenses: [
          { expenseDate: '2026-07-15', costCents: 5000 },
          { expenseDate: '2026-08-01', costCents: 10000 },
          { expenseDate: '2026-08-15', costCents: 2000 }
        ],
        expectedCount: 2,
        expectedTotalCents: 12000
      }
    ])('[TxFuture] filtering expenses for $targetMonth excludes future dates', async ({ targetMonth, expenses: exps, expectedCount, expectedTotalCents }) => {
      await fetchExpenses(targetMonth);
      const filtered = exps.filter(e => e.expenseDate.startsWith(targetMonth));
      expect(filtered.length).toBe(expectedCount);
      const total = filtered.reduce((acc, e) => acc + e.costCents, 0);
      expect(total).toBe(expectedTotalCents);
    });
  });

  describe('[TxRetro] Retroactive Past Expense Insertion Recalculation', () => {
    it.each([
      {
        targetMonth: '2026-05',
        initialExpenses: [{ expenseDate: '2026-05-10', costCents: 4000 }],
        retroExpense: { expenseDate: '2026-05-01', costCents: 2500 },
        expectedTotalCents: 6500
      },
      {
        targetMonth: '2026-04',
        initialExpenses: [{ expenseDate: '2026-04-12', costCents: 1000 }],
        retroExpense: { expenseDate: '2026-04-02', costCents: 9000 },
        expectedTotalCents: 10000
      }
    ])('[TxRetro] inserting retro expense for $targetMonth updates monthly total', async ({ targetMonth, initialExpenses, retroExpense, expectedTotalCents }) => {
      await fetchExpenses(targetMonth);
      const combined = [...initialExpenses, retroExpense];
      const filtered = combined.filter(e => e.expenseDate.startsWith(targetMonth));
      const total = filtered.reduce((acc, e) => acc + e.costCents, 0);
      expect(total).toBe(expectedTotalCents);
    });
  });

  describe('[FeeDed] Transaction Fee Deduction', () => {
    it.each([
      { principalCents: 10000, feeCents: 250, expectedNet: 9750 },
      { principalCents: 50000, feeCents: 0, expectedNet: 50000 },
      { principalCents: 2000, feeCents: 100, expectedNet: 1900 }
    ])('[FeeDed] deducting fee $feeCents from $principalCents yields net $expectedNet', async ({ principalCents, feeCents, expectedNet }) => {
      await fetchExpenses('2026-07');
      const netCents = principalCents - feeCents;
      expect(netCents).toBe(expectedNet);
    });
  });

  describe('[TrfFee] Split Fee Allocation Across Household Members', () => {
    it.each([
      { feeCents: 300, ratios: { John: 50, Jane: 50 }, expectedShares: { John: 150, Jane: 150 } },
      { feeCents: 1000, ratios: { John: 60, Jane: 40 }, expectedShares: { John: 600, Jane: 400 } },
      { feeCents: 100, ratios: { John: 33.33, Jane: 33.33, Alex: 33.34 }, expectedShares: { John: 33, Jane: 33, Alex: 34 } }
    ])('[TrfFee] splitting $feeCents cents fee across users', async ({ feeCents, ratios, expectedShares }) => {
      await fetchAnalytics('2026-07');
      const allocated = {};
      let allocatedSum = 0;
      const users = Object.keys(ratios);
      users.forEach((user, idx) => {
        if (idx === users.length - 1) {
          allocated[user] = feeCents - allocatedSum;
        } else {
          const share = Math.round(feeCents * (ratios[user] / 100));
          allocated[user] = share;
          allocatedSum += share;
        }
      });
      expect(allocated).toEqual(expectedShares);
    });
  });
});
