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

  it.each([
    { cat: 'GROCERIES', limitFormatted: '€300.00', pctFormatted: '50.0%' },
  ])('renders configured budget limits table ($cat)', async ({ cat, limitFormatted, pctFormatted }) => {
    render(BudgetManager);

    await new Promise((r) => setTimeout(r, 10));

    expect(screen.getAllByText(cat).length).toBeGreaterThan(0);
    expect(screen.getByText(limitFormatted)).toBeInTheDocument();
    expect(screen.getByText(pctFormatted)).toBeInTheDocument();
  });

  it.each([
    { expectedError: 'Choose a category and enter a valid amount.' },
  ])('validates form submission when category or amount is missing/invalid', async ({ expectedError }) => {
    render(BudgetManager);

    const saveBtn = screen.getByRole('button', { name: /Save/i });
    await fireEvent.click(saveBtn);

    expect(screen.getByText(expectedError)).toBeInTheDocument();
  });

  it.each([
    { category: 'UTILITIES', limitStr: '150.00', expectedCents: 15000 },
  ])('successfully submits new budget limit ($category)', async ({ category, limitStr, expectedCents }) => {
    const upsertSpy = vi.spyOn(api, 'upsertBudget').mockResolvedValue({});

    render(BudgetManager);

    const categorySelect = screen.getByLabelText(/Category/i);
    await fireEvent.change(categorySelect, { target: { value: category } });

    const limitInput = screen.getByLabelText(/Limit/i);
    await fireEvent.input(limitInput, { target: { value: limitStr } });

    const saveBtn = screen.getByRole('button', { name: /Save/i });
    await fireEvent.click(saveBtn);

    expect(upsertSpy).toHaveBeenCalledWith({
      category: category,
      month: 'ALL',
      limit_cents: expectedCents,
    });
  });

  it.each([
    { targetCat: 'GROCERIES', month: 'ALL' },
  ])('handles budget deletion flow with inline confirmation ($targetCat)', async ({ targetCat, month }) => {
    const delSpy = vi.spyOn(api, 'deleteBudget').mockResolvedValue({});

    render(BudgetManager);

    const removeBtn = screen.getByTitle('Remove');
    await fireEvent.click(removeBtn);

    const yesBtn = screen.getByRole('button', { name: 'Yes' });
    await fireEvent.click(yesBtn);

    expect(delSpy).toHaveBeenCalledWith(targetCat, month);
  });
});
