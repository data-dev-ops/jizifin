import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import IncomeTab from '../../lib/IncomeTab.svelte';
import { selectedMonth, users, incomeEntries, incomeCategories } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('IncomeTab.svelte — Income Ledger & Category Management', () => {
  beforeEach(() => {
    selectedMonth.set('2026-07');
    users.set([{ name: 'Jim', color: '#6366f1', is_active: true }]);
    incomeCategories.set([{ category: 'SALARY' }, { category: 'FREELANCE' }]);
    incomeEntries.set([
      { id: 1, name: 'July Salary', amount_cents: 350000, who: 'Jim', category: 'SALARY', income_date: '2026-07-01' },
    ]);

    vi.restoreAllMocks();
  });

  it('renders income entries and categories correctly', async () => {
    render(IncomeTab);

    expect(screen.getByText('July Salary')).toBeInTheDocument();
    expect(screen.getAllByText('€3500.00').length).toBeGreaterThan(0);
    expect(screen.getAllByText('SALARY').length).toBeGreaterThan(0);
  });

  it('validates form input before logging income', async () => {
    render(IncomeTab);

    const submitBtn = document.getElementById('income-submit');
    await fireEvent.click(submitBtn);

    expect(screen.getByText('Name is required.')).toBeInTheDocument();
  });

  it('logs income successfully when form is valid', async () => {
    const createSpy = vi.spyOn(api, 'createIncome').mockResolvedValue({});
    vi.spyOn(api, 'fetchIncome').mockResolvedValue([]);

    render(IncomeTab);

    const nameInput = document.getElementById('income-name');
    await fireEvent.input(nameInput, { target: { value: 'Consulting Fee' } });

    const amountInput = document.getElementById('income-amount');
    await fireEvent.input(amountInput, { target: { value: '500.00' } });

    const submitBtn = document.getElementById('income-submit');
    await fireEvent.click(submitBtn);

    expect(createSpy).toHaveBeenCalled();
  });

  it('adds a new income category', async () => {
    const catSpy = vi.spyOn(api, 'createIncomeCategory').mockResolvedValue({});
    vi.spyOn(api, 'fetchIncomeCategories').mockResolvedValue([]);

    render(IncomeTab);

    const catInput = document.getElementById('income-cat-input');
    await fireEvent.input(catInput, { target: { value: 'BONUS' } });

    const addBtn = document.getElementById('income-cat-add');
    await fireEvent.click(addBtn);

    expect(catSpy).toHaveBeenCalledWith('BONUS');
  });

  it('deletes an income entry on double click confirm', async () => {
    const delSpy = vi.spyOn(api, 'deleteIncome').mockResolvedValue({});
    vi.spyOn(api, 'fetchIncome').mockResolvedValue([]);

    render(IncomeTab);

    const delBtn = document.getElementById('delete-income-1');
    await fireEvent.click(delBtn); // first click -> pending confirmation
    await fireEvent.click(delBtn); // second click -> confirm delete

    expect(delSpy).toHaveBeenCalledWith(1);
  });
});
