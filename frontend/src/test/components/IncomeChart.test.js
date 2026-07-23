import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import IncomeChart from '../../lib/IncomeChart.svelte';
import { incomeAnalytics, users } from '../../lib/stores.js';

describe('IncomeChart.svelte — Per-Person Income Bar Chart', () => {
  beforeEach(() => {
    users.set([
      { name: 'Jim', color: '#6366f1' },
      { name: 'Zina', color: '#ec4899' },
    ]);
  });

  it('renders mini-cards for recorded and carried-forward income', () => {
    incomeAnalytics.set([
      { who: 'Jim', total_cents: 350000, is_carried: false },
      { who: 'Zina', total_cents: 320000, is_carried: true },
    ]);

    render(IncomeChart);

    expect(screen.getByText('Jim')).toBeInTheDocument();
    expect(screen.getByText('€3500.00')).toBeInTheDocument();
    expect(screen.getByText('recorded this month')).toBeInTheDocument();

    expect(screen.getByText('Zina')).toBeInTheDocument();
    expect(screen.getByText('€3200.00')).toBeInTheDocument();
    expect(screen.getByText('↩ carried from last salary')).toBeInTheDocument();
  });

  it('renders empty state when no income analytics are available', () => {
    incomeAnalytics.set([]);

    render(IncomeChart);

    expect(screen.getAllByText('No income data yet.').length).toBeGreaterThan(0);
  });
});
