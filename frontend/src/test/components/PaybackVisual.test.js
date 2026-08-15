import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import PaybackVisual from '../../lib/PaybackVisual.svelte';
import { paybacks, settlements, selectedMonth, users } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('PaybackVisual.svelte — Payback & Settlement Summary', () => {
  beforeEach(() => {
    selectedMonth.set('2026-07');
    users.set([
      { name: 'John', color: '#6366f1' },
      { name: 'Jane', color: '#ec4899' },
    ]);
    settlements.set([]);
    vi.restoreAllMocks();
  });

  it.each([
    { statusText: 'All Settled Up!' },
  ])('renders "All Settled Up!" when there are no debts ($statusText)', ({ statusText }) => {
    paybacks.set({ rows: [], debts: [], month: '2026-07' });

    render(PaybackVisual);

    expect(screen.getByText(statusText)).toBeInTheDocument();
  });

  it.each([
    { fromUser: 'John', toUser: 'Jane', amount: 45.50, expectedStr: '€45.50' },
  ])('renders debt transfer card when payback is owed ($fromUser -> $toUser)', ({ fromUser, toUser, amount, expectedStr }) => {
    paybacks.set({
      rows: [],
      debts: [{ from_user: fromUser, to_user: toUser, amount: amount }],
      month: '2026-07',
    });

    render(PaybackVisual);

    expect(screen.getByText('Settlement Transfers')).toBeInTheDocument();
    expect(screen.getAllByText(fromUser).length).toBeGreaterThan(0);
    expect(screen.getAllByText(toUser).length).toBeGreaterThan(0);
    expect(screen.getByText(expectedStr)).toBeInTheDocument();
  });

  it.each([
    { month: '2026-07', amount: 45.50, expectedCents: 4550 },
  ])('executes month settlement lock flow successfully ($month)', async ({ month, amount, expectedCents }) => {
    const settleSpy = vi.spyOn(api, 'createSettlement').mockResolvedValue({});
    vi.spyOn(api, 'fetchSettlements').mockResolvedValue([]);

    paybacks.set({
      rows: [],
      debts: [{ from_user: 'John', to_user: 'Jane', amount: amount }],
      month: month,
    });

    render(PaybackVisual);

    const lockBtn = screen.getByRole('button', { name: /Mark as Settled & Lock Month/i });
    await fireEvent.click(lockBtn);

    expect(settleSpy).toHaveBeenCalledWith({
      month: month,
      net_balance_transferred_cents: expectedCents,
    });
  });

  it.each([
    { errMsg: 'Settlement failed: month already closed' },
  ])('displays error message when month settlement lock fails ($errMsg)', async ({ errMsg }) => {
    vi.spyOn(api, 'createSettlement').mockRejectedValue(new Error(errMsg));

    paybacks.set({
      rows: [],
      debts: [{ from_user: 'John', to_user: 'Jane', amount: 45.50 }],
      month: '2026-07',
    });

    render(PaybackVisual);

    const lockBtn = screen.getByRole('button', { name: /Mark as Settled & Lock Month/i });
    await fireEvent.click(lockBtn);

    expect(await screen.findByText(errMsg)).toBeInTheDocument();
  });

  it.each([
    { settledMonth: '2026-07', settledAt: '2026-08-01' },
  ])('renders locked banner when month is settled ($settledMonth)', ({ settledMonth, settledAt }) => {
    settlements.set([
      { month: settledMonth, settled_at: settledAt, net_balance_transferred_cents: 4550 },
    ]);
    paybacks.set({ rows: [], debts: [], month: settledMonth });

    render(PaybackVisual);

    expect(screen.getByText('✔️ Month Settled')).toBeInTheDocument();
    expect(screen.getByText(new RegExp(`${settledMonth} was locked on ${settledAt}`, 'i'))).toBeInTheDocument();
  });
});
