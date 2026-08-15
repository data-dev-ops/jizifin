import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import RecurringManager from '../../lib/RecurringManager.svelte';
import { recurringExpenses, splits, users } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('RecurringManager.svelte — Recurring Expense Templates', () => {
  beforeEach(() => {
    users.set([{ name: 'John', color: '#6366f1', is_active: true }]);
    splits.set([{ category: 'UTILITIES' }]);
    recurringExpenses.set([
      {
        id: 1,
        name: 'Internet Bill',
        cost_cents: 4500,
        who_paid: 'John',
        category: 'UTILITIES',
        day_of_month: 1,
      },
    ]);
    vi.restoreAllMocks();
  });

  it.each([
    { recName: 'Internet Bill', costFormatted: '€45.00', dayOrdinal: '1st' },
  ])('renders configured recurring templates table ($recName)', ({ recName, costFormatted, dayOrdinal }) => {
    render(RecurringManager);

    expect(screen.getByText(recName)).toBeInTheDocument();
    expect(screen.getByText(costFormatted)).toBeInTheDocument();
    expect(screen.getByText(dayOrdinal)).toBeInTheDocument();
  });

  it.each([
    { expectedErr: 'Fill in all fields with valid values.' },
  ])('validates form submission fields ($expectedErr)', async ({ expectedErr }) => {
    render(RecurringManager);

    const addBtn = screen.getByRole('button', { name: /\+ Add Recurring/i });
    await fireEvent.click(addBtn);

    expect(screen.getByText(expectedErr)).toBeInTheDocument();
  });

  it.each([
    { recName: 'Spotify', amountStr: '14.99', expectedCents: 1499 },
  ])('submits valid recurring expense template ($recName)', async ({ recName, amountStr, expectedCents }) => {
    const createSpy = vi.spyOn(api, 'createRecurring').mockResolvedValue({});

    render(RecurringManager);

    const nameInput = document.getElementById('rec-name');
    await fireEvent.input(nameInput, { target: { value: recName } });

    const amountInput = document.getElementById('rec-amount');
    await fireEvent.input(amountInput, { target: { value: amountStr } });

    const categorySelect = document.getElementById('rec-cat');
    await fireEvent.change(categorySelect, { target: { value: 'UTILITIES' } });

    const addBtn = screen.getByRole('button', { name: /\+ Add Recurring/i });
    await fireEvent.click(addBtn);

    expect(createSpy).toHaveBeenCalledWith({
      name: recName,
      cost_cents: expectedCents,
      who_paid: 'John',
      category: 'UTILITIES',
      day_of_month: 1,
      is_joint: false,
    });
  });
});
