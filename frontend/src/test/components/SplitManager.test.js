import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen, waitFor } from '@testing-library/svelte';
import SplitManager from '../../lib/SplitManager.svelte';
import { splits, users, selectedMonth, incomeAnalytics } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('SplitManager.svelte — Household Category Splits & Salary Ratios', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    selectedMonth.set('2026-07');

    users.set([
      { name: 'John', color: '#6366f1', is_active: true },
      { name: 'Jane', color: '#ec4899', is_active: true },
    ]);

    incomeAnalytics.set([
      { who: 'John', salary_cents: 300000 },
      { who: 'Jane', salary_cents: 200000 },
    ]);

    splits.set([
      {
        category: 'GROCERIES',
        allocations: [
          { user_name: 'John', pct: 50 },
          { user_name: 'Jane', pct: 50 },
        ],
      },
    ]);

    vi.spyOn(api, 'updateSplit').mockResolvedValue({});
    vi.spyOn(api, 'fetchLatestSalaries').mockResolvedValue([
      { who: 'John', amount_cents: 300000 },
      { who: 'Jane', amount_cents: 200000 },
    ]);
  });

  it.each([
    { cat: 'GROCERIES' },
  ])('renders category split allocation inputs and current monthly salaries ($cat)', async ({ cat }) => {

    render(SplitManager);

    expect(await screen.findByText(cat)).toBeInTheDocument();
    expect(screen.getByText('Current Monthly Salaries')).toBeInTheDocument();
    expect(screen.getByText('Manage in Income Tab')).toBeInTheDocument();
  });

  it.each([
    { cat: 'GROCERIES', expectedJohnPct: 60, expectedJanePct: 40 },
  ])('calculates salary ratios correctly using Largest Remainder Method from fetched salaries ($cat)', async ({ cat, expectedJohnPct, expectedJanePct }) => {
    const { component } = render(SplitManager);
    let navTriggered = false;
    component.$on('navigateIncome', () => { navTriggered = true; });

    await waitFor(() => {
      expect(screen.getByText(/3000.00/i)).toBeInTheDocument();
    });

    const linkBtn = document.getElementById('link-manage-income');
    expect(linkBtn).toBeInTheDocument();
    await fireEvent.click(linkBtn);
    expect(navTriggered).toBe(true);

    const resetBtn = document.getElementById(`reset-split-${cat}`);
    await fireEvent.click(resetBtn);

    const saveBtn = document.getElementById(`save-split-${cat}`);
    await fireEvent.click(saveBtn);

    expect(api.updateSplit).toHaveBeenCalledWith(cat, {
      allocations: [
        { user_name: 'John', pct: expectedJohnPct },
        { user_name: 'Jane', pct: expectedJanePct },
      ],
    });
  });

  it.each([
    { newCat: 'FREELANCE' },
  ])('adds an income category when Income type toggle is selected ($newCat)', async ({ newCat }) => {
    const incCatSpy = vi.spyOn(api, 'createIncomeCategory').mockResolvedValue({});
    vi.spyOn(api, 'fetchIncomeCategories').mockResolvedValue([]);

    render(SplitManager);

    const input = document.getElementById('new-category');
    await fireEvent.input(input, { target: { value: newCat } });

    const incomeToggle = document.getElementById('cat-type-income');
    await fireEvent.click(incomeToggle);

    const addBtn = document.getElementById('add-category-btn');
    await fireEvent.click(addBtn);

    expect(incCatSpy).toHaveBeenCalledWith(newCat);
  });
});
