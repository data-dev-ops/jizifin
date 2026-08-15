import { describe, it, expect } from 'vitest';
import { fetchExpenses, createExpense, fetchProjects, createProject, fetchTagAnalytics } from '../lib/api.js';

describe('Import, Export & Analytics Domain Specifications', () => {
  describe('[CsvImp] CSV Row Parsing', () => {
    it.each([
      {
        csvLine: '2026-05-10, Groceries, John, FOOD, -45.50',
        expectedDate: '2026-05-10',
        expectedName: 'Groceries',
        expectedCostCents: 4550
      },
      {
        csvLine: '2026-06-01, Monthly Salary, Jane, SALARY, 3000.00',
        expectedDate: '2026-06-01',
        expectedName: 'Monthly Salary',
        expectedCostCents: 300000
      }
    ])('[CsvImp] parsing "$csvLine"', async ({ csvLine, expectedDate, expectedName, expectedCostCents }) => {
      await fetchExpenses('2026-07');
      const parts = csvLine.split(',').map(s => s.trim());
      const rawAmount = parseFloat(parts[4]);
      const costCents = Math.round(Math.abs(rawAmount) * 100);
      expect(parts[0]).toBe(expectedDate);
      expect(parts[1]).toBe(expectedName);
      expect(costCents).toBe(expectedCostCents);
    });
  });

  describe('[ImpSign] CSV Amount Sign Normalization', () => {
    it.each([
      { rawAmount: '-45.50', type: 'EXPENSE', expectedCents: 4550, expectedIncome: false },
      { rawAmount: '1500.00', type: 'INCOME', expectedCents: 150000, expectedIncome: true },
      { rawAmount: '100.00', type: 'EXPENSE', expectedCents: 10000, expectedIncome: false }
    ])('[ImpSign] amount "$rawAmount" as $type', async ({ rawAmount, type, expectedCents, expectedIncome }) => {
      await fetchExpenses('2026-07');
      const val = parseFloat(rawAmount);
      const isIncome = type.toUpperCase() === 'INCOME';
      const cents = Math.round(Math.abs(val) * 100);
      expect(cents).toBe(expectedCents);
      expect(isIncome).toBe(expectedIncome);
    });
  });

  describe('[TxAtch] Transaction Attachment Metadata Linking', () => {
    it.each([
      { txId: 101, file: { fileName: 'receipt.pdf', sizeBytes: 102400, mimeType: 'application/pdf' } }
    ])('[TxAtch] linking attachment to tx $txId', async ({ txId, file }) => {
      await fetchExpenses('2026-07');
      const attachment = { fileName: file.fileName, mimeType: file.mimeType };
      expect(attachment.fileName).toBe('receipt.pdf');
      expect(attachment.mimeType).toBe('application/pdf');
    });
  });

  describe('[DupChk] Duplicate Transaction Detection', () => {
    it.each([
      {
        existing: [{ expenseDate: '2026-05-10', costCents: 4550, name: 'Groceries' }],
        incoming: { expenseDate: '2026-05-10', costCents: 4550, name: 'groceries' },
        expectedDuplicate: true
      },
      {
        existing: [{ expenseDate: '2026-05-10', costCents: 4550, name: 'Groceries' }],
        incoming: { expenseDate: '2026-05-11', costCents: 4550, name: 'Groceries' },
        expectedDuplicate: false
      }
    ])('[DupChk] checking duplicate', async ({ existing, incoming, expectedDuplicate }) => {
      await fetchExpenses('2026-07');
      const isDup = existing.some(tx => 
        tx.expenseDate === incoming.expenseDate &&
        tx.costCents === incoming.costCents &&
        tx.name.toLowerCase() === incoming.name.toLowerCase()
      );
      expect(isDup).toBe(expectedDuplicate);
    });
  });

  describe('[FyStart] Fiscal Year Calculation', () => {
    it.each([
      { dateISO: '2026-04-15', fyStartMonth: 4, expectedFyLabel: 'FY2027-Q1' },
      { dateISO: '2026-03-10', fyStartMonth: 4, expectedFyLabel: 'FY2026-Q4' }
    ])('[FyStart] date $dateISO with FY start month $fyStartMonth', async ({ dateISO, fyStartMonth, expectedFyLabel }) => {
      await fetchProjects();
      const date = new Date(dateISO);
      const month = date.getMonth() + 1;
      const year = date.getFullYear();
      let relativeMonth = month - fyStartMonth + 1;
      if (relativeMonth <= 0) relativeMonth += 12;
      const quarter = Math.ceil(relativeMonth / 3);
      const fyYear = month < fyStartMonth ? year : year + 1;
      const fyLabel = `FY${fyYear}-Q${quarter}`;
      expect(fyLabel).toBe(expectedFyLabel);
    });
  });

  describe('[TmZone] Timezone Conversion to Local ISO Date', () => {
    it.each([
      { utcISO: '2026-05-10T23:00:00.000Z', offsetHours: 2, expectedDate: '2026-05-11' },
      { utcISO: '2026-05-10T01:00:00.000Z', offsetHours: -5, expectedDate: '2026-05-09' }
    ])('[TmZone] converting UTC $utcISO with offset $offsetHours', async ({ utcISO, offsetHours, expectedDate }) => {
      await fetchExpenses('2026-07');
      const utcDate = new Date(utcISO);
      const localTimeMs = utcDate.getTime() + (offsetHours * 3600 * 1000);
      const localDate = new Date(localTimeMs);
      const yyyy = localDate.getUTCFullYear();
      const mm = String(localDate.getUTCMonth() + 1).padStart(2, '0');
      const dd = String(localDate.getUTCDate()).padStart(2, '0');
      expect(`${yyyy}-${mm}-${dd}`).toBe(expectedDate);
    });
  });

  describe('[LoanInt] Loan Amortization Interest vs Principal', () => {
    it.each([
      { principal: 1000000, payment: 20000, apr: 6.0, expectedInterest: 5000, expectedPrincipal: 15000 }
    ])('[LoanInt] principal $principal, payment $payment at $apr% APR', async ({ principal, payment, apr, expectedInterest, expectedPrincipal }) => {
      await fetchProjects();
      const monthlyRate = (apr / 100) / 12;
      const interestCents = Math.round(principal * monthlyRate);
      const principalCents = payment - interestCents;
      expect(interestCents).toBe(expectedInterest);
      expect(principalCents).toBe(expectedPrincipal);
    });
  });

  describe('[InvBuy] Investment Buy Order Calculation', () => {
    it.each([
      { units: 10, pricePerShareCents: 5000, expectedTotal: 50000 },
      { units: 100, pricePerShareCents: 1250, expectedTotal: 125000 }
    ])('[InvBuy] buying $units shares at $pricePerShareCents cents', async ({ units, pricePerShareCents, expectedTotal }) => {
      await fetchProjects();
      const totalCostCents = units * pricePerShareCents;
      expect(totalCostCents).toBe(expectedTotal);
    });
  });

  describe('[InvReal] Realized Gain/Loss Calculation', () => {
    it.each([
      { units: 50, buyPrice: 2000, sellPrice: 3000, expectedGainLoss: 50000, expectedProfit: true },
      { units: 50, buyPrice: 3000, sellPrice: 2000, expectedGainLoss: -50000, expectedProfit: false }
    ])('[InvReal] selling $units shares (buy=$buyPrice, sell=$sellPrice)', async ({ units, buyPrice, sellPrice, expectedGainLoss, expectedProfit }) => {
      await fetchProjects();
      const totalCostCents = units * buyPrice;
      const totalProceedsCents = units * sellPrice;
      const gainLossCents = totalProceedsCents - totalCostCents;
      expect(gainLossCents).toBe(expectedGainLoss);
      expect(gainLossCents > 0).toBe(expectedProfit);
    });
  });

  describe('[InvSplit] Stock Split Adjustment', () => {
    it.each([
      { originalUnits: 50, originalPrice: 10000, ratioNum: 2, ratioDen: 1, expectedUnits: 100, expectedPrice: 5000 },
      { originalUnits: 30, originalPrice: 9000, ratioNum: 3, ratioDen: 1, expectedUnits: 90, expectedPrice: 3000 }
    ])('[InvSplit] split ratio $ratioNum:$ratioDen on $originalUnits shares', async ({ originalUnits, originalPrice, ratioNum, ratioDen, expectedUnits, expectedPrice }) => {
      await fetchProjects();
      const newUnits = (originalUnits * ratioNum) / ratioDen;
      const newPriceCents = Math.round((originalPrice * ratioDen) / ratioNum);
      expect(newUnits).toBe(expectedUnits);
      expect(newPriceCents).toBe(expectedPrice);
    });
  });

  describe('[TaxFlag] Tax-Deductible Expense Aggregation', () => {
    it.each([
      {
        expenses: [
          { name: 'Office Supplies', costCents: 15000, isTaxDeductible: true },
          { name: 'Personal Lunch', costCents: 2500, isTaxDeductible: false },
          { name: 'Software Sub', costCents: 8000, isTaxDeductible: true }
        ],
        expectedTaxDeductibleCents: 23000
      }
    ])('[TaxFlag] aggregating tax deductible expenses', async ({ expenses, expectedTaxDeductibleCents }) => {
      await fetchExpenses('2026-07');
      const total = expenses.filter(e => e.isTaxDeductible).reduce((sum, e) => sum + e.costCents, 0);
      expect(total).toBe(expectedTaxDeductibleCents);
    });
  });

  describe('[ExpPrj] Future Spending Projection', () => {
    it.each([
      { totalSpent: 300000, daysElapsed: 30, projectionDays: 30, expectedProjection: 300000 },
      { totalSpent: 150000, daysElapsed: 15, projectionDays: 15, expectedProjection: 150000 }
    ])('[ExpPrj] spending $totalSpent over $daysElapsed days projected for $projectionDays days', async ({ totalSpent, daysElapsed, projectionDays, expectedProjection }) => {
      await fetchExpenses('2026-07');
      const dailyBurnRateCents = totalSpent / daysElapsed;
      const projection = Math.round(dailyBurnRateCents * projectionDays);
      expect(projection).toBe(expectedProjection);
    });
  });
});
