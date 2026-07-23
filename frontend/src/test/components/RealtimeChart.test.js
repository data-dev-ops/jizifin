import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render } from '@testing-library/svelte';
import RealtimeChart from '../../lib/RealtimeChart.svelte';
import { expenses, selectedMonth } from '../../lib/stores.js';

class MockWebSocket {
  constructor(url) {
    this.url = url;
    setTimeout(() => this.onopen?.(), 0);
  }
  send() {}
  close() {
    this.onclose?.();
  }
}

describe('RealtimeChart.svelte — Realtime WebSocket Chart', () => {
  beforeEach(() => {
    vi.stubGlobal('WebSocket', MockWebSocket);
    selectedMonth.set('2026-07');
    expenses.set([
      { id: 1, expense_date: '2026-07-01', cost_cents: 1000 },
      { id: 2, expense_date: '2026-07-02', cost_cents: 2000 },
    ]);
  });

  it('renders canvas element and initializes chart', () => {
    const { container } = render(RealtimeChart);
    const canvas = container.querySelector('canvas#realtime-expense-chart');
    expect(canvas).not.toBeNull();
  });
});
