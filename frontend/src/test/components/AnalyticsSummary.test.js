import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import AnalyticsSummary from '../../lib/AnalyticsSummary.svelte';
import { analytics, users, currencySymbol } from '../../lib/stores.js';

describe('AnalyticsSummary.svelte — Monthly Summary & Category Breakdown', () => {
  beforeEach(() => {
    currencySymbol.set('€');
    users.set([
      { name: 'Jim', color: '#6366f1' },
      { name: 'Zina', color: '#ec4899' },
    ]);
  });

  it('renders monthly total and payer cards correctly', async () => {
    analytics.set({
      monthly_total: { total_amount: 150.50, expense_count: 3, month: '2026-07' },
      by_payer: [
        { who_paid: 'Jim', total_amount: 100.00, expense_count: 2 },
        { who_paid: 'Zina', total_amount: 50.50, expense_count: 1 },
      ],
      by_category: [
        { category: 'GROCERIES', total_amount: 100.00, expense_count: 2 },
        { category: 'UTILITIES', total_amount: 50.50, expense_count: 1 },
      ],
    });

    render(AnalyticsSummary);

    expect(screen.getByText('Monthly Total')).toBeInTheDocument();
    expect(screen.getByText('€150.50')).toBeInTheDocument();

    expect(screen.getByText('Jim')).toBeInTheDocument();
    expect(screen.getAllByText('€100.00').length).toBeGreaterThan(0);
    expect(screen.getByText('66% of total spend')).toBeInTheDocument();

    expect(screen.getByText('Zina')).toBeInTheDocument();
    expect(screen.getAllByText('€50.50').length).toBeGreaterThan(0);
    expect(screen.getByText('34% of total spend')).toBeInTheDocument();
  });

  it('renders empty category state when no expenses exist', async () => {
    analytics.set({
      monthly_total: { total_amount: 0.0, expense_count: 0, month: '2026-07' },
      by_payer: [],
      by_category: [],
    });

    render(AnalyticsSummary);

    expect(screen.getByText('No category data yet.')).toBeInTheDocument();
    expect(screen.getByText('€0.00')).toBeInTheDocument();
  });

  it('respects dynamic currency symbol preference', async () => {
    currencySymbol.set('$');
    analytics.set({
      monthly_total: { total_amount: 250.00, expense_count: 1, month: '2026-07' },
      by_payer: [{ who_paid: 'Jim', total_amount: 250.00, expense_count: 1 }],
      by_category: [{ category: 'RENT', total_amount: 250.00, expense_count: 1 }],
    });

    render(AnalyticsSummary);

    expect(screen.getAllByText('$250.00').length).toBeGreaterThan(0);
  });
});
