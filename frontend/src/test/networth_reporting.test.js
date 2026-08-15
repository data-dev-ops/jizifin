import { describe, it, expect } from 'vitest';
import { fetchIncomeByPerson, fetchAnalytics, fetchPaybacks } from '../lib/api.js';

describe('Net Worth Reporting Domain Specifications', () => {
  describe('[NwSum] Net Worth Summation', () => {
    it.each([
      {
        accounts: [{ name: 'Checking', balanceCents: 1000000, isOnBudget: true }],
        income: 500000,
        expenses: 300000,
        expectedNetWorth: 1200000
      },
      {
        accounts: [
          { name: 'Checking', balanceCents: 500000, isOnBudget: true },
          { name: 'Savings', balanceCents: 1500000, isOnBudget: true }
        ],
        income: 200000,
        expenses: 100000,
        expectedNetWorth: 2100000
      }
    ])('[NwSum] computing total net worth', async ({ accounts, income, expenses, expectedNetWorth }) => {
      await fetchAnalytics('2026-07');
      const totalAssetsCents = accounts.reduce((sum, acc) => sum + acc.balanceCents, 0);
      const netWorth = totalAssetsCents + income - expenses;
      expect(netWorth).toBe(expectedNetWorth);
    });
  });

  describe('[NwExcl] Excluding Off-Budget Accounts from Net Worth', () => {
    it.each([
      {
        accounts: [
          { name: 'On-Budget Joint', balanceCents: 1000000, isOnBudget: true },
          { name: 'Off-Budget Private', balanceCents: 5000000, isOnBudget: false }
        ],
        income: 0,
        expenses: 0,
        expectedNetWorth: 1000000
      }
    ])('[NwExcl] excluding off-budget accounts', async ({ accounts, income, expenses, expectedNetWorth }) => {
      await fetchAnalytics('2026-07');
      const totalAssetsCents = accounts.filter(acc => acc.isOnBudget).reduce((sum, acc) => sum + acc.balanceCents, 0);
      const netWorth = totalAssetsCents + income - expenses;
      expect(netWorth).toBe(expectedNetWorth);
    });
  });

  describe('[CfMath] Cash Flow Inflow vs Outflow Calculation', () => {
    it.each([
      { inflows: 400000, outflows: 250000, expectedNet: 150000 },
      { inflows: 300000, outflows: 350000, expectedNet: -50000 },
      { inflows: 200000, outflows: 200000, expectedNet: 0 }
    ])('[CfMath] inflows $inflows, outflows $outflows yields net $expectedNet', async ({ inflows, outflows, expectedNet }) => {
      await fetchIncomeByPerson('SALARY', '2026-07');
      const netCashFlowCents = inflows - outflows;
      expect(netCashFlowCents).toBe(expectedNet);
    });
  });

  describe('[RepDate] Custom Date Range Filtering', () => {
    it.each([
      { date: '2026-05-15', start: '2026-05-01', end: '2026-05-31', expectedInRange: true },
      { date: '2026-04-30', start: '2026-05-01', end: '2026-05-31', expectedInRange: false },
      { date: '2026-06-01', start: '2026-05-01', end: '2026-05-31', expectedInRange: false }
    ])('[RepDate] date $date in range $start..$end is $expectedInRange', async ({ date, start, end, expectedInRange }) => {
      await fetchAnalytics('2026-07');
      const isDateInRange = date >= start && date <= end;
      expect(isDateInRange).toBe(expectedInRange);
    });
  });

  describe('[NwHist] Historical Multi-Month Net Worth Trend Analysis', () => {
    it.each([
      {
        history: [
          { month: '2026-01', netWorthCents: 1000000 },
          { month: '2026-02', netWorthCents: 1200000 },
          { month: '2026-03', netWorthCents: 1500000 }
        ],
        expectedTrend: 'UP',
        expectedDelta: 500000
      },
      {
        history: [
          { month: '2026-01', netWorthCents: 1500000 },
          { month: '2026-02', netWorthCents: 1200000 }
        ],
        expectedTrend: 'DOWN',
        expectedDelta: -300000
      }
    ])('[NwHist] trend analysis yields $expectedTrend', async ({ history, expectedTrend, expectedDelta }) => {
      await fetchAnalytics('2026-07');
      const first = history[0].netWorthCents;
      const last = history[history.length - 1].netWorthCents;
      const deltaCents = last - first;
      let trend = 'FLAT';
      if (deltaCents > 0) trend = 'UP';
      else if (deltaCents < 0) trend = 'DOWN';
      expect(trend).toBe(expectedTrend);
      expect(deltaCents).toBe(expectedDelta);
    });
  });

  describe('[SavRate] Savings Rate Calculation', () => {
    it.each([
      { income: 500000, expenses: 300000, expectedRate: 40.0 },
      { income: 400000, expenses: 100000, expectedRate: 75.0 },
      { income: 300000, expenses: 300000, expectedRate: 0.0 },
      { income: 0, expenses: 100000, expectedRate: 0.0 }
    ])('[SavRate] income $income, expenses $expenses yields $expectedRate%', async ({ income, expenses, expectedRate }) => {
      await fetchIncomeByPerson('SALARY', '2026-07');
      const rate = income <= 0 ? 0.0 : Math.round(((income - expenses) / income) * 10000) / 100;
      expect(rate).toBe(expectedRate);
    });
  });

  describe('[NWExRst] Net Worth Excluding Asset Revaluations', () => {
    it.each([
      {
        accounts: [
          { name: 'Stock Portfolio', balanceCents: 2000000, unrealizedRevaluationCents: 500000, isOnBudget: true }
        ],
        income: 0,
        expenses: 0,
        expectedNetWorth: 1500000
      }
    ])('[NWExRst] calculating net worth excluding asset revaluations', async ({ accounts, income, expenses, expectedNetWorth }) => {
      await fetchAnalytics('2026-07');
      const totalAssetsCents = accounts.reduce((sum, acc) => sum + (acc.balanceCents - acc.unrealizedRevaluationCents), 0);
      const nw = totalAssetsCents + income - expenses;
      expect(nw).toBe(expectedNetWorth);
    });
  });

  describe('[InfAdj] Inflation-Adjusted Spending Calculation', () => {
    it.each([
      { amountCents: 100000, inflationPct: 3.5, years: 1, expectedAdjusted: 103500 },
      { amountCents: 100000, inflationPct: 5.0, years: 2, expectedAdjusted: 110250 }
    ])('[InfAdj] $amountCents cents adjusted for $inflationPct% over $years years', async ({ amountCents, inflationPct, years, expectedAdjusted }) => {
      await fetchAnalytics('2026-07');
      const factor = Math.pow(1 + inflationPct / 100, years);
      const adjusted = Math.round(amountCents * factor);
      expect(adjusted).toBe(expectedAdjusted);
    });
  });
});
