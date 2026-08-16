import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import RecurringManager from '../../lib/RecurringManager.svelte';
import { recurringExpenses, splits, users, selectedMonth } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('RecurringManager.svelte — Recurring Expense Templates & Frequency Intervals', () => {
  beforeEach(() => {
    selectedMonth.set('2026-08');
    users.set([{ name: 'John', color: '#6366f1', is_active: true }]);
    splits.set([{ category: 'UTILITIES' }, { category: 'FITNESS' }]);
    recurringExpenses.set([
      {
        id: 1,
        name: 'Internet Bill',
        cost_cents: 4500,
        who_paid: 'John',
        category: 'UTILITIES',
        frequency: 'monthly',
        day_of_month: 1,
        start_date: '2026-01-01',
        is_active: true,
      },
      {
        id: 2,
        name: 'Gym Membership',
        cost_cents: 4000,
        who_paid: 'John',
        category: 'FITNESS',
        frequency: '4-weekly',
        start_date: '2026-08-02',
        is_active: true,
      },
    ]);
    vi.restoreAllMocks();
  });

  it.each([
    { recName: 'Internet Bill', costFormatted: '€45.00', dayOrdinal: '1st' },
  ])('renders configured recurring templates table ($recName)', ({ recName, costFormatted, dayOrdinal }) => {
    render(RecurringManager);

    expect(screen.getByText(recName)).toBeInTheDocument();
    expect(screen.getAllByText(costFormatted).length).toBeGreaterThan(0);
    expect(screen.getByText(dayOrdinal)).toBeInTheDocument();
    expect(screen.getByText('Recurring Commitments')).toBeInTheDocument();
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
  ])('submits valid recurring expense template with frequency and timeline ($recName)', async ({ recName, amountStr, expectedCents }) => {
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

    expect(createSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        name: recName,
        cost_cents: expectedCents,
        who_paid: 'John',
        category: 'UTILITIES',
        frequency: 'monthly',
        day_of_month: 1,
        is_joint: false,
        is_active: true,
      })
    );
  });

  it('calculates 4-weekly occurrences and monthly total in category breakdown', () => {
    render(RecurringManager);

    // Internet (1x €45) + Gym (2x Aug 2, Aug 30 = 2x €40 = €80) -> Total = €125.00
    expect(screen.getAllByText('€125.00').length).toBeGreaterThan(0);
    expect(screen.getByText(/2× \(€80\.00\)/i)).toBeInTheDocument();
  });

  it('opens edit modal and saves changes via updateRecurring', async () => {
    const updateSpy = vi.spyOn(api, 'updateRecurring').mockResolvedValue({
      id: 1,
      name: 'Gigabit Internet',
      cost_cents: 6000,
      who_paid: 'John',
      category: 'UTILITIES',
      frequency: 'monthly',
      day_of_month: 1,
      start_date: '2026-01-01',
      is_active: true,
    });

    render(RecurringManager);

    const editBtns = screen.getAllByTitle('Edit recurring expense');
    await fireEvent.click(editBtns[0]);

    expect(screen.getByText('Edit Recurring Commitment')).toBeInTheDocument();

    const nameInput = document.getElementById('edit-rec-name');
    await fireEvent.input(nameInput, { target: { value: 'Gigabit Internet' } });

    const amountInput = document.getElementById('edit-rec-amount');
    await fireEvent.input(amountInput, { target: { value: '60.00' } });

    const saveBtn = screen.getByRole('button', { name: /Save Changes/i });
    await fireEvent.click(saveBtn);

    expect(updateSpy).toHaveBeenCalledWith(
      1,
      expect.objectContaining({
        name: 'Gigabit Internet',
        cost_cents: 6000,
      })
    );
  });
});
