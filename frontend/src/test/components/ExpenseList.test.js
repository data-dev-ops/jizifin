import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import ExpenseList from '../../lib/ExpenseList.svelte';
import { expenses, selectedMonth, projects, tags, settlements, users } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('ExpenseList.svelte — Expense Ledger List', () => {
  beforeEach(() => {
    selectedMonth.set('2026-07');
    users.set([{ name: 'Jim', color: '#6366f1' }]);
    projects.set([{ id: 1, name: 'Vacation Fund' }]);
    tags.set([{ id: 1, name: 'Summer', color: '#f59e0b' }]);
    settlements.set([{ month: '2026-05', settled_at: '2026-06-01', net_balance_transferred_cents: 0 }]);

    expenses.set([
      {
        id: 101,
        name: 'Supermarket Groceries',
        cost_cents: 4500,
        expense_date: '2026-07-15',
        who_paid: 'Jim',
        category: 'GROCERIES',
        project_id: 1,
        tag_id: 1,
      },
      {
        id: 102,
        name: 'Old Rent Payment',
        cost_cents: 80000,
        expense_date: '2026-05-01',
        who_paid: 'Jim',
        category: 'RENT',
      },
    ]);

    vi.restoreAllMocks();
  });

  it('renders expenses matching the selected month with project and tag badges', () => {
    render(ExpenseList);

    expect(screen.getByText('Supermarket Groceries')).toBeInTheDocument();
    expect(screen.getByText('15/07/2026')).toBeInTheDocument();
    expect(screen.getByText('€45.00')).toBeInTheDocument();
    expect(screen.getByText(/Vacation Fund/i)).toBeInTheDocument();
    expect(screen.getByText(/Summer/i)).toBeInTheDocument();

    // The May expense should not appear when July is selected
    expect(screen.queryByText('Old Rent Payment')).not.toBeInTheDocument();
  });

  it('renders locked indicator for expenses in a settled month', () => {
    selectedMonth.set('2026-05');
    render(ExpenseList);

    expect(screen.getByText('Old Rent Payment')).toBeInTheDocument();
    expect(screen.getByText('🔒 Locked')).toBeInTheDocument();
  });

  it('handles delete flow with inline confirmation and allows cancellation', async () => {
    const deleteSpy = vi.spyOn(api, 'deleteExpense').mockResolvedValue({});

    render(ExpenseList);

    const deleteBtn = screen.getByTitle('Delete expense');
    await fireEvent.click(deleteBtn);

    expect(screen.getByText('Delete?')).toBeInTheDocument();

    // Click No / Cancel
    const noBtn = screen.getByRole('button', { name: 'No' });
    await fireEvent.click(noBtn);

    expect(screen.queryByText('Delete?')).not.toBeInTheDocument();
    expect(deleteSpy).not.toHaveBeenCalled();

    // Trigger delete and confirm with Yes
    await fireEvent.click(screen.getByTitle('Delete expense'));
    const yesBtn = screen.getByRole('button', { name: 'Yes' });
    await fireEvent.click(yesBtn);

    expect(deleteSpy).toHaveBeenCalledWith(101, '2026-07');
  });

  it('displays delete error message when deletion API fails', async () => {
    vi.spyOn(api, 'deleteExpense').mockRejectedValue(new Error('Network error deleting expense'));

    render(ExpenseList);

    const deleteBtn = screen.getByTitle('Delete expense');
    await fireEvent.click(deleteBtn);

    const yesBtn = screen.getByRole('button', { name: 'Yes' });
    await fireEvent.click(yesBtn);

    expect(await screen.findByText('Network error deleting expense')).toBeInTheDocument();
  });

  it('renders empty message when no expenses exist for selected month', () => {
    selectedMonth.set('2026-08');
    render(ExpenseList);

    expect(screen.getByText('No expenses for this month.')).toBeInTheDocument();
  });
});
