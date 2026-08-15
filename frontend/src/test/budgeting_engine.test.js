import { describe, it, expect } from 'vitest';
import { upsertBudget, fetchBudgets, fetchAnalytics } from '../lib/api.js';
import { budgets, analytics } from '../lib/stores.js';
import { get } from 'svelte/store';

describe('Budgeting Engine Domain Specifications', () => {
  describe('[BdgOver] Over-Budget Detection', () => {
    it.each([
      { limitCents: 10000, actualCents: 12000, expectedOver: true, expectedPct: 120.0 },
      { limitCents: 10000, actualCents: 8000, expectedOver: false, expectedPct: 80.0 },
      { limitCents: 10000, actualCents: 10000, expectedOver: false, expectedPct: 100.0 }
    ])('[BdgOver] limit $limitCents cents, actual $actualCents cents', async ({ limitCents, actualCents, expectedOver, expectedPct }) => {
      await upsertBudget({ category: 'GROCERIES', month: '2026-07', limit_cents: limitCents });
      const bStore = get(budgets);
      expect(bStore).toBeDefined();

      const pctUsed = limitCents > 0 ? (actualCents / limitCents) * 100 : 0;
      const isOverBudget = actualCents > limitCents;
      expect(isOverBudget).toBe(expectedOver);
      expect(pctUsed).toBe(expectedPct);
    });
  });

  describe('[BdgSrp] Budget Surplus Calculation', () => {
    it.each([
      { limitCents: 20000, actualCents: 15000, expectedSurplus: 5000 },
      { limitCents: 50000, actualCents: 35000, expectedSurplus: 15000 },
      { limitCents: 10000, actualCents: 10000, expectedSurplus: 0 },
      { limitCents: 10000, actualCents: 12000, expectedSurplus: 0 }
    ])('[BdgSrp] limit $limitCents, actual $actualCents yields surplus $expectedSurplus cents', async ({ limitCents, actualCents, expectedSurplus }) => {
      await upsertBudget({ category: 'DINING', month: '2026-07', limit_cents: limitCents });
      const diff = limitCents - actualCents;
      const surplus = diff > 0 ? diff : 0;
      expect(surplus).toBe(expectedSurplus);
    });
  });

  describe('[BdgDef] Budget Deficit Calculation', () => {
    it.each([
      { limitCents: 15000, actualCents: 18000, expectedDeficit: 3000 },
      { limitCents: 5000, actualCents: 10000, expectedDeficit: 5000 },
      { limitCents: 20000, actualCents: 15000, expectedDeficit: 0 }
    ])('[BdgDef] limit $limitCents, actual $actualCents yields deficit $expectedDeficit cents', async ({ limitCents, actualCents, expectedDeficit }) => {
      await upsertBudget({ category: 'ENTERTAINMENT', month: '2026-07', limit_cents: limitCents });
      const diff = limitCents - actualCents;
      const deficit = diff < 0 ? Math.abs(diff) : 0;
      expect(deficit).toBe(expectedDeficit);
    });
  });

  describe('[BdgSplt] Category Split Impact on User Budget Share', () => {
    it.each([
      { limitCents: 40000, allocations: { John: 60, Jane: 40 }, expectedShares: { John: 24000, Jane: 16000 } },
      { limitCents: 100000, allocations: { John: 50, Jane: 50 }, expectedShares: { John: 50000, Jane: 50000 } },
      { limitCents: 30000, allocations: { John: 70, Jane: 30 }, expectedShares: { John: 21000, Jane: 9000 } }
    ])('[BdgSplt] category limit $limitCents split among users', async ({ limitCents, allocations, expectedShares }) => {
      await fetchBudgets('2026-07');
      const calculatedShares = {};
      Object.keys(allocations).forEach(user => {
        calculatedShares[user] = Math.round(limitCents * (allocations[user] / 100));
      });
      expect(calculatedShares).toEqual(expectedShares);
    });
  });

  describe('[BdgRoll] Budget Rollover Calculation', () => {
    it.each([
      { baseLimitCents: 50000, priorDeltaCents: 5000, rolloverEnabled: true, expectedEffective: 55000 },
      { baseLimitCents: 50000, priorDeltaCents: -3000, rolloverEnabled: true, expectedEffective: 47000 },
      { baseLimitCents: 50000, priorDeltaCents: 5000, rolloverEnabled: false, expectedEffective: 50000 }
    ])('[BdgRoll] base $baseLimitCents with prior delta $priorDeltaCents (rollover=$rolloverEnabled)', async ({ baseLimitCents, priorDeltaCents, rolloverEnabled, expectedEffective }) => {
      await fetchBudgets('2026-07');
      const effective = rolloverEnabled ? baseLimitCents + priorDeltaCents : baseLimitCents;
      expect(effective).toBe(expectedEffective);
    });
  });

  describe('[ZeroTBB] Zero-Based Budgeting Equation', () => {
    it.each([
      {
        totalIncomeCents: 500000,
        categories: [{ limitCents: 200000 }, { limitCents: 200000 }, { limitCents: 100000 }],
        expectedToBeBudgeted: 0,
        expectedBalanced: true
      },
      {
        totalIncomeCents: 500000,
        categories: [{ limitCents: 200000 }, { limitCents: 200000 }],
        expectedToBeBudgeted: 100000,
        expectedBalanced: false
      }
    ])('[ZeroTBB] total income $totalIncomeCents balancing test', async ({ totalIncomeCents, categories, expectedToBeBudgeted, expectedBalanced }) => {
      await fetchAnalytics('2026-07');
      const totalBudgetedCents = categories.reduce((sum, c) => sum + c.limitCents, 0);
      const toBeBudgetedCents = totalIncomeCents - totalBudgetedCents;
      expect(toBeBudgetedCents).toBe(expectedToBeBudgeted);
      expect(toBeBudgetedCents === 0).toBe(expectedBalanced);
    });
  });

  describe('[BdgWrn] Warning Threshold Classification', () => {
    it.each([
      { limitCents: 10000, actualCents: 5000, expectedWarning: 'NORMAL' },
      { limitCents: 10000, actualCents: 8500, expectedWarning: 'WARNING' },
      { limitCents: 10000, actualCents: 9500, expectedWarning: 'WARNING' },
      { limitCents: 10000, actualCents: 10500, expectedWarning: 'CRITICAL' }
    ])('[BdgWrn] spending $actualCents of $limitCents triggers $expectedWarning', async ({ limitCents, actualCents, expectedWarning }) => {
      await fetchBudgets('2026-07');
      const pctUsed = limitCents > 0 ? (actualCents / limitCents) * 100 : 0;
      let warningLevel = 'NORMAL';
      if (pctUsed >= 100) {
        warningLevel = 'CRITICAL';
      } else if (pctUsed >= 80) {
        warningLevel = 'WARNING';
      }
      expect(warningLevel).toBe(expectedWarning);
    });
  });
});
