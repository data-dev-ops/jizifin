import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import IncomeChart from '../../lib/IncomeChart.svelte';
import { incomeAnalytics, users } from '../../lib/stores.js';

describe('IncomeChart.svelte — Per-Person Income Bar Chart', () => {
  beforeEach(() => {
    users.set([
      { name: 'John', color: '#6366f1' },
      { name: 'Jane', color: '#ec4899' },
    ]);
  });

  it.each([
    {
      data: [
        { who: 'John', total_cents: 350000, is_carried: false },
        { who: 'Jane', total_cents: 320000, is_carried: true },
      ],
    },
  ])('renders mini-cards for recorded and carried-forward income', ({ data }) => {
    incomeAnalytics.set(data);

    render(IncomeChart);

    expect(screen.getByText('John')).toBeInTheDocument();
    expect(screen.getByText('€3500.00')).toBeInTheDocument();
    expect(screen.getByText('recorded this month')).toBeInTheDocument();

    expect(screen.getByText('Jane')).toBeInTheDocument();
    expect(screen.getByText('€3200.00')).toBeInTheDocument();
    expect(screen.getByText('↩ carried from last salary')).toBeInTheDocument();
  });

  it.each([
    { emptyData: [] },
  ])('renders empty state when no income analytics are available', ({ emptyData }) => {
    incomeAnalytics.set(emptyData);

    render(IncomeChart);

    expect(screen.getAllByText('No income data yet.').length).toBeGreaterThan(0);
  });
});
