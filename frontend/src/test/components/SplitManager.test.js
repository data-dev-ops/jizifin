import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import SplitManager from '../../lib/SplitManager.svelte';
import { splits, users, selectedMonth, mobileSplitsEditable } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('SplitManager.svelte — Household Category Splits & Salary Ratios', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    selectedMonth.set('2026-07');
    mobileSplitsEditable.set(true);

    users.set([
      { name: 'Jim', color: '#6366f1', is_active: true },
      { name: 'Zina', color: '#ec4899', is_active: true },
    ]);

    splits.set([
      {
        category: 'GROCERIES',
        allocations: [
          { user_name: 'Jim', pct: 50 },
          { user_name: 'Zina', pct: 50 },
        ],
      },
    ]);

    vi.spyOn(api, 'updateSplit').mockResolvedValue({});
  });

  it('renders category split allocation inputs', async () => {
    render(SplitManager);

    expect(await screen.findByText('GROCERIES')).toBeInTheDocument();
    expect(screen.getByText('Monthly Salaries')).toBeInTheDocument();
  });

  it('calculates salary ratios correctly using Largest Remainder Method', async () => {
    render(SplitManager);

    const jimSalary = document.getElementById('salary-Jim');
    const zinaSalary = document.getElementById('salary-Zina');

    await fireEvent.input(jimSalary, { target: { value: '3000' } });
    await fireEvent.input(zinaSalary, { target: { value: '2000' } });

    const resetBtn = document.getElementById('reset-split-GROCERIES');
    expect(resetBtn).not.toBeDisabled();

    await fireEvent.click(resetBtn);

    const saveBtn = document.getElementById('save-split-GROCERIES');
    await fireEvent.click(saveBtn);

    expect(api.updateSplit).toHaveBeenCalledWith('GROCERIES', {
      allocations: [
        { user_name: 'Jim', pct: 60 },
        { user_name: 'Zina', pct: 40 },
      ],
    });
  });
});
