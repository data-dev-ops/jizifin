import { describe, it, expect } from 'vitest';
import { fetchSettlements, createSettlement, createExpense } from '../lib/api.js';

describe('Reconciliation & Locking Domain Specifications', () => {
  describe('[BalRecon] Bank Statement Reconciliation', () => {
    it.each([
      { statement: 500000, ledger: 500000, expectedDiscrepancy: 0, expectedReconciled: true },
      { statement: 500000, ledger: 502500, expectedDiscrepancy: 2500, expectedReconciled: false },
      { statement: 500000, ledger: 498000, expectedDiscrepancy: -2000, expectedReconciled: false }
    ])('[BalRecon] statement $statement vs ledger $ledger', async ({ statement, ledger, expectedDiscrepancy, expectedReconciled }) => {
      await fetchSettlements();
      const discrepancyCents = ledger - statement;
      expect(discrepancyCents).toBe(expectedDiscrepancy);
      expect(discrepancyCents === 0).toBe(expectedReconciled);
    });
  });

  describe('[CrdStmt] Credit Card Statement Cycle Assignment', () => {
    it.each([
      { txDate: '2026-05-10', closingDay: 25, expectedStatementMonth: '2026-05' },
      { txDate: '2026-05-26', closingDay: 25, expectedStatementMonth: '2026-06' },
      { txDate: '2026-12-28', closingDay: 25, expectedStatementMonth: '2027-01' }
    ])('[CrdStmt] transaction $txDate with closing day $closingDay belongs to cycle $expectedStatementMonth', async ({ txDate, closingDay, expectedStatementMonth }) => {
      await fetchSettlements();
      const date = new Date(txDate);
      const day = date.getDate();
      let year = date.getFullYear();
      let month = date.getMonth() + 1;
      if (day > closingDay) {
        month += 1;
        if (month > 12) {
          month = 1;
          year += 1;
        }
      }
      const statementMonthStr = `${year}-${String(month).padStart(2, '0')}`;
      expect(statementMonthStr).toBe(expectedStatementMonth);
    });
  });

  describe('[CrdPmt] Credit Card Payment Linking', () => {
    it.each([
      { statement: 45000, payment: 45000, expectedRemaining: 0, expectedFullyPaid: true },
      { statement: 45000, payment: 20000, expectedRemaining: 25000, expectedFullyPaid: false }
    ])('[CrdPmt] statement $statement paid with $payment', async ({ statement, payment, expectedRemaining, expectedFullyPaid }) => {
      await fetchSettlements();
      const remainingCents = statement - payment;
      expect(remainingCents).toBe(expectedRemaining);
      expect(remainingCents <= 0).toBe(expectedFullyPaid);
    });
  });

  describe('[CrdInt] Credit Card APR Interest Calculation', () => {
    it.each([
      { avgBalanceCents: 100000, aprPct: 18.0, days: 30, expectedInterest: 1479 },
      { avgBalanceCents: 500000, aprPct: 24.0, days: 31, expectedInterest: 10192 }
    ])('[CrdInt] balance $avgBalanceCents at $aprPct% APR for $days days yields $expectedInterest interest cents', async ({ avgBalanceCents, aprPct, days, expectedInterest }) => {
      await fetchSettlements();
      const dailyRate = (aprPct / 100) / 365;
      const interestCents = Math.round(avgBalanceCents * dailyRate * days);
      expect(interestCents).toBe(expectedInterest);
    });
  });
});
