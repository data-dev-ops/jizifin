<script>
  /**
   * RealtimeChart.svelte — twelve months of spend per category (stacked bars) from
   * `/views/series`, refetched on every ledger event.
   */
  import { onMount, onDestroy } from 'svelte';
  import Chart from 'chart.js/auto';
  import { views } from './api.js';
  import { money, fmtMonth } from './format.js';
  import { chartColors, PALETTE } from './chartTheme.js';
  import { selectedMonth, ledgerVersion, categoryName, theme } from './stores.js';

  let canvas;
  let chart = null;
  let points = [];
  let error = null;

  $: load($selectedMonth, $ledgerVersion);
  $: if (chart) render(points, $theme);

  async function load(month) {
    try {
      points = await views.series({ to: month });
      error = null;
    } catch (err) {
      error = err.message;
    }
  }

  /** Pivot points into one dataset per category (presentation only). */
  function datasets(rows) {
    const months = [...new Set(rows.map((p) => p.month))];
    const categories = [...new Set(rows.map((p) => p.category))];
    const value = (m, c) => (rows.find((p) => p.month === m && p.category === c)?.total ?? 0) / 100;
    return {
      labels: months.map((m) => fmtMonth(m)),
      sets: categories.map((c, i) => ({ label: $categoryName(c), data: months.map((m) => value(m, c)), backgroundColor: PALETTE[i % PALETTE.length], borderRadius: 4 })),
    };
  }

  function render(rows) {
    const { labels, sets } = datasets(rows);
    const c = chartColors();
    chart.data.labels = labels;
    chart.data.datasets = sets;
    chart.options.scales.x.ticks.color = c.tick;
    chart.options.scales.y.ticks.color = c.tick;
    chart.options.scales.y.grid.color = c.grid;
    chart.options.plugins.legend.labels.color = c.tick;
    Object.assign(chart.options.plugins.tooltip, c.tooltip);
    chart.update();
  }

  onMount(() => {
    const c = chartColors();
    chart = new Chart(canvas.getContext('2d'), {
      type: 'bar',
      data: { labels: [], datasets: [] },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        scales: {
          x: { stacked: true, grid: { display: false }, ticks: { color: c.tick } },
          y: { stacked: true, grid: { color: c.grid }, ticks: { color: c.tick } },
        },
        plugins: {
          legend: { position: 'bottom', labels: { color: c.tick, boxWidth: 10, boxHeight: 10 } },
          tooltip: { ...c.tooltip, callbacks: { label: (ctx) => ` ${ctx.dataset.label}: ${$money(Math.round(ctx.raw * 100))}` } },
        },
      },
    });
    render(points);
  });

  onDestroy(() => chart?.destroy());
</script>

{#if error}<p class="text-xs text-rose-600 dark:text-red-400">{error}</p>{/if}
<div class="relative h-64 sm:h-72">
  <canvas bind:this={canvas} id="expense-timeline-chart"></canvas>
</div>
