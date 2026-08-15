import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import ExpenseForm from '../../lib/ExpenseForm.svelte';
import { users, splits, settlements, defaultPayer, defaultCategory, selectedMonth } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('ExpenseForm.svelte — Expense Creation Form', () => {
  beforeEach(() => {
    selectedMonth.set('2026-07');
    users.set([
      { name: 'John', color: '#6366f1', is_active: true },
      { name: 'Jane', color: '#ec4899', is_active: true },
    ]);
    splits.set([
      { category: 'GROCERIES' },
      { category: 'RENT' },
    ]);
    settlements.set([
      { month: '2026-05', settled_at: '2026-06-01', net_balance_transferred_cents: 0 },
    ]);
    defaultPayer.set('John');
    defaultCategory.set('GROCERIES');

    vi.restoreAllMocks();
  });

  it.each([
    { placeholderText: '0.00' },
  ])('renders form inputs correctly ($placeholderText)', ({ placeholderText }) => {
    render(ExpenseForm);

    expect(screen.getByPlaceholderText(/e.g. Weekly Groceries/i)).toBeInTheDocument();
    expect(screen.getByPlaceholderText(placeholderText)).toBeInTheDocument();
  });

  it.each([
    { expectedError: 'Description is required.' },
  ])('validates description field', async ({ expectedError }) => {
    render(ExpenseForm);

    const submitBtn = screen.getByRole('button', { name: /Log Expense/i });
    await fireEvent.click(submitBtn);

    expect(screen.getByText(expectedError)).toBeInTheDocument();
  });

  it.each([
    { desc: 'Supermarket', expectedError: 'Amount is required.' },
  ])('validates invalid/zero expense amount ($desc)', async ({ desc, expectedError }) => {
    render(ExpenseForm);

    const descInput = screen.getByPlaceholderText(/e.g. Weekly Groceries/i);
    await fireEvent.input(descInput, { target: { value: desc } });

    const submitBtn = screen.getByRole('button', { name: /Log Expense/i });
    await fireEvent.click(submitBtn);

    expect(screen.getByText(expectedError)).toBeInTheDocument();
  });

  it.each([
    { lockedDate: '2026-05-15' },
  ])('displays lock indicator when expense date is in a settled month ($lockedDate)', async ({ lockedDate }) => {
    render(ExpenseForm);

    const dateInput = screen.getByLabelText(/Date/i);
    await fireEvent.input(dateInput, { target: { value: lockedDate } });

    expect(screen.getByText(/This month is locked/i)).toBeInTheDocument();
  });

  it.each([
    { desc: 'Organic Veggies', amountStr: '34.50', expectedCents: 3450 },
  ])('submits valid expense payload converted to cents ($desc)', async ({ desc, amountStr, expectedCents }) => {
    const createSpy = vi.spyOn(api, 'createExpense').mockResolvedValue({});

    render(ExpenseForm);

    const descInput = screen.getByPlaceholderText(/e.g. Weekly Groceries/i);
    await fireEvent.input(descInput, { target: { value: desc } });

    const amountInput = screen.getByPlaceholderText('0.00');
    await fireEvent.input(amountInput, { target: { value: amountStr } });

    const submitBtn = screen.getByRole('button', { name: /Log Expense/i });
    await fireEvent.click(submitBtn);

    expect(createSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        name: desc,
        cost_cents: expectedCents,
        who_paid: 'John',
        category: 'GROCERIES',
        is_joint: false,
      }),
      '2026-07'
    );
  });

  it.each([
    { desc: 'Joint Groceries', amountStr: '50.00', expectedCents: 5000 },
  ])('submits expense with is_joint set to true when Paid BY Joint Account is checked ($desc)', async ({ desc, amountStr, expectedCents }) => {
    const createSpy = vi.spyOn(api, 'createExpense').mockResolvedValue({});
    const { jointAccountEnabled } = await import('../../lib/stores.js');
    jointAccountEnabled.set(true);

    render(ExpenseForm);

    const descInput = screen.getByPlaceholderText(/e.g. Weekly Groceries/i);
    await fireEvent.input(descInput, { target: { value: desc } });

    const amountInput = screen.getByPlaceholderText('0.00');
    await fireEvent.input(amountInput, { target: { value: amountStr } });

    const jointCheckbox = document.getElementById('paid-by-joint-checkbox');
    expect(jointCheckbox).toBeInTheDocument();
    await fireEvent.click(jointCheckbox);

    const submitBtn = screen.getByRole('button', { name: /Log Expense/i });
    await fireEvent.click(submitBtn);

    expect(createSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        name: desc,
        cost_cents: expectedCents,
        is_joint: true,
      }),
      '2026-07'
    );
  });
});
