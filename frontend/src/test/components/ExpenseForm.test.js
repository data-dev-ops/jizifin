import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import ExpenseForm from '../../lib/ExpenseForm.svelte';
import { users, splits, settlements, defaultPayer, defaultCategory, selectedMonth } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('ExpenseForm.svelte — Expense Creation Form', () => {
  beforeEach(() => {
    selectedMonth.set('2026-07');
    users.set([
      { name: 'Jim', color: '#6366f1', is_active: true },
      { name: 'Zina', color: '#ec4899', is_active: true },
    ]);
    splits.set([
      { category: 'GROCERIES' },
      { category: 'RENT' },
    ]);
    settlements.set([
      { month: '2026-05', settled_at: '2026-06-01', net_balance_transferred_cents: 0 },
    ]);
    defaultPayer.set('Jim');
    defaultCategory.set('GROCERIES');

    vi.restoreAllMocks();
  });

  it('renders form inputs correctly', () => {
    render(ExpenseForm);

    expect(screen.getByPlaceholderText(/e.g. Weekly Groceries/i)).toBeInTheDocument();
    expect(screen.getByPlaceholderText('0.00')).toBeInTheDocument();
  });

  it('validates description field', async () => {
    render(ExpenseForm);

    const submitBtn = screen.getByRole('button', { name: /Log Expense/i });
    await fireEvent.click(submitBtn);

    expect(screen.getByText('Description is required.')).toBeInTheDocument();
  });

  it('validates invalid/zero expense amount', async () => {
    render(ExpenseForm);

    const descInput = screen.getByPlaceholderText(/e.g. Weekly Groceries/i);
    await fireEvent.input(descInput, { target: { value: 'Supermarket' } });

    const submitBtn = screen.getByRole('button', { name: /Log Expense/i });
    await fireEvent.click(submitBtn);

    expect(screen.getByText('Amount is required.')).toBeInTheDocument();
  });

  it('displays lock indicator when expense date is in a settled month', async () => {
    render(ExpenseForm);

    const dateInput = screen.getByLabelText(/Date/i);
    await fireEvent.input(dateInput, { target: { value: '2026-05-15' } });

    expect(screen.getByText(/This month is locked/i)).toBeInTheDocument();
  });

  it('submits valid expense payload converted to cents', async () => {
    const createSpy = vi.spyOn(api, 'createExpense').mockResolvedValue({});

    render(ExpenseForm);

    const descInput = screen.getByPlaceholderText(/e.g. Weekly Groceries/i);
    await fireEvent.input(descInput, { target: { value: 'Organic Veggies' } });

    const amountInput = screen.getByPlaceholderText('0.00');
    await fireEvent.input(amountInput, { target: { value: '34.50' } });

    const submitBtn = screen.getByRole('button', { name: /Log Expense/i });
    await fireEvent.click(submitBtn);

    expect(createSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        name: 'Organic Veggies',
        cost_cents: 3450,
        who_paid: 'Jim',
        category: 'GROCERIES',
      }),
      '2026-07'
    );
  });
});
