<script>
  /**
   * IncomeChart.svelte — effective income per member for the selected month (Auditor).
   */
  import { onMount, onDestroy } from 'svelte';
  import Chart from 'chart.js/auto';
  import { audit } from './api.js';
  import { money } from './format.js';
  import { chartColors, hexToRgba } from './chartTheme.js';
  import { selectedMonth, ledgerVersion, memberById, theme } from './stores.js';

  let canvas;
  let chart = null;
  let rows = [];
  let error = null;

  $: load($selectedMonth, $ledgerVersion);
  $: if (chart) render(rows, $theme);
  $: hasData = rows.some((r) => r.total !== 0);

  async function load(month) {
    try {
      rows = Object.values((await audit.income(month)).value);
      error = null;
    } catch (err) {
      error = err.message;
    }
  }

  function render(list) {
    const c = chartColors();
    const color = (r) => $memberById.get(r.member)?.color ?? '#6366f1';
    chart.data.labels = list.map((r) => $memberById.get(r.member)?.name ?? `#${r.member}`);
    chart.data.datasets[0].data = list.map((r) => r.total / 100);
    chart.data.datasets[0].backgroundColor = list.map((r) => hexToRgba(color(r), 0.75));
    chart.data.datasets[0].borderColor = list.map(color);
    chart.options.scales.x.ticks.color = c.tick;
    chart.options.scales.x.grid.color = c.grid;
    chart.options.scales.y.ticks.color = c.label;
    Object.assign(chart.options.plugins.tooltip, c.tooltip);
    chart.update();
  }

  onMount(() => {
    const c = chartColors();
    chart = new Chart(canvas.getContext('2d'), {
      type: 'bar',
      data: { labels: [], datasets: [{ data: [], borderWidth: 1.5, borderRadius: 6 }] },
      options: {
        indexAxis: 'y',
        responsive: true,
        maintainAspectRatio: false,
        scales: { x: { grid: { color: c.grid }, ticks: { color: c.tick } }, y: { grid: { display: false }, ticks: { color: c.label } } },
        plugins: { legend: { display: false }, tooltip: { ...c.tooltip, callbacks: { label: (ctx) => ` ${$money(Math.round(ctx.raw * 100))}` } } },
      },
    });
    render(rows);
  });

  onDestroy(() => chart?.destroy());
</script>

{#if error}<p class="text-xs text-rose-600 dark:text-red-400">{error}</p>{/if}
<div class="relative h-48" class:hidden={!hasData}>
  <canvas bind:this={canvas} id="income-chart"></canvas>
</div>
{#if !hasData}
  <div class="empty-state-box py-6"><p class="text-xs text-neutral-500">No income recorded for this month.</p></div>
{/if}
