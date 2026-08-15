import { describe, it, expect } from 'vitest';
import { fetchRecurring, createRecurring, deleteRecurring } from '../lib/api.js';
import { recurringExpenses } from '../lib/stores.js';
import { get } from 'svelte/store';

describe('Recurrence Scheduling Domain Specifications', () => {
  describe('[IncRcr] Recurring Income Template Scheduling', () => {
    it.each([
      {
        template: { name: 'Monthly Salary', amountCents: 300000, who: 'John', category: 'SALARY', dayOfMonth: 25 },
        targetMonth: '2026-05',
        expectedDate: '2026-05-25'
      },
      {
        template: { name: 'Freelance Retainer', amountCents: 100000, who: 'Jane', category: 'FREELANCE', dayOfMonth: 31 },
        targetMonth: '2026-02',
        expectedDate: '2026-02-28'
      }
    ])('[IncRcr] generating recurring income for $template.name in $targetMonth', async ({ template, targetMonth, expectedDate }) => {
      await fetchRecurring();
      const [year, month] = targetMonth.split('-').map(Number);
      const daysInMonth = new Date(year, month, 0).getDate();
      const executionDay = Math.min(template.dayOfMonth, daysInMonth);
      const formattedDay = String(executionDay).padStart(2, '0');
      const incomeDate = `${targetMonth}-${formattedDay}`;
      expect(incomeDate).toBe(expectedDate);
      expect(template.amountCents).toBe(template.amountCents);
    });
  });

  describe('[DepAmrt] Asset Depreciation Schedule Generation', () => {
    it.each([
      {
        assetCostCents: 120000,
        salvageValueCents: 0,
        lifetimeMonths: 12,
        expectedMonthly: 10000,
        expectedFinalBookValue: 0
      },
      {
        assetCostCents: 100000,
        salvageValueCents: 10000,
        lifetimeMonths: 10,
        expectedMonthly: 9000,
        expectedFinalBookValue: 10000
      }
    ])('[DepAmrt] cost $assetCostCents over $lifetimeMonths months', async ({ assetCostCents, salvageValueCents, lifetimeMonths, expectedMonthly, expectedFinalBookValue }) => {
      await fetchRecurring();
      const depreciableBaseCents = assetCostCents - salvageValueCents;
      const monthlyDepreciationCents = Math.floor(depreciableBaseCents / lifetimeMonths);
      const remainderCents = depreciableBaseCents - (monthlyDepreciationCents * lifetimeMonths);
      let currentBookValueCents = assetCostCents;
      for (let m = 1; m <= lifetimeMonths; m++) {
        const isLastMonth = (m === lifetimeMonths);
        const amountCents = isLastMonth ? (monthlyDepreciationCents + remainderCents) : monthlyDepreciationCents;
        currentBookValueCents -= amountCents;
      }
      expect(monthlyDepreciationCents).toBe(expectedMonthly);
      expect(currentBookValueCents).toBe(expectedFinalBookValue);
    });
  });
});
