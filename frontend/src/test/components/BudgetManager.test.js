import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import BudgetManager from '../../lib/BudgetManager.svelte';
import { budgets, splits, selectedMonth } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('BudgetManager.svelte — Budget Limit Configuration', () => {
  beforeEach(() => {
    selectedMonth.set('2026-07');
    budgets.set([
      { category: 'GROCERIES', month: 'ALL', limit_cents: 30000 },
    ]);
    splits.set([
      { category: 'GROCERIES' },
      { category: 'UTILITIES' },
    ]);

    vi.restoreAllMocks();
    vi.spyOn(api, 'fetchBudgetAnalytics').mockResolvedValue([
      { category: 'GROCERIES', limit_cents: 30000, actual_cents: 15000, pct_used: 50.0 },
    ]);
  });

  it('renders configured budget limits table', async () => {
    render(BudgetManager);

    await new Promise((r) => setTimeout(r, 10));

    expect(screen.getAllByText('GROCERIES').length).toBeGreaterThan(0);
    expect(screen.getByText('€300.00')).toBeInTheDocument();
    expect(screen.getByText('50.0%')).toBeInTheDocument();
  });

  it('validates form submission when category or amount is missing/invalid', async () => {
    render(BudgetManager);

    const saveBtn = screen.getByRole('button', { name: /Save/i });
    await fireEvent.click(saveBtn);

    expect(screen.getByText('Choose a category and enter a valid amount.')).toBeInTheDocument();
  });

  it('successfully submits new budget limit', async () => {
    const upsertSpy = vi.spyOn(api, 'upsertBudget').mockResolvedValue({});

    render(BudgetManager);

    const categorySelect = screen.getByLabelText(/Category/i);
    await fireEvent.change(categorySelect, { target: { value: 'UTILITIES' } });

    const limitInput = screen.getByLabelText(/Limit/i);
    await fireEvent.input(limitInput, { target: { value: '150.00' } });

    const saveBtn = screen.getByRole('button', { name: /Save/i });
    await fireEvent.click(saveBtn);

    expect(upsertSpy).toHaveBeenCalledWith({
      category: 'UTILITIES',
      month: 'ALL',
      limit_cents: 15000,
    });
  });

  it('handles budget deletion flow with inline confirmation', async () => {
    const delSpy = vi.spyOn(api, 'deleteBudget').mockResolvedValue({});

    render(BudgetManager);

    const removeBtn = screen.getByTitle('Remove');
    await fireEvent.click(removeBtn);

    const yesBtn = screen.getByRole('button', { name: 'Yes' });
    await fireEvent.click(yesBtn);

    expect(delSpy).toHaveBeenCalledWith('GROCERIES', 'ALL');
  });
});
