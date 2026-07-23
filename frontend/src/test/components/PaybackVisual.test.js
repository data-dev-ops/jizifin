import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import PaybackVisual from '../../lib/PaybackVisual.svelte';
import { paybacks, settlements, selectedMonth, users } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('PaybackVisual.svelte — Payback & Settlement Summary', () => {
  beforeEach(() => {
    selectedMonth.set('2026-07');
    users.set([
      { name: 'Jim', color: '#6366f1' },
      { name: 'Zina', color: '#ec4899' },
    ]);
    settlements.set([]);
    vi.restoreAllMocks();
  });

  it('renders "All Settled Up!" when there are no debts', () => {
    paybacks.set({ rows: [], debts: [], month: '2026-07' });

    render(PaybackVisual);

    expect(screen.getByText('All Settled Up!')).toBeInTheDocument();
  });

  it('renders debt transfer card when payback is owed', () => {
    paybacks.set({
      rows: [],
      debts: [{ from_user: 'Jim', to_user: 'Zina', amount: 45.50 }],
      month: '2026-07',
    });

    render(PaybackVisual);

    expect(screen.getByText('Settlement Transfers')).toBeInTheDocument();
    expect(screen.getAllByText('Jim').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Zina').length).toBeGreaterThan(0);
    expect(screen.getByText('€45.50')).toBeInTheDocument();
  });

  it('executes month settlement lock flow successfully', async () => {
    const settleSpy = vi.spyOn(api, 'createSettlement').mockResolvedValue({});
    vi.spyOn(api, 'fetchSettlements').mockResolvedValue([]);

    paybacks.set({
      rows: [],
      debts: [{ from_user: 'Jim', to_user: 'Zina', amount: 45.50 }],
      month: '2026-07',
    });

    render(PaybackVisual);

    const lockBtn = screen.getByRole('button', { name: /Mark as Settled & Lock Month/i });
    await fireEvent.click(lockBtn);

    expect(settleSpy).toHaveBeenCalledWith({
      month: '2026-07',
      net_balance_transferred_cents: 4550,
    });
  });

  it('displays error message when month settlement lock fails', async () => {
    vi.spyOn(api, 'createSettlement').mockRejectedValue(new Error('Settlement failed: month already closed'));

    paybacks.set({
      rows: [],
      debts: [{ from_user: 'Jim', to_user: 'Zina', amount: 45.50 }],
      month: '2026-07',
    });

    render(PaybackVisual);

    const lockBtn = screen.getByRole('button', { name: /Mark as Settled & Lock Month/i });
    await fireEvent.click(lockBtn);

    expect(await screen.findByText('Settlement failed: month already closed')).toBeInTheDocument();
  });

  it('renders locked banner when month is settled', () => {
    settlements.set([
      { month: '2026-07', settled_at: '2026-08-01', net_balance_transferred_cents: 4550 },
    ]);
    paybacks.set({ rows: [], debts: [], month: '2026-07' });

    render(PaybackVisual);

    expect(screen.getByText('✔️ Month Settled')).toBeInTheDocument();
    expect(screen.getByText(/2026-07 was locked on 2026-08-01/i)).toBeInTheDocument();
  });
});
