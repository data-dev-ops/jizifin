<script>
  /**
   * RealtimeChart.svelte
   *
   * Renders a Chart.js line chart on a <canvas> element.
   * - Rebuilds itself reactively whenever the `expenses` store changes or
   *   `selectedMonth` changes, filtering to only show the selected month.
   * - Also opens a WebSocket to /ws/finance and pushes `expense_created`
   *   ticks directly onto the chart instance when they belong to the current
   *   selected month.
   * - On deletion the store update triggers a full reactive rebuild, keeping
   *   the chart in sync automatically.
   */

  import { onMount, onDestroy } from 'svelte';
  import Chart from 'chart.js/auto';
  import { expenses, selectedMonth } from './stores.js';

  const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const WS_URL = `${wsProtocol}//${window.location.hostname}:8000/ws/finance`;

  let canvas;
  let chart;
  let ws;
  let wsStatus = 'connecting'; // 'connecting' | 'open' | 'closed'

  // ── Gradient fill factory (called after canvas is mounted) ────────────────
  function makeGradient(ctx) {
    const gradient = ctx.createLinearGradient(0, 0, 0, 300);
    gradient.addColorStop(0, 'rgba(99, 102, 241, 0.25)');
    gradient.addColorStop(1, 'rgba(99, 102, 241, 0.00)');
    return gradient;
  }

  // ── Rebuild chart from expenses store filtered by selectedMonth ────────────
  function rebuildChart(allExpenses, month) {
    if (!chart) return;
    const filtered = allExpenses
      .filter((r) => r.expense_date && r.expense_date.startsWith(month))
      .sort((a, b) => a.expense_date.localeCompare(b.expense_date));

    chart.data.labels = filtered.map((r) => r.expense_date);
    chart.data.datasets[0].data = filtered.map((r) => r.cost_cents / 100);
    chart.update('none'); // no animation on bulk rebuild
  }

  // React to store changes (add, delete, month change)
  $: if (chart) {
    rebuildChart($expenses, $selectedMonth);
  }

  // ── WebSocket connection with auto-reconnect ──────────────────────────────
  function connect() {
    wsStatus = 'connecting';
    ws = new WebSocket(WS_URL);

    ws.onopen = () => {
      wsStatus = 'open';
    };

    ws.onmessage = (event) => {
      let msg;
      try { msg = JSON.parse(event.data); } catch { return; }

      if (msg.event === 'expense_created' && chart) {
        const expense = msg.payload;
        // Only animate onto the chart when it belongs to the selected month
        if (expense.expense_date && expense.expense_date.startsWith($selectedMonth)) {
          chart.data.labels.push(expense.expense_date);
          chart.data.datasets[0].data.push(expense.cost_cents / 100);
          chart.update(); // animated tick
        }
      }
    };

    ws.onclose = () => {
      wsStatus = 'closed';
      // Auto-reconnect after 4 s so the live indicator recovers automatically
      setTimeout(connect, 4000);
    };

    ws.onerror = () => ws.close();
  }

  onMount(() => {
    const ctx = canvas.getContext('2d');
    const gradient = makeGradient(ctx);

    chart = new Chart(ctx, {
      type: 'line',
      data: {
        labels: [],
        datasets: [
          {
            label:                'Amount (€)',
            data:                 [],
            borderColor:          '#6366f1',
            backgroundColor:      gradient,
            fill:                 true,
            tension:              0.45,
            pointBackgroundColor: '#6366f1',
            pointBorderColor:     '#030712',
            pointBorderWidth:     2,
            pointRadius:          4,
            pointHoverRadius:     7,
          },
        ],
      },
      options: {
        responsive:          true,
        maintainAspectRatio: false,
        animation:           { duration: 500, easing: 'easeInOutQuart' },
        interaction:         { mode: 'index', intersect: false },
        plugins: {
          legend: {
            labels: {
              color:     '#9ca3af',
              font:      { family: 'Inter, system-ui, sans-serif', size: 12 },
              boxWidth:  10,
              boxHeight: 10,
            },
          },
          tooltip: {
            backgroundColor: 'rgba(15, 15, 25, 0.9)',
            borderColor:     'rgba(99, 102, 241, 0.4)',
            borderWidth:     1,
            titleColor:      '#e5e7eb',
            bodyColor:       '#9ca3af',
            padding:         10,
            callbacks: {
              label: (ctx) => ` €${Number(ctx.raw).toFixed(2)}`,
            },
          },
        },
        scales: {
          x: {
            ticks: {
              color:         '#6b7280',
              maxTicksLimit: 12,
              font:          { size: 11 },
            },
            grid:   { color: 'rgba(255,255,255,0.04)' },
            border: { color: 'rgba(255,255,255,0.08)' },
          },
          y: {
            ticks: {
              color:    '#6b7280',
              font:     { size: 11 },
              callback: (v) => `€${v}`,
            },
            grid:   { color: 'rgba(255,255,255,0.04)' },
            border: { color: 'rgba(255,255,255,0.08)' },
          },
        },
      },
    });

    // Seed from the store (already loaded by fetchAllData in App.svelte)
    rebuildChart($expenses, $selectedMonth);
    connect();
  });

  onDestroy(() => {
    if (ws) ws.close();
    if (chart) chart.destroy();
  });
</script>

<div class="relative w-full h-72 lg:h-96">
  <canvas bind:this={canvas} id="realtime-expense-chart"></canvas>
</div>
