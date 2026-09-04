import { describe, it, expect } from 'vitest';
import {
  parseCsvRows,
  extractIngBeneficiary,
  formatToIsoDate,
  parseEuroAmount,
  parseBankCsv,
  cascadeCategory,
  prepareExpensesForImport,
  prepareTransactionsForImport,
} from '../lib/csvParser.js';
import realIngCsv from './fixtures/ing_statement_sample.csv?raw';

describe('csvParser - Low-level helpers', () => {
  it('parses basic CSV lines and strips BOM', () => {
    const csv = '\uFEFFcol1,col2,"col,3"\na,b,c\n"hello ""world""",d,e';
    const rows = parseCsvRows(csv);
    expect(rows).toHaveLength(3);
    expect(rows[0]).toEqual(['col1', 'col2', 'col,3']);
    expect(rows[1]).toEqual(['a', 'b', 'c']);
    expect(rows[2]).toEqual(['hello "world"', 'd', 'e']);
  });

  it('formats DD/MM/YYYY to YYYY-MM-DD', () => {
    expect(formatToIsoDate('01/08/2026')).toBe('2026-08-01');
    expect(formatToIsoDate('15/12/2025')).toBe('2025-12-15');
  });

  it('parses Euro amounts to cents and detects credit vs debit', () => {
    const debit = parseEuroAmount('"-3,90"');
    expect(debit.isCredit).toBe(false);
    expect(debit.amountCents).toBe(390);
    expect(debit.amount).toBe(-3.9);

    const credit = parseEuroAmount('"2105,68"');
    expect(credit.isCredit).toBe(true);
    expect(credit.amountCents).toBe(210568);
    expect(credit.amount).toBe(2105.68);
  });
});

describe('csvParser - ING Beneficiary Extraction', () => {
  it('extracts merchant from Debit Mastercard with city and postal code', () => {
    const desc = 'Betaling Debit Mastercard 01/08/26 - 16.09 uur - DECATHLON 0252 88000 - EPINAL - FRA Kaartnummer 5229 62XX XXXX 6510 Referentie ING: COP000372131056';
    expect(extractIngBeneficiary(desc)).toBe('DECATHLON');
  });

  it('extracts merchant from Bancontact payment', () => {
    const desc = 'Betaling Bancontact 16/08/26 - 17.08 uur - Wash Time 3080 - Tervuren - BEL Kaartnummer 5229 62XX XXXX 6510';
    expect(extractIngBeneficiary(desc)).toBe('Wash Time');
  });

  it('extracts creditor from SEPA Direct Debit (Domiciliëring)', () => {
    const desc = 'Domiciliëring in euro (SEPA) Amazon EU S.a r.l. Bericht als bijlage';
    expect(extractIngBeneficiary(desc)).toBe('Amazon EU S.a r.l.');

    const desc2 = 'Domiciliëring in euro (SEPA) Luminus SA Bericht als bijlage';
    expect(extractIngBeneficiary(desc2)).toBe('Luminus SA');
  });

  it('extracts counterparty from SEPA credit transfers', () => {
    const descNaar = 'Doorlopende betalingsopdracht in euro (SEPA) Naar: PEETERMANS VERMEULEN - BE18775599639065 Mededeling: Huur appartement';
    expect(extractIngBeneficiary(descNaar)).toBe('PEETERMANS VERMEULEN');

    const descVan = 'Overschrijving in euro (SEPA) Van: GIELEN MARIA - BE46001417949636 Mededeling: gsm abonnement oma';
    expect(extractIngBeneficiary(descVan)).toBe('GIELEN MARIA');
  });

  it('identifies bank fee statements', () => {
    const desc = 'Kostenafrekening nr. 337785411 Bewijsstuk in bijlage';
    expect(extractIngBeneficiary(desc)).toBe('ING Bankkosten');
  });
});

describe('csvParser - Real ING CSV parsing', () => {
  const csvContent = realIngCsv;

  it('parses the real ING statement export accurately', () => {
    const result = parseBankCsv(csvContent, 'ING', 'Jim');
    expect(result.transactions).toHaveLength(44);
    expect(result.stats.total).toBe(44);
    expect(result.stats.expensesCount).toBe(39);
    expect(result.stats.creditsCount).toBe(5);

    // Verify first row
    const first = result.transactions[0];
    expect(first.date).toBe('2026-08-01');
    expect(first.amountCents).toBe(390);
    expect(first.isCredit).toBe(false);
    expect(first.beneficiary).toBe('ING Bankkosten');
    expect(first.include).toBe(true);

    // Verify an incoming credit/salary row has isCredit=true and category='' (requires explicit selection)
    const salary = result.transactions.find((t) => t.beneficiary === 'VLAAMSE VERENIGING VOOR');
    expect(salary).toBeDefined();
    expect(salary.isCredit).toBe(true);
    expect(salary.amountCents).toBe(210568);
    expect(salary.category).toBe('');
    expect(salary.include).toBe(true);
  });

  it('throws friendly error for KBC selection until parser is implemented', () => {
    expect(() => parseBankCsv(csvContent, 'KBC', 'Jim')).toThrowError(/KBC Bank export format is not yet supported/i);
  });
});

describe('csvParser - Category Cascading & Grouping', () => {
  it('cascades category to all transactions with the same beneficiary', () => {
    const transactions = [
      { beneficiary: 'DECATHLON', category: '' },
      { beneficiary: 'AH', category: '' },
      { beneficiary: 'DECATHLON', category: '' },
    ];

    const { updatedTransactions, count } = cascadeCategory(transactions, 'DECATHLON', 'SPORTS');
    expect(count).toBe(2);
    expect(updatedTransactions[0].category).toBe('SPORTS');
    expect(updatedTransactions[1].category).toBe('');
    expect(updatedTransactions[2].category).toBe('SPORTS');
  });

  it('prepares expenses for import and consolidates "group to other" items', () => {
    const transactions = [
      {
        include: true,
        groupToOther: false,
        beneficiary: 'Decathlon Store',
        amountCents: 5000,
        date: '2026-08-10',
        whoPaid: 'Jim',
        category: 'SPORTS',
        tagId: null,
      },
      {
        include: true,
        groupToOther: true,
        beneficiary: 'Small Coffee',
        amountCents: 350,
        date: '2026-08-12',
        whoPaid: 'Jim',
        category: '',
        tagId: null,
      },
      {
        include: true,
        groupToOther: true,
        beneficiary: 'Parking Meter',
        amountCents: 200,
        date: '2026-08-14',
        whoPaid: 'Jim',
        category: '',
        tagId: null,
      },
      {
        include: false, // Skipped item
        groupToOther: false,
        beneficiary: 'Skipped Salary',
        amountCents: 200000,
        date: '2026-08-15',
        whoPaid: 'Jim',
        category: 'INCOME',
        tagId: null,
      },
    ];

    const prepared = prepareExpensesForImport(transactions, 'Jim', 'OTHER');
    // Expect 2 items: 1 individual item and 1 consolidated bundle of the 2 other items
    expect(prepared).toHaveLength(2);

    expect(prepared[0]).toEqual({
      name: 'Decathlon Store',
      cost_cents: 5000,
      expense_date: '2026-08-10',
      who_paid: 'Jim',
      category: 'SPORTS',
      tag_id: null,
      is_joint: false,
      joint_account_id: null,
      overrides: [],
    });

    expect(prepared[1]).toEqual({
      name: 'Other expenses (2 items bundled)',
      cost_cents: 550, // 350 + 200
      expense_date: '2026-08-14',
      who_paid: 'Jim',
      category: 'OTHER',
      tag_id: null,
      is_joint: false,
      joint_account_id: null,
      overrides: [],
    });
  });

  it('separates expenses and incomes with prepareTransactionsForImport', () => {
    const transactions = [
      {
        include: true,
        isCredit: false,
        groupToOther: false,
        beneficiary: 'Supermarket',
        amountCents: 5000,
        date: '2026-08-10',
        whoPaid: 'Jim',
        category: 'GROCERIES',
      },
      {
        include: true,
        isCredit: true,
        groupToOther: false,
        beneficiary: 'Employer Inc',
        amountCents: 250000,
        date: '2026-08-25',
        whoPaid: 'Jim',
        category: 'SALARY',
      },
      {
        include: true,
        isCredit: false,
        groupToOther: true,
        beneficiary: 'Snack',
        amountCents: 250,
        date: '2026-08-11',
        whoPaid: 'Jim',
        category: 'OTHER',
      },
    ];

    const { expenses, incomes } = prepareTransactionsForImport(transactions, 'Jim');
    expect(expenses).toHaveLength(2); // 1 individual + 1 bundled
    expect(incomes).toHaveLength(1);
    expect(incomes[0]).toEqual({
      name: 'Employer Inc',
      amount_cents: 250000,
      income_date: '2026-08-25',
      who: 'Jim',
      category: 'SALARY',
      is_joint: false,
    });
  });

  it('throws an error if an included transaction has no category set (no default to other)', () => {
    const transactions = [
      {
        include: true,
        isCredit: false,
        groupToOther: false,
        beneficiary: 'Bakery',
        amountCents: 450,
        date: '2026-08-10',
        whoPaid: 'Jim',
        category: '', // Missing category!
      },
    ];

    expect(() => prepareTransactionsForImport(transactions, 'Jim')).toThrowError(
      /Category is required for expense "Bakery"/i
    );
  });
});
