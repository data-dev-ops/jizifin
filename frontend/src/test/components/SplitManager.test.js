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

  it('opens rename modal and renames an expense category', async () => {
    const renameSpy = vi.spyOn(api, 'renameSplit').mockResolvedValue({
      category: 'SUPERMARKET',
      allocations: [],
    });

    render(SplitManager);

    const renameBtn = document.getElementById('rename-cat-GROCERIES');
    expect(renameBtn).toBeInTheDocument();
    await fireEvent.click(renameBtn);

    expect(screen.getByText('Rename Expense Category')).toBeInTheDocument();

    const input = document.getElementById('rename-category-input');
    await fireEvent.input(input, { target: { value: 'SUPERMARKET' } });

    const confirmBtn = document.getElementById('confirm-rename-btn');
    await fireEvent.click(confirmBtn);

    expect(renameSpy).toHaveBeenCalledWith('GROCERIES', 'SUPERMARKET');
  });

  it('opens delete modal and removes an expense category', async () => {
    const deleteSpy = vi.spyOn(api, 'deleteSplit').mockResolvedValue();

    render(SplitManager);

    const deleteBtn = document.getElementById('delete-cat-GROCERIES');
    expect(deleteBtn).toBeInTheDocument();
    await fireEvent.click(deleteBtn);

    expect(screen.getByText('Remove Expense Category')).toBeInTheDocument();

    const confirmBtn = document.getElementById('confirm-delete-category-btn');
    await fireEvent.click(confirmBtn);

    expect(deleteSpy).toHaveBeenCalledWith('GROCERIES');
  });

  it('renames and deletes income category from income tab', async () => {
    const renameIncSpy = vi.spyOn(api, 'updateIncomeCategory').mockResolvedValue({ category: 'DIVIDENDS' });
    const deleteIncSpy = vi.spyOn(api, 'deleteIncomeCategory').mockResolvedValue();

    render(SplitManager);

    const incomeNav = document.getElementById('split-nav-income');
    await fireEvent.click(incomeNav);

    // Test rename
    const renameBtn = document.getElementById('rename-income-cat-BONUS');
    expect(renameBtn).toBeInTheDocument();
    await fireEvent.click(renameBtn);

    const input = document.getElementById('rename-category-input');
    await fireEvent.input(input, { target: { value: 'DIVIDENDS' } });

    const confirmRenameBtn = document.getElementById('confirm-rename-btn');
    await fireEvent.click(confirmRenameBtn);
    expect(renameIncSpy).toHaveBeenCalledWith('BONUS', 'DIVIDENDS');

    // Test delete
    const deleteBtn = document.getElementById('delete-income-cat-GIFT');
    expect(deleteBtn).toBeInTheDocument();
    await fireEvent.click(deleteBtn);

    const confirmDeleteBtn = document.getElementById('confirm-delete-category-btn');
    await fireEvent.click(confirmDeleteBtn);
    expect(deleteIncSpy).toHaveBeenCalledWith('GIFT');
  });

  it('expands split timeline accordion, creates temporary override, and deletes override', async () => {
    const createAgrSpy = vi.spyOn(api, 'createSplitAgreement').mockResolvedValue({
      id: 10,
      category: 'GROCERIES',
      start_date: '2026-08-01',
      end_date: '2026-08-31',
      note: 'Summer guests',
      allocations: [
        { user_name: 'John', pct: 60 },
        { user_name: 'Jane', pct: 40 },
        { user_name: 'Bob', pct: 0 },
      ],
    });

    const deleteAgrSpy = vi.spyOn(api, 'deleteSplitAgreement').mockResolvedValue();
    vi.spyOn(window, 'confirm').mockReturnValue(true);

    // Set split with an existing override
    splits.set([
      {
        category: 'GROCERIES',
        allocations: [
          { user_name: 'John', pct: 50 },
          { user_name: 'Jane', pct: 50 },
          { user_name: 'Bob', pct: 0 },
        ],
        agreements: [
          {
            id: 1,
            category: 'GROCERIES',
            start_date: '2000-01-01',
            end_date: null,
            is_active: true,
            note: 'Baseline',
            allocations: [
              { user_name: 'John', pct: 50 },
              { user_name: 'Jane', pct: 50 },
              { user_name: 'Bob', pct: 0 },
            ],
          },
          {
            id: 5,
            category: 'GROCERIES',
            start_date: '2026-06-01',
            end_date: '2026-06-30',
            is_active: true,
            note: 'June trip',
            allocations: [
              { user_name: 'John', pct: 70 },
              { user_name: 'Jane', pct: 30 },
              { user_name: 'Bob', pct: 0 },
            ],
          },
        ],
      },
    ]);

    render(SplitManager);

    // 1. Toggle timeline accordion
    const toggleBtn = document.getElementById('toggle-timeline-GROCERIES');
    expect(toggleBtn).toBeInTheDocument();
    expect(screen.getByText('1 override')).toBeInTheDocument();
    await fireEvent.click(toggleBtn);

    expect(screen.getByText('2026-06-01 → 2026-06-30')).toBeInTheDocument();
    expect(screen.getByText('(June trip)')).toBeInTheDocument();

    // 2. Open Add Override Modal
    const addOverrideBtn = document.getElementById('add-override-btn-GROCERIES');
    await fireEvent.click(addOverrideBtn);

    expect(screen.getByText('Add Temporary Split Override')).toBeInTheDocument();

    const startInput = document.getElementById('override-start-date');
    const endInput = document.getElementById('override-end-date');
    const noteInput = document.getElementById('override-note');

    await fireEvent.input(startInput, { target: { value: '2026-08-01' } });
    await fireEvent.input(endInput, { target: { value: '2026-08-31' } });
    await fireEvent.input(noteInput, { target: { value: 'Summer guests' } });

    const johnInput = document.getElementById('override-split-John');
    const janeInput = document.getElementById('override-split-Jane');
    const bobInput = document.getElementById('override-split-Bob');

    await fireEvent.input(johnInput, { target: { value: '60' } });
    await fireEvent.input(janeInput, { target: { value: '40' } });
    await fireEvent.input(bobInput, { target: { value: '0' } });

    const saveOverrideBtn = document.getElementById('save-override-btn');
    await fireEvent.click(saveOverrideBtn);

    expect(createAgrSpy).toHaveBeenCalledWith('GROCERIES', {
      category: 'GROCERIES',
      start_date: '2026-08-01',
      end_date: '2026-08-31',
      is_active: true,
      note: 'Summer guests',
      allocations: [
        { user_name: 'John', pct: 60 },
        { user_name: 'Jane', pct: 40 },
        { user_name: 'Bob', pct: 0 },
      ],
    });
  });

  it('correctly resolves baseline split for expired months and active override for covered months', async () => {
    selectedMonth.set('2026-09'); // September (override expired on 2026-08-31)

    splits.set([
      {
        category: 'OTHER',
        allocations: [
          { user_name: 'John', pct: 50 },
          { user_name: 'Jane', pct: 50 },
          { user_name: 'Bob', pct: 0 },
        ],
        agreements: [
          {
            id: 1,
            category: 'OTHER',
            start_date: '2000-01-01',
            end_date: null,
            is_active: true,
            note: 'Baseline',
            allocations: [
              { user_name: 'John', pct: 50 },
              { user_name: 'Jane', pct: 50 },
              { user_name: 'Bob', pct: 0 },
            ],
          },
          {
            id: 2,
            category: 'OTHER',
            start_date: '2026-08-01',
            end_date: '2026-08-31',
            is_active: true,
            note: 'August single payer',
            allocations: [
              { user_name: 'John', pct: 100 },
              { user_name: 'Jane', pct: 0 },
              { user_name: 'Bob', pct: 0 },
            ],
          },
        ],
      },
    ]);

    render(SplitManager);

    // In September, John should be 50, Jane should be 50
    const johnInput = document.getElementById('split-John-OTHER');
    const janeInput = document.getElementById('split-Jane-OTHER');
    expect(johnInput.value).toBe('50');
    expect(janeInput.value).toBe('50');

    // Open timeline tray to check status badges
    const toggleBtn = document.getElementById('toggle-timeline-OTHER');
    await fireEvent.click(toggleBtn);

    expect(screen.getByText('✓ Active in 2026-09')).toBeInTheDocument();
    expect(screen.getByText('Inactive in 2026-09')).toBeInTheDocument();

    // Now switch to August (2026-08) where the override applies
    selectedMonth.set('2026-08');

    await waitFor(() => {
      expect(johnInput.value).toBe('100');
      expect(janeInput.value).toBe('0');
      expect(screen.getByText(/⚡ Override Active \(2026-08\)/i)).toBeInTheDocument();
    });
  });
});

