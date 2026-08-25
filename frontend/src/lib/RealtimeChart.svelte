<script>
  /**
   * RealtimeChart.svelte
   *
   * Renders a Chart.js line chart on a <canvas> element with full reactive theming.
   * - Rebuilds itself reactively whenever the `expenses` store changes,
   *   `selectedMonth` changes, or `theme` changes.
   * - Also opens a WebSocket to /ws/finance and pushes `expense_created`
   *   ticks directly onto the chart instance.
   */

  import { onMount, onDestroy } from 'svelte';
  import Chart from 'chart.js/auto';
  import { expenses, selectedMonth, currencySymbol, sessionToken, theme } from './stores.js';

  const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const WS_URL = `${wsProtocol}//${window.location.host}/api/ws/finance`;

  let canvas;
  let chart;
  let ws;
  let wsStatus = 'connecting'; // 'connecting' | 'open' | 'closed'

  function getIsDark() {
    if (typeof document === 'undefined') return true;
    return document.documentElement.classList.contains('dark');
  }

  // ── Gradient fill factory (called after canvas is mounted) ────────────────
  function makeGradient(ctx, isDark) {
    const gradient = ctx.createLinearGradient(0, 0, 0, 300);
    if (isDark) {
      gradient.addColorStop(0, 'rgba(99, 102, 241, 0.35)');
      gradient.addColorStop(1, 'rgba(99, 102, 241, 0.00)');
    } else {
      gradient.addColorStop(0, 'rgba(99, 102, 241, 0.25)');
      gradient.addColorStop(1, 'rgba(99, 102, 241, 0.00)');
    }
    return gradient;
  }

  // ── Apply theme colors to chart instance ──────────────────────────────────
  function applyChartTheme() {
    if (!chart || !canvas) return;
    const isDark = getIsDark();
    const ctx = canvas.getContext('2d');
    const gradient = makeGradient(ctx, isDark);

    chart.data.datasets[0].backgroundColor = gradient;
    chart.data.datasets[0].pointBorderColor = isDark ? '#080c14' : '#ffffff';
    
    // Legend & tooltips
    if (chart.options.plugins?.legend?.labels) {
      chart.options.plugins.legend.labels.color = isDark ? '#9ca3af' : '#64748b';
    }
    if (chart.options.plugins?.tooltip) {
      chart.options.plugins.tooltip.backgroundColor = isDark ? 'rgba(15, 23, 42, 0.95)' : 'rgba(255, 255, 255, 0.95)';
      chart.options.plugins.tooltip.titleColor = isDark ? '#f1f5f9' : '#0f172a';
      chart.options.plugins.tooltip.bodyColor = isDark ? '#cbd5e1' : '#475569';
      chart.options.plugins.tooltip.borderColor = isDark ? 'rgba(99, 102, 241, 0.4)' : 'rgba(99, 102, 241, 0.25)';
    }

    // Scales
    if (chart.options.scales?.x) {
      chart.options.scales.x.ticks.color = isDark ? '#9ca3af' : '#64748b';
      chart.options.scales.x.grid.color = isDark ? 'rgba(255, 255, 255, 0.05)' : 'rgba(0, 0, 0, 0.05)';
      chart.options.scales.x.border.color = isDark ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)';
    }
    if (chart.options.scales?.y) {
      chart.options.scales.y.ticks.color = isDark ? '#9ca3af' : '#64748b';
      chart.options.scales.y.grid.color = isDark ? 'rgba(255, 255, 255, 0.05)' : 'rgba(0, 0, 0, 0.05)';
      chart.options.scales.y.border.color = isDark ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)';
    }

    chart.update('none');
  }

  // ── Rebuild chart from expenses store filtered by selectedMonth ────────────
  function rebuildChart(allExpenses, month) {
    if (!chart) return;
    const filtered = allExpenses
      .filter((r) => r.expense_date && r.expense_date.startsWith(month))
      .sort((a, b) => a.expense_date.localeCompare(b.expense_date));

    chart.data.labels = filtered.map((r) => r.expense_date);
    chart.data.datasets[0].data = filtered.map((r) => r.cost_cents / 100);
    chart.update('none');
  }

  // React to store changes (add, delete, month change)
  $: if (chart) {
    rebuildChart($expenses, $selectedMonth);
  }

  // React to theme store changes
  $: if (chart && $theme) {
    applyChartTheme();
  }

  // ── WebSocket connection with auto-reconnect ──────────────────────────────
  function connect() {
    wsStatus = 'connecting';
    const tokenParam = $sessionToken ? `?token=${encodeURIComponent($sessionToken)}` : '';
    ws = new WebSocket(`${WS_URL}${tokenParam}`);

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
    const isDark = getIsDark();
    const ctx = canvas.getContext('2d');
    const gradient = makeGradient(ctx, isDark);

    chart = new Chart(ctx, {
      type: 'line',
      data: {
        labels: [],
        datasets: [
          {
            label:                `Amount (${$currencySymbol})`,
            data:                 [],
            borderColor:          '#6366f1',
            backgroundColor:      gradient,
            fill:                 true,
            tension:              0.45,
            pointBackgroundColor: '#6366f1',
            pointBorderColor:     isDark ? '#080c14' : '#ffffff',
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
              color:     isDark ? '#9ca3af' : '#64748b',
              font:      { family: 'Plus Jakarta Sans, Inter, system-ui, sans-serif', size: 12 },
              boxWidth:  10,
              boxHeight: 10,
            },
          },
          tooltip: {
            backgroundColor: isDark ? 'rgba(15, 23, 42, 0.95)' : 'rgba(255, 255, 255, 0.95)',
            borderColor:     isDark ? 'rgba(99, 102, 241, 0.4)' : 'rgba(99, 102, 241, 0.25)',
            borderWidth:     1,
            titleColor:      isDark ? '#f1f5f9' : '#0f172a',
            bodyColor:       isDark ? '#cbd5e1' : '#475569',
            padding:         10,
            callbacks: {
              label: (ctx) => ` ${$currencySymbol}${Number(ctx.raw).toFixed(2)}`,
            },
          },
        },
        scales: {
          x: {
            ticks: {
              color:         isDark ? '#9ca3af' : '#64748b',
              maxTicksLimit: 12,
              font:          { size: 11 },
            },
            grid:   { color: isDark ? 'rgba(255,255,255,0.05)' : 'rgba(0,0,0,0.05)' },
            border: { color: isDark ? 'rgba(255,255,255,0.1)' : 'rgba(0,0,0,0.1)' },
          },
          y: {
            ticks: {
              color:    isDark ? '#9ca3af' : '#64748b',
              font:     { size: 11 },
              callback: (v) => `${$currencySymbol}${v}`,
            },
            grid:   { color: isDark ? 'rgba(255,255,255,0.05)' : 'rgba(0,0,0,0.05)' },
            border: { color: isDark ? 'rgba(255,255,255,0.1)' : 'rgba(0,0,0,0.1)' },
          },
        },
      },
    });

    // Seed from the store
    rebuildChart($expenses, $selectedMonth);
    applyChartTheme();
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
