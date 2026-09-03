import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import DrilldownModal from '../../lib/DrilldownModal.svelte';
import {
  drilldownTarget,
  expenses,
  users,
  currencySymbol,
  currencyPrecisionMode,
  selectedMonth,
  paybacks,
  dashboardScope,
  openDrilldown,
  closeDrilldown,
} from '../../lib/stores.js';

describe('DrilldownModal.svelte — Interactive Dashboard Info Cards Drill-Down', () => {
  beforeEach(() => {
    currencySymbol.set('€');
    currencyPrecisionMode.set('always_exact');
    selectedMonth.set('2026-07');
    dashboardScope.set('ALL');
    users.set([
      { name: 'John', color: '#6366f1', is_active: true },
      { name: 'Jane', color: '#ec4899', is_active: true },
    ]);
    expenses.set([
      {
        id: 1,
        name: 'Weekly Supermarket',
        cost_cents: 8000,
        expense_date: '2026-07-05',
        who_paid: 'John',
        category: 'GROCERIES',
      },
      {
        id: 2,
        name: 'Farmers Market Fruit',
        cost_cents: 2000,
        expense_date: '2026-07-12',
        who_paid: 'Jane',
        category: 'GROCERIES',
      },
      {
        id: 3,
        name: 'Electric Bill',
        cost_cents: 5000,
        expense_date: '2026-07-15',
        who_paid: 'Jane',
        category: 'UTILITIES',
      },
    ]);
    closeDrilldown();
  });

  it('renders nothing when drilldownTarget is null', () => {
    render(DrilldownModal);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders category drill-down with expenses per user and itemized transactions', async () => {
    render(DrilldownModal);

    openDrilldown('category', 'GROCERIES');

    // Dialog heading and category title
    expect(await screen.findByRole('dialog')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'GROCERIES' })).toBeInTheDocument();

    // Total category spent (€80.00 + €20.00 = €100.00)
    expect(screen.getByText('€100.00')).toBeInTheDocument();

    // Expenses per user breakdown
    expect(screen.getByText('Expenses Per User')).toBeInTheDocument();
    expect(screen.getAllByText('John').length).toBeGreaterThan(0);
    expect(screen.getByText('80% of cost')).toBeInTheDocument();
    expect(screen.getAllByText('€80.00').length).toBeGreaterThan(0);

    expect(screen.getAllByText('Jane').length).toBeGreaterThan(0);
    expect(screen.getByText('20% of cost')).toBeInTheDocument();
    expect(screen.getAllByText('€20.00').length).toBeGreaterThan(0);

    // Itemized expenses
    expect(screen.getByText('Weekly Supermarket')).toBeInTheDocument();
    expect(screen.getByText('Farmers Market Fruit')).toBeInTheDocument();
    // Utilities expense should NOT be present in Groceries drill-down
    expect(screen.queryByText('Electric Bill')).not.toBeInTheDocument();
  });

  it('filters itemized expenses in real-time when searching', async () => {
    render(DrilldownModal);
    openDrilldown('category', 'GROCERIES');

    expect(await screen.findByText('Weekly Supermarket')).toBeInTheDocument();
    expect(screen.getByText('Farmers Market Fruit')).toBeInTheDocument();

    const searchInput = screen.getByPlaceholderText('Filter expenses…');
    await fireEvent.input(searchInput, { target: { value: 'Farmers' } });

    expect(screen.getByText('Farmers Market Fruit')).toBeInTheDocument();
    expect(screen.queryByText('Weekly Supermarket')).not.toBeInTheDocument();
  });

  it('renders category-adjustment drill-down with settlement net amounts and underlying expenses', async () => {
    paybacks.set({
      rows: [
        {
          category: 'GROCERIES',
          total_amount: 100.00,
          per_user_paid: { John: 80.00, Jane: 20.00 },
          per_user_share_pct: { John: 50.0, Jane: 50.0 },
          net_per_user: { John: 30.00, Jane: -30.00 },
        },
      ],
      debts: [],
      month: '2026-07',
    });

    render(DrilldownModal);
    openDrilldown('category-adjustment', 'GROCERIES');

    expect(await screen.findByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText('Settlement Allocation & Net Balance')).toBeInTheDocument();
    expect(screen.getByText('+€30.00 owed back')).toBeInTheDocument();
    expect(screen.getByText('-€30.00 owes')).toBeInTheDocument();

    // Itemized contributing expenses are shown
    expect(screen.getByText('Weekly Supermarket')).toBeInTheDocument();
    expect(screen.getByText('Farmers Market Fruit')).toBeInTheDocument();
  });

  it('renders payer drilldown with spending by category and itemized expenses', async () => {
    render(DrilldownModal);
    openDrilldown('payer', 'Jane');

    expect(await screen.findByRole('dialog')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Jane' })).toBeInTheDocument();

    // Total paid by Jane: €20 (Groceries) + €50 (Utilities) = €70.00
    expect(screen.getByText('€70.00')).toBeInTheDocument();
    expect(screen.getByText('Spending by Category')).toBeInTheDocument();
    expect(screen.getByText('Farmers Market Fruit')).toBeInTheDocument();
    expect(screen.getByText('Electric Bill')).toBeInTheDocument();
    expect(screen.queryByText('Weekly Supermarket')).not.toBeInTheDocument();
  });

  it('closes drill-down modal on Done button click', async () => {
    render(DrilldownModal);
    openDrilldown('category', 'GROCERIES');

    expect(await screen.findByRole('dialog')).toBeInTheDocument();

    const doneButton = screen.getByRole('button', { name: /Done/i });
    await fireEvent.click(doneButton);

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('closes drill-down modal on Escape key press', async () => {
    render(DrilldownModal);
    openDrilldown('category', 'GROCERIES');

    expect(await screen.findByRole('dialog')).toBeInTheDocument();

    await fireEvent.keyDown(window, { key: 'Escape' });

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('filters category drilldown when a user is selected in dashboardScope', async () => {
    dashboardScope.set('USER:John');

    render(DrilldownModal);
    openDrilldown('category', 'GROCERIES');

    expect(await screen.findByRole('dialog')).toBeInTheDocument();

    // Scope pill should be displayed
    expect(screen.getByText('Scope:')).toBeInTheDocument();

    // Total category spent should only be John's spend (€80.00), NOT €100.00
    expect(screen.getAllByText('€80.00').length).toBeGreaterThan(0);
    expect(screen.queryByText('€100.00')).not.toBeInTheDocument();

    // Only John's expenses should appear in itemized list
    expect(screen.getByText('Weekly Supermarket')).toBeInTheDocument();
    // Jane's grocery expense should NOT be present
    expect(screen.queryByText('Farmers Market Fruit')).not.toBeInTheDocument();
  });

  it('updates drilldown reactively when dashboardScope changes while open', async () => {
    dashboardScope.set('ALL');

    render(DrilldownModal);
    openDrilldown('category', 'GROCERIES');

    expect(await screen.findByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText('Farmers Market Fruit')).toBeInTheDocument();
    expect(screen.getByText('Weekly Supermarket')).toBeInTheDocument();

    // Now switch scope to John
    dashboardScope.set('USER:John');

    // Jane's expense disappears from drilldown
    await waitFor(() => {
      expect(screen.queryByText('Farmers Market Fruit')).not.toBeInTheDocument();
    });
    expect(screen.getByText('Weekly Supermarket')).toBeInTheDocument();
  });
});
