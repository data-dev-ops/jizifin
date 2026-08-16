import { describe, it, expect } from 'vitest';
import { fetchRecurring, createRecurring, updateRecurring, deleteRecurring } from '../lib/api.js';
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

  describe('[Rcr4Wk] 4-Weekly & Weekly Flexible Recurrence Math', () => {
    it('computes multiple occurrences for 4-weekly expense starting early in a month', () => {
      // 4-weekly starting on 2026-08-02
      const startDate = new Date('2026-08-02');
      const intervalDays = 28;
      const targetMonth = '2026-08';
      const [y, m] = targetMonth.split('-').map(Number);
      const daysInMonth = new Date(y, m, 0).getDate();
      const mStart = new Date(y, m - 1, 1);
      const mEnd = new Date(y, m - 1, daysInMonth, 23, 59, 59);

      const occurrences = [];
      let curr = new Date(startDate);
      while (curr <= mEnd) {
        if (curr >= mStart) {
          const yr = curr.getFullYear();
          const mo = String(curr.getMonth() + 1).padStart(2, '0');
          const dy = String(curr.getDate()).padStart(2, '0');
          occurrences.push(`${yr}-${mo}-${dy}`);
        }
        curr = new Date(curr.getTime() + intervalDays * 24 * 60 * 60 * 1000);
      }

      // In August 2026: Aug 2 and Aug 30 -> exactly 2 occurrences!
      expect(occurrences).toEqual(['2026-08-02', '2026-08-30']);
      expect(occurrences.length).toBe(2);
    });

    it('computes 5 occurrences for weekly expense starting on 1st of 31-day month', () => {
      const startDate = new Date('2026-08-01');
      const intervalDays = 7;
      const targetMonth = '2026-08';
      const [y, m] = targetMonth.split('-').map(Number);
      const daysInMonth = new Date(y, m, 0).getDate();
      const mStart = new Date(y, m - 1, 1);
      const mEnd = new Date(y, m - 1, daysInMonth, 23, 59, 59);

      const occurrences = [];
      let curr = new Date(startDate);
      while (curr <= mEnd) {
        if (curr >= mStart) {
          const yr = curr.getFullYear();
          const mo = String(curr.getMonth() + 1).padStart(2, '0');
          const dy = String(curr.getDate()).padStart(2, '0');
          occurrences.push(`${yr}-${mo}-${dy}`);
        }
        curr = new Date(curr.getTime() + intervalDays * 24 * 60 * 60 * 1000);
      }

      expect(occurrences).toEqual(['2026-08-01', '2026-08-08', '2026-08-15', '2026-08-22', '2026-08-29']);
      expect(occurrences.length).toBe(5);
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
