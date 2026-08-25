import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen, waitFor } from '@testing-library/svelte';
import SplitManager from '../../lib/SplitManager.svelte';
import { splits, users, selectedMonth, incomeAnalytics, incomeCategories, jointAccounts, jointCategories } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('SplitManager.svelte — Household Category Splits & Subtab Agreements', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    selectedMonth.set('2026-07');

    users.set([
      { name: 'John', color: '#6366f1', is_active: true },
      { name: 'Jane', color: '#ec4899', is_active: true },
      { name: 'Bob', color: '#10b981', is_active: true },
    ]);

    incomeAnalytics.set([
      { who: 'John', salary_cents: 300000 },
      { who: 'Jane', salary_cents: 200000 },
      { who: 'Bob', salary_cents: 100000 },
    ]);

    splits.set([
      {
        category: 'GROCERIES',
        allocations: [
          { user_name: 'John', pct: 50 },
          { user_name: 'Jane', pct: 50 },
          { user_name: 'Bob', pct: 0 },
        ],
      },
      {
        category: 'UTILITIES',
        allocations: [
          { user_name: 'John', pct: 34 },
          { user_name: 'Jane', pct: 33 },
          { user_name: 'Bob', pct: 33 },
        ],
      },
    ]);

    jointAccounts.set([
      { id: 1, name: 'Couple Account', member_names: ['John', 'Jane'] },
    ]);

    jointCategories.set([]);
    incomeCategories.set([
      { category: 'BONUS' },
      { category: 'GIFT' },
    ]);

    vi.spyOn(api, 'updateSplit').mockResolvedValue({});
    vi.spyOn(api, 'fetchLatestSalaries').mockResolvedValue([
      { who: 'John', amount_cents: 300000 },
      { who: 'Jane', amount_cents: 200000 },
      { who: 'Bob', amount_cents: 100000 },
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
    { cat: 'GROCERIES', expectedJohnPct: 50, expectedJanePct: 33, expectedBobPct: 17 },
  ])('calculates salary ratios correctly using Largest Remainder Method from fetched salaries ($cat)', async ({ cat, expectedJohnPct, expectedJanePct, expectedBobPct }) => {
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
        { user_name: 'Bob', pct: expectedBobPct },
      ],
    });
  });

  it('switches between subtabs (Agreements, Matrix, Batch Builder, Income)', async () => {
    render(SplitManager);

    // Default is Agreements
    expect(screen.getByText('Subgroup Split Preset Bar')).toBeInTheDocument();

    // Switch to Matrix
    const matrixNav = document.getElementById('split-nav-matrix');
    await fireEvent.click(matrixNav);
    expect(screen.getByText('Household Category Agreement Matrix')).toBeInTheDocument();

    // Switch to Batch
    const batchNav = document.getElementById('split-nav-batch');
    await fireEvent.click(batchNav);
    expect(screen.getByText('Batch Category Split Rule Applicator')).toBeInTheDocument();

    // Switch to Income
    const incomeNav = document.getElementById('split-nav-income');
    await fireEvent.click(incomeNav);
    expect(screen.getByText('Income Classification Categories')).toBeInTheDocument();
    expect(screen.getByText('BONUS')).toBeInTheDocument();
  });

  it('applies batch split rules across multiple categories', async () => {
    render(SplitManager);

    const batchNav = document.getElementById('split-nav-batch');
    await fireEvent.click(batchNav);

    // Select all categories
    const selectAllBtn = screen.getByText(/Select All/i);
    await fireEvent.click(selectAllBtn);

    // Click apply batch
    const applyBtn = screen.getByRole('button', { name: /Apply Split Rule to 2 Category/i });
    await fireEvent.click(applyBtn);

    expect(api.updateSplit).toHaveBeenCalledTimes(2);
    expect(await screen.findByText(/Successfully updated 2 category split agreement/i)).toBeInTheDocument();
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
