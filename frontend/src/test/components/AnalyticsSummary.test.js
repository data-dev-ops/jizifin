import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import AnalyticsSummary from '../../lib/AnalyticsSummary.svelte';
import { analytics, users, currencySymbol } from '../../lib/stores.js';

describe('AnalyticsSummary.svelte — Monthly Summary & Category Breakdown', () => {
  beforeEach(() => {
    currencySymbol.set('€');
    users.set([
      { name: 'John', color: '#6366f1' },
      { name: 'Jane', color: '#ec4899' },
    ]);
  });

  it.each([
    {
      total: 150.50,
      payerData: [
        { who_paid: 'John', total_amount: 100.00, expense_count: 2 },
        { who_paid: 'Jane', total_amount: 50.50, expense_count: 1 },
      ],
      categoryData: [
        { category: 'GROCERIES', total_amount: 100.00, expense_count: 2 },
        { category: 'UTILITIES', total_amount: 50.50, expense_count: 1 },
      ],
    },
  ])('renders monthly total and payer cards correctly', async ({ total, payerData, categoryData }) => {
    analytics.set({
      monthly_total: { total_amount: total, expense_count: 3, month: '2026-07' },
      by_payer: payerData,
      by_category: categoryData,
    });

    render(AnalyticsSummary);

    expect(screen.getByText('Monthly Total')).toBeInTheDocument();
    expect(screen.getByText('€150.50')).toBeInTheDocument();

    expect(screen.getByText('John')).toBeInTheDocument();
    expect(screen.getAllByText('€100.00').length).toBeGreaterThan(0);
    expect(screen.getByText('66% of total spend')).toBeInTheDocument();

    expect(screen.getByText('Jane')).toBeInTheDocument();
    expect(screen.getAllByText('€50.50').length).toBeGreaterThan(0);
    expect(screen.getByText('34% of total spend')).toBeInTheDocument();
  });

  it.each([
    { total: 0.0 },
  ])('renders empty category state when no expenses exist ($total)', async ({ total }) => {
    analytics.set({
      monthly_total: { total_amount: total, expense_count: 0, month: '2026-07' },
      by_payer: [],
      by_category: [],
    });

    render(AnalyticsSummary);

    expect(screen.getByText('No category data yet.')).toBeInTheDocument();
    expect(screen.getByText('€0.00')).toBeInTheDocument();
  });

  it.each([
    { symbol: '$', amount: 250.00 },
    { symbol: '£', amount: 250.00 },
  ])('respects dynamic currency symbol preference ($symbol)', async ({ symbol, amount }) => {
    currencySymbol.set(symbol);
    analytics.set({
      monthly_total: { total_amount: amount, expense_count: 1, month: '2026-07' },
      by_payer: [{ who_paid: 'John', total_amount: amount, expense_count: 1 }],
      by_category: [{ category: 'RENT', total_amount: amount, expense_count: 1 }],
    });

    render(AnalyticsSummary);

    expect(screen.getAllByText(`${symbol}250.00`).length).toBeGreaterThan(0);
  });
});
