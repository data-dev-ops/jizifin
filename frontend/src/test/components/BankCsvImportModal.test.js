import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import BankCsvImportModal from '../../lib/BankCsvImportModal.svelte';
import { users, splits, tags, currencySymbol, selectedMonth, incomeCategories, expenses } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('BankCsvImportModal.svelte — Bank Statement CSV Importer', () => {
  beforeEach(() => {
    selectedMonth.set('2026-08');
    currencySymbol.set('€');
    expenses.set([]);
    users.set([
      { name: 'Jim', is_active: true, color: '#10b981' },
      { name: 'Partner', is_active: true, color: '#6366f1' },
    ]);
    splits.set([
      { category: 'GROCERIES' },
      { category: 'UTILITIES' },
      { category: 'SPORTS' },
      { category: 'OTHER' },
    ]);
    incomeCategories.set([
      { category: 'SALARY' },
      { category: 'BONUS' },
      { category: 'REIMBURSEMENT' },
    ]);
    tags.set([
      { id: 1, name: 'Holiday', is_active: true },
      { id: 2, name: 'Renovation', is_active: true },
    ]);

    vi.restoreAllMocks();
  });

  it('renders bank selector, payer selection, and upload dropzone when opened', () => {
    render(BankCsvImportModal, { props: { isOpen: true } });

    expect(screen.getByText('Bank Statement CSV Importer')).toBeInTheDocument();
    expect(screen.getByLabelText(/Select Origin Bank/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Who Paid/i)).toBeInTheDocument();
    expect(screen.getByText(/Click to browse or drag and drop your bank CSV file/i)).toBeInTheDocument();

    const bankSelect = screen.getByLabelText(/Select Origin Bank/i);
    expect(bankSelect.value).toBe('ING');
  });

  it('shows warning when KBC Bank is selected', async () => {
    render(BankCsvImportModal, { props: { isOpen: true } });

    const bankSelect = screen.getByLabelText(/Select Origin Bank/i);
    await fireEvent.change(bankSelect, { target: { value: 'KBC' } });

    expect(screen.getByText(/KBC parser is scheduled for a future update/i)).toBeInTheDocument();
  });

  it('parses CSV input and renders transactions in review table', async () => {
    const csvContent = `Account Number,Account Name,Counterparty account,Entry number,Booking date,Value date,Amount,Currency,Description,Entry Details,Message
BE76377126940095,DE H JIM VEKEMANS,,500,01/08/2026,30/07/2026,"-3,90",EUR,Kostenafrekening nr. 337785411 Bewijsstuk in bijlage,,
BE76377126940095,DE H JIM VEKEMANS,,506,03/08/2026,01/08/2026,"-32,97",EUR,Betaling Debit Mastercard 01/08/26 - 16.09 uur - DECATHLON 0252 88000 - EPINAL - FRA Kaartnummer 5229 62XX XXXX 6510 Referentie ING: COP000372131056,,
BE76377126940095,DE H JIM VEKEMANS,,507,04/08/2026,02/08/2026,"-15,00",EUR,Betaling Debit Mastercard 02/08/26 - 10.00 uur - DECATHLON 0252 88000 - EPINAL - FRA Kaartnummer 5229 62XX XXXX 6510 Referentie ING: COP000372131057,,
`;

    const { container } = render(BankCsvImportModal, { props: { isOpen: true } });

    const file = new File([csvContent], 'statement.csv', { type: 'text/csv' });
    const input = container.querySelector('input[type="file"]');
    await fireEvent.change(input, { target: { files: [file] } });

    await new Promise((r) => setTimeout(r, 50));

    expect(screen.getByText('Total Rows')).toBeInTheDocument();
    expect(screen.getByDisplayValue('ING Bankkosten')).toBeInTheDocument();
    expect(screen.getAllByDisplayValue('DECATHLON').length).toBe(2);
  });

  it('cascades category selection to all transactions with the same beneficiary', async () => {
    const csvContent = `Account Number,Account Name,Counterparty account,Entry number,Booking date,Value date,Amount,Currency,Description,Entry Details,Message
BE76377126940095,DE H JIM VEKEMANS,,506,03/08/2026,01/08/2026,"-32,97",EUR,Betaling Debit Mastercard 01/08/26 - 16.09 uur - DECATHLON 0252 88000 - EPINAL - FRA Kaartnummer 5229 62XX XXXX 6510 Referentie ING: COP000372131056,,
BE76377126940095,DE H JIM VEKEMANS,,507,04/08/2026,02/08/2026,"-15,00",EUR,Betaling Debit Mastercard 02/08/26 - 10.00 uur - DECATHLON 0252 88000 - EPINAL - FRA Kaartnummer 5229 62XX XXXX 6510 Referentie ING: COP000372131057,,
`;

    const { container } = render(BankCsvImportModal, { props: { isOpen: true } });

    const file = new File([csvContent], 'statement.csv', { type: 'text/csv' });
    const input = container.querySelector('input[type="file"]');
    await fireEvent.change(input, { target: { files: [file] } });

    await new Promise((r) => setTimeout(r, 50));

    // Category select is in column 6
    const categorySelects = container.querySelectorAll('tbody tr td:nth-child(6) select');
    await fireEvent.change(categorySelects[0], { target: { value: 'SPORTS' } });

    await new Promise((r) => setTimeout(r, 20));

    const updatedSelects = container.querySelectorAll('tbody tr td:nth-child(6) select');
    expect(updatedSelects[0].value).toBe('SPORTS');
    expect(updatedSelects[1].value).toBe('SPORTS');

    expect(screen.getByText(/Updated 2 expenses for "DECATHLON" to "SPORTS"/i)).toBeInTheDocument();
  });

  it('separates and imports both expenses and income entries with respective categories', async () => {
    const batchExpenseSpy = vi.spyOn(api, 'createExpensesBatch').mockResolvedValue([]);
    const createIncomeSpy = vi.spyOn(api, 'createIncome').mockResolvedValue([]);

    const csvContent = `Account Number,Account Name,Counterparty account,Entry number,Booking date,Value date,Amount,Currency,Description,Entry Details,Message
BE76377126940095,DE H JIM VEKEMANS,,506,03/08/2026,01/08/2026,"-32,97",EUR,Betaling Debit Mastercard 01/08/26 - 16.09 uur - DECATHLON 0252 88000 - EPINAL - FRA Kaartnummer 5229 62XX XXXX 6510 Referentie ING: COP000372131056,,
BE76377126940095,DE H JIM VEKEMANS,BE82363137073568,532,25/08/2026,25/08/2026,"2105,68",EUR,Overschrijving in euro (SEPA) Van: VLAAMSE VERENIGING VOOR - BE82363137073568 Mededeling: Salary,,
`;

    const { container } = render(BankCsvImportModal, { props: { isOpen: true } });

    const file = new File([csvContent], 'statement.csv', { type: 'text/csv' });
    const input = container.querySelector('input[type="file"]');
    await fireEvent.change(input, { target: { files: [file] } });

    await new Promise((r) => setTimeout(r, 50));

    // Verify badges
    expect(screen.getByText('Expense')).toBeInTheDocument();
    expect(screen.getByText('Income')).toBeInTheDocument();

    // Verify that the second row (income) has income categories available
    const categorySelects = container.querySelectorAll('tbody tr td:nth-child(6) select');
    const incomeOptions = Array.from(categorySelects[1].options).map((o) => o.value);
    expect(incomeOptions).toContain('SALARY');
    expect(incomeOptions).toContain('BONUS');
    expect(incomeOptions).not.toContain('GROCERIES');

    // Verify that the first row (expense) has expense categories
    const expenseOptions = Array.from(categorySelects[0].options).map((o) => o.value);
    expect(expenseOptions).toContain('GROCERIES');
    expect(expenseOptions).toContain('SPORTS');

    // Initially, both are missing a category: Import button must be disabled with warning text
    expect(screen.getByText(/Assign Categories to Import \(2 missing\)/i)).toBeInTheDocument();
    expect(screen.getByText(/2 included items/i)).toBeInTheDocument();
    expect(screen.getByText(/Missing Category/i)).toBeInTheDocument();

    // Select category for the expense
    await fireEvent.change(categorySelects[0], { target: { value: 'SPORTS' } });
    await new Promise((r) => setTimeout(r, 20));

    // Still 1 missing
    expect(screen.getByText(/Assign Categories to Import \(1 missing\)/i)).toBeInTheDocument();

    // Select category for the income
    await fireEvent.change(categorySelects[1], { target: { value: 'SALARY' } });
    await new Promise((r) => setTimeout(r, 20));

    // Now all categorized: Import button is enabled!
    const importBtn = screen.getByRole('button', { name: /Import 1 Expenses & 1 Incomes/i });
    expect(importBtn).not.toBeDisabled();
    await fireEvent.click(importBtn);

    expect(batchExpenseSpy).toHaveBeenCalledTimes(1);
    expect(batchExpenseSpy).toHaveBeenCalledWith(
      [
        expect.objectContaining({
          name: 'DECATHLON',
          cost_cents: 3297,
          expense_date: '2026-08-03',
          who_paid: 'Jim',
          category: 'SPORTS',
        }),
      ],
      '2026-08'
    );

    expect(createIncomeSpy).toHaveBeenCalledTimes(1);
    expect(createIncomeSpy).toHaveBeenCalledWith(
      [
        expect.objectContaining({
          name: 'VLAAMSE VERENIGING VOOR',
          amount_cents: 210568,
          income_date: '2026-08-25',
          who: 'Jim',
          category: 'SALARY',
        }),
      ],
      '2026-08'
    );
  });

  it('detects duplicate matching expenses on import, prompts user, and allows skipping duplicates', async () => {
    const batchExpenseSpy = vi.spyOn(api, 'createExpensesBatch').mockResolvedValue([]);
    const createIncomeSpy = vi.spyOn(api, 'createIncome').mockResolvedValue([]);

    // Seed existing expenses with a matching category and amount
    expenses.set([
      {
        id: 42,
        name: 'DECATHLON Previous Purchase',
        cost_cents: 3297,
        category: 'SPORTS',
        who_paid: 'Jim',
        expense_date: '2026-08-01',
      },
    ]);

    const csvContent = `Account Number,Account Name,Counterparty account,Entry number,Booking date,Value date,Amount,Currency,Description,Entry Details,Message
BE76377126940095,DE H JIM VEKEMANS,,506,03/08/2026,01/08/2026,"-32,97",EUR,Betaling Debit Mastercard 01/08/26 - 16.09 uur - DECATHLON 0252 88000 - EPINAL - FRA Kaartnummer 5229 62XX XXXX 6510 Referentie ING: COP000372131056,,
BE76377126940095,DE H JIM VEKEMANS,BE82363137073568,532,25/08/2026,25/08/2026,"2105,68",EUR,Overschrijving in euro (SEPA) Van: VLAAMSE VERENIGING VOOR - BE82363137073568 Mededeling: Salary,,
`;

    const { container } = render(BankCsvImportModal, { props: { isOpen: true } });

    const file = new File([csvContent], 'statement.csv', { type: 'text/csv' });
    const input = container.querySelector('input[type="file"]');
    await fireEvent.change(input, { target: { files: [file] } });
    await new Promise((r) => setTimeout(r, 50));

    // Assign categories
    const categorySelects = container.querySelectorAll('tbody tr td:nth-child(6) select');
    await fireEvent.change(categorySelects[0], { target: { value: 'SPORTS' } });
    await fireEvent.change(categorySelects[1], { target: { value: 'SALARY' } });
    await new Promise((r) => setTimeout(r, 20));

    // Notice the duplicate badge in the table
    expect(screen.getByText(/Duplicate match/i)).toBeInTheDocument();

    // Click Import
    const importBtn = screen.getByRole('button', { name: /Import 1 Expenses & 1 Incomes/i });
    await fireEvent.click(importBtn);

    // Duplicate prompt modal should appear
    expect(screen.getByText(/Potential Duplicate Expense Detected/i)).toBeInTheDocument();
    expect(screen.getByText(/This expense might already have been entered:/i)).toBeInTheDocument();
    expect(screen.getByText('DECATHLON Previous Purchase')).toBeInTheDocument();
    expect(screen.getByText(/Do you still want to add it\?/i)).toBeInTheDocument();

    // Batch expense not called yet
    expect(batchExpenseSpy).not.toHaveBeenCalled();

    // User chooses "No, Skip Duplicates"
    const skipBtn = screen.getByRole('button', { name: /No, Skip Duplicates/i });
    await fireEvent.click(skipBtn);

    // Only the income should have been imported! The duplicate expense was skipped!
    expect(batchExpenseSpy).not.toHaveBeenCalled();
    expect(createIncomeSpy).toHaveBeenCalledTimes(1);
    expect(createIncomeSpy).toHaveBeenCalledWith(
      [
        expect.objectContaining({
          name: 'VLAAMSE VERENIGING VOOR',
          amount_cents: 210568,
          category: 'SALARY',
        }),
      ],
      '2026-08'
    );
  });
});
