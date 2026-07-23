import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import RecurringManager from '../../lib/RecurringManager.svelte';
import { recurringExpenses, splits, users } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('RecurringManager.svelte — Recurring Expense Templates', () => {
  beforeEach(() => {
    users.set([{ name: 'Jim', color: '#6366f1', is_active: true }]);
    splits.set([{ category: 'UTILITIES' }]);
    recurringExpenses.set([
      {
        id: 1,
        name: 'Internet Bill',
        cost_cents: 4500,
        who_paid: 'Jim',
        category: 'UTILITIES',
        day_of_month: 1,
      },
    ]);
    vi.restoreAllMocks();
  });

  it('renders configured recurring templates table', () => {
    render(RecurringManager);

    expect(screen.getByText('Internet Bill')).toBeInTheDocument();
    expect(screen.getByText('€45.00')).toBeInTheDocument();
    expect(screen.getByText('1st')).toBeInTheDocument();
  });

  it('validates form submission fields', async () => {
    render(RecurringManager);

    const addBtn = screen.getByRole('button', { name: /\+ Add Recurring/i });
    await fireEvent.click(addBtn);

    expect(screen.getByText('Fill in all fields with valid values.')).toBeInTheDocument();
  });

  it('submits valid recurring expense template', async () => {
    const createSpy = vi.spyOn(api, 'createRecurring').mockResolvedValue({});

    render(RecurringManager);

    const nameInput = document.getElementById('rec-name');
    await fireEvent.input(nameInput, { target: { value: 'Spotify' } });

    const amountInput = document.getElementById('rec-amount');
    await fireEvent.input(amountInput, { target: { value: '14.99' } });

    const categorySelect = document.getElementById('rec-cat');
    await fireEvent.change(categorySelect, { target: { value: 'UTILITIES' } });

    const addBtn = screen.getByRole('button', { name: /\+ Add Recurring/i });
    await fireEvent.click(addBtn);

    expect(createSpy).toHaveBeenCalledWith({
      name: 'Spotify',
      cost_cents: 1499,
      who_paid: 'Jim',
      category: 'UTILITIES',
      day_of_month: 1,
    });
  });
});
