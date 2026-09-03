import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import ExpenseForm from '../../lib/ExpenseForm.svelte';
import { users, splits, settlements, defaultPayer, defaultCategory, selectedMonth, expenses } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('ExpenseForm.svelte — Expense Creation Form', () => {
  beforeEach(() => {
    selectedMonth.set('2026-07');
    expenses.set([]);
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
    { desc: 'Supermarket', expectedError: 'Cost is required.' },
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

  it('hides tag dropdown when there are no active tags', async () => {
    const { tags } = await import('../../lib/stores.js');
    tags.set([
      { id: 1, name: 'Closed Tag', color: '#f59e0b', is_active: false },
    ]);

    render(ExpenseForm);

    expect(document.getElementById('expense-tag')).not.toBeInTheDocument();
  });

  it('shows tag dropdown when active tags exist', async () => {
    const { tags } = await import('../../lib/stores.js');
    tags.set([
      { id: 1, name: 'Active Vacation', color: '#f59e0b', is_active: true },
    ]);

    render(ExpenseForm);

    expect(document.getElementById('expense-tag')).toBeInTheDocument();
    expect(screen.getByText(/Active Vacation/i)).toBeInTheDocument();
  });

  it('prompts user when a matching expense with same category and amount already exists, allowing cancellation', async () => {
    const createSpy = vi.spyOn(api, 'createExpense').mockResolvedValue({});
    expenses.set([
      {
        id: 99,
        name: 'Existing Supermarket Bill',
        cost_cents: 4500,
        category: 'GROCERIES',
        who_paid: 'John',
        expense_date: '2026-07-02',
      },
    ]);

    render(ExpenseForm);

    const descInput = screen.getByPlaceholderText(/e.g. Weekly Groceries/i);
    await fireEvent.input(descInput, { target: { value: 'New Supermarket Bill' } });

    const amountInput = screen.getByPlaceholderText('0.00');
    await fireEvent.input(amountInput, { target: { value: '45.00' } });

    const submitBtn = screen.getByRole('button', { name: /Log Expense/i });
    await fireEvent.click(submitBtn);

    // Duplicate prompt modal should appear
    expect(screen.getByText(/Potential Duplicate Expense Detected/i)).toBeInTheDocument();
    expect(screen.getByText(/This expense might already have been entered:/i)).toBeInTheDocument();
    expect(screen.getByText('Existing Supermarket Bill')).toBeInTheDocument();
    expect(screen.getByText(/Do you still want to add it\?/i)).toBeInTheDocument();

    // Expense is not saved yet
    expect(createSpy).not.toHaveBeenCalled();

    // Click "No, Cancel"
    const cancelBtn = screen.getByRole('button', { name: /No, Cancel/i });
    await fireEvent.click(cancelBtn);

    // Prompt closed, createExpense never called
    expect(screen.queryByText(/Potential Duplicate Expense Detected/i)).not.toBeInTheDocument();
    expect(createSpy).not.toHaveBeenCalled();
  });

  it('prompts user when duplicate matches, and saves when user chooses "Yes, Add Anyway"', async () => {
    const createSpy = vi.spyOn(api, 'createExpense').mockResolvedValue({});
    expenses.set([
      {
        id: 101,
        name: 'Existing Grocery Run',
        cost_cents: 2500,
        category: 'GROCERIES',
        who_paid: 'Jane',
        expense_date: '2026-07-10',
      },
    ]);

    render(ExpenseForm);

    const descInput = screen.getByPlaceholderText(/e.g. Weekly Groceries/i);
    await fireEvent.input(descInput, { target: { value: 'Another Grocery Run' } });

    const amountInput = screen.getByPlaceholderText('0.00');
    await fireEvent.input(amountInput, { target: { value: '25.00' } });

    const submitBtn = screen.getByRole('button', { name: /Log Expense/i });
    await fireEvent.click(submitBtn);

    // Prompt appears
    expect(screen.getByText(/Potential Duplicate Expense Detected/i)).toBeInTheDocument();

    // Click "Yes, Add Anyway"
    const confirmBtn = screen.getByRole('button', { name: /Yes, Add Anyway/i });
    await fireEvent.click(confirmBtn);

    expect(createSpy).toHaveBeenCalledTimes(1);
    expect(createSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        name: 'Another Grocery Run',
        cost_cents: 2500,
        category: 'GROCERIES',
      }),
      '2026-07'
    );
  });
});
