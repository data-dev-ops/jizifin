import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen, waitFor } from '@testing-library/svelte';
import ExpenseForm from '../../lib/ExpenseForm.svelte';
import SettingsTab from '../../lib/SettingsTab.svelte';
import * as api from '../../lib/api.js';
import {
  authSalt,
  cryptoKey,
  users,
  splits,
  expenses
} from '../../lib/stores.js';
import { deriveKey } from '../../lib/crypto.js';

describe('Visual Webpage Interaction & Form Entry Tests', () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    const key = await deriveKey('xps');
    authSalt.set('xps');
    cryptoKey.set(key);

    users.set([
      { name: 'Alice', color: '#6366f1', is_active: true },
      { name: 'Bob', color: '#ec4899', is_active: true }
    ]);

    splits.set([
      { category: 'GROCERIES' },
      { category: 'UTILITIES' },
      { category: 'RENT' }
    ]);

    expenses.set([]);
  });

  it('simulates keyboard typing and mouse clicks in ExpenseForm to add an expense', async () => {
    const mockExpense = {
      id: 42,
      name: 'Farmer Market Berries',
      cost_cents: 1450,
      cost: 14.50,
      expense_date: '2026-08-27',
      who_paid: 'Alice',
      category: 'GROCERIES',
      is_joint: 0
    };

    vi.spyOn(api, 'createExpense').mockImplementation(async (payload) => {
      expenses.update((list) => [mockExpense, ...list]);
      return mockExpense;
    });

    render(ExpenseForm);

    // 1. Keyboard entry: Description / Name (#expense-name)
    const descInput = document.getElementById('expense-name');
    expect(descInput).toBeInTheDocument();
    await fireEvent.input(descInput, { target: { value: 'Farmer Market Berries' } });

    // 2. Keyboard entry: Amount / Cost (#expense-cost)
    const costInput = document.getElementById('expense-cost');
    expect(costInput).toBeInTheDocument();
    await fireEvent.input(costInput, { target: { value: '14.50' } });

    // 3. Mouse click: Category select (#expense-category)
    const catSelect = document.getElementById('expense-category');
    expect(catSelect).toBeInTheDocument();
    await fireEvent.change(catSelect, { target: { value: 'GROCERIES' } });

    // 4. Mouse click: Payer checkbox (#who-paid-alice)
    const aliceBox = document.getElementById('who-paid-alice');
    expect(aliceBox).toBeInTheDocument();
    await fireEvent.click(aliceBox);

    // 5. Mouse click: Submit button (#submit-expense)
    const submitBtn = document.getElementById('submit-expense');
    expect(submitBtn).toBeInTheDocument();
    await fireEvent.click(submitBtn);

    // 6. Verify API was called and expense store updated
    await waitFor(() => {
      expect(api.createExpense).toHaveBeenCalled();
    });

    let currentExpenses = [];
    expenses.subscribe((v) => { currentExpenses = v; })();
    expect(currentExpenses.length).toBe(1);
    expect(currentExpenses[0].name).toBe('Farmer Market Berries');
    expect(currentExpenses[0].cost).toBe(14.50);
  });

  it('simulates mouse navigation to Settings tab and executing Start from Scratch reset modal', async () => {
    vi.spyOn(api, 'resetDatabase').mockResolvedValue({ status: 'ok' });

    render(SettingsTab, { props: { tabs: [{ id: 'settings', label: 'Settings' }] } });

    // 1. Mouse click: Open Start from Scratch modal
    const scratchBtn = document.getElementById('start-from-scratch-btn');
    expect(scratchBtn).toBeInTheDocument();
    await fireEvent.click(scratchBtn);

    expect(screen.getByText(/Confirm Reset to Scratch/i)).toBeInTheDocument();

    // 2. Keyboard entry: Type current salt in verification prompt
    const confirmInput = document.getElementById('reset-confirm-salt');
    expect(confirmInput).toBeInTheDocument();
    await fireEvent.input(confirmInput, { target: { value: 'xps' } });

    // 3. Mouse click: Confirm reset
    const confirmBtn = document.getElementById('confirm-reset-btn');
    expect(confirmBtn).toBeInTheDocument();
    await fireEvent.click(confirmBtn);

    // 4. Verify reset executed
    await waitFor(() => {
      expect(api.resetDatabase).toHaveBeenCalled();
    });
  });
});
