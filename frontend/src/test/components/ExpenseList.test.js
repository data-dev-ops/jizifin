import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import ExpenseList from '../../lib/ExpenseList.svelte';
import { expenses, selectedMonth, projects, tags, settlements, users } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('ExpenseList.svelte — Expense Ledger List', () => {
  beforeEach(() => {
    selectedMonth.set('2026-07');
    users.set([{ name: 'John', color: '#6366f1' }]);
    projects.set([{ id: 1, name: 'Vacation Fund' }]);
    tags.set([{ id: 1, name: 'Summer', color: '#f59e0b' }]);
    settlements.set([{ month: '2026-05', settled_at: '2026-06-01', net_balance_transferred_cents: 0 }]);

    expenses.set([
      {
        id: 101,
        name: 'Supermarket Groceries',
        cost_cents: 4500,
        expense_date: '2026-07-15',
        who_paid: 'John',
        category: 'GROCERIES',
        project_id: 1,
        tag_id: 1,
      },
      {
        id: 102,
        name: 'Old Rent Payment',
        cost_cents: 80000,
        expense_date: '2026-05-01',
        who_paid: 'John',
        category: 'RENT',
      },
    ]);

    vi.restoreAllMocks();
  });

  it.each([
    { expName: 'Supermarket Groceries', costStr: '€45.00', formattedDate: '15/07/2026' },
  ])('renders expenses matching the selected month with project and tag badges ($expName)', ({ expName, costStr, formattedDate }) => {
    render(ExpenseList);

    expect(screen.getByText(expName)).toBeInTheDocument();
    expect(screen.getByText(formattedDate)).toBeInTheDocument();
    expect(screen.getAllByText(costStr).length).toBeGreaterThan(0);
    expect(screen.getByText(/Vacation Fund/i)).toBeInTheDocument();
    expect(screen.getAllByText(/Summer/i).length).toBeGreaterThan(0);

    expect(screen.queryByText('Old Rent Payment')).not.toBeInTheDocument();
  });

  it.each([
    { settledMonth: '2026-05', expName: 'Old Rent Payment' },
  ])('renders locked indicator for expenses in a settled month ($settledMonth)', ({ settledMonth, expName }) => {
    selectedMonth.set(settledMonth);
    render(ExpenseList);

    expect(screen.getByText(expName)).toBeInTheDocument();
    expect(screen.getByText('🔒 Locked')).toBeInTheDocument();
  });

  it.each([
    { expId: 101, month: '2026-07' },
  ])('handles delete flow with inline confirmation and allows cancellation ($expId)', async ({ expId, month }) => {
    const deleteSpy = vi.spyOn(api, 'deleteExpense').mockResolvedValue({});

    render(ExpenseList);

    const deleteBtn = screen.getByTitle('Delete expense');
    await fireEvent.click(deleteBtn);

    expect(screen.getByText('Delete?')).toBeInTheDocument();

    const noBtn = screen.getByRole('button', { name: 'No' });
    await fireEvent.click(noBtn);

    expect(screen.queryByText('Delete?')).not.toBeInTheDocument();
    expect(deleteSpy).not.toHaveBeenCalled();

    await fireEvent.click(screen.getByTitle('Delete expense'));
    const yesBtn = screen.getByRole('button', { name: 'Yes' });
    await fireEvent.click(yesBtn);

    expect(deleteSpy).toHaveBeenCalledWith(expId, month);
  });

  it.each([
    { expId: 101, newTagId: 1 },
  ])('assigns tag via quick tag popover on expense row ($expId)', async ({ expId, newTagId }) => {
    const updateSpy = vi.spyOn(api, 'updateExpense').mockResolvedValue({});

    render(ExpenseList);

    const tagBadge = document.getElementById(`tag-badge-${expId}`);
    expect(tagBadge).toBeInTheDocument();
    await fireEvent.click(tagBadge);

    expect(screen.getByText('Assign Tag')).toBeInTheDocument();
    const tagOptionBtn = screen.getByText('Summer');
    await fireEvent.click(tagOptionBtn);

    expect(updateSpy).toHaveBeenCalledWith(expId, { tag_id: newTagId }, '2026-07');
  });

  it.each([
    { expId: 101 },
  ])('opens and saves full expense edit modal ($expId)', async ({ expId }) => {
    const updateSpy = vi.spyOn(api, 'updateExpense').mockResolvedValue({});

    render(ExpenseList);

    const editBtn = document.getElementById(`edit-expense-${expId}`);
    expect(editBtn).toBeInTheDocument();
    await fireEvent.click(editBtn);

    expect(screen.getByText('Edit Expense')).toBeInTheDocument();

    const nameInput = document.getElementById('edit-expense-name');
    await fireEvent.input(nameInput, { target: { value: 'Updated Groceries Name' } });

    const saveBtn = document.getElementById('save-expense-edit-btn');
    await fireEvent.click(saveBtn);

    expect(updateSpy).toHaveBeenCalledWith(
      expId,
      expect.objectContaining({
        name: 'Updated Groceries Name',
        cost_cents: 4500,
        expense_date: '2026-07-15',
        who_paid: 'John',
        category: 'GROCERIES',
      }),
      '2026-07'
    );
  });

  it.each([
    { errMsg: 'Network error deleting expense' },
  ])('displays delete error message when deletion API fails ($errMsg)', async ({ errMsg }) => {
    vi.spyOn(api, 'deleteExpense').mockRejectedValue(new Error(errMsg));

    render(ExpenseList);

    const deleteBtn = screen.getByTitle('Delete expense');
    await fireEvent.click(deleteBtn);

    const yesBtn = screen.getByRole('button', { name: 'Yes' });
    await fireEvent.click(yesBtn);

    expect(await screen.findByText(errMsg)).toBeInTheDocument();
  });

  it.each([
    { emptyMonth: '2026-08' },
  ])('renders empty message when no expenses exist for selected month ($emptyMonth)', ({ emptyMonth }) => {
    selectedMonth.set(emptyMonth);
    render(ExpenseList);

    expect(screen.getByText('No expenses for this month.')).toBeInTheDocument();
  });
});
