<script>
  /**
   * IncomeChart.svelte
   *
   * Horizontal bar chart showing total income per active user for the selected month.
   * If a user has no income recorded in that month, their most recent Salary
   * value is carried forward and displayed with a dashed indicator.
   *
   * Colors are sourced from the users store (user.color) rather than hardcoded.
   */

  import { onMount, onDestroy, tick } from 'svelte';
  import { get } from 'svelte/store';
  import Chart from 'chart.js/auto';
  import { incomeAnalytics, users, currencySymbol } from './stores.js';

  let canvas;
  let chart = null;

  $: hasData = $incomeAnalytics.length > 0;

  function fmt(cents) {
    return `${$currencySymbol}${(cents / 100).toFixed(2)}`;
  }

  /** Convert a hex colour to rgba(r,g,b,alpha). */
  function hexToRgba(hex, alpha) {
    const h = hex.replace('#', '');
    const r = parseInt(h.slice(0, 2), 16);
    const g = parseInt(h.slice(2, 4), 16);
    const b = parseInt(h.slice(4, 6), 16);
    return `rgba(${r},${g},${b},${alpha})`;
  }

  /** Look up a user's colour from the users store. */
  function userColor(name) {
    return get(users).find((u) => u.name === name)?.color ?? '#6366f1';
  }

  function rebuildChart(rows) {
    if (!chart) return;
    const labels  = rows.map((r) => r.who);
    const values  = rows.map((r) => r.total_cents / 100);
    const colours = rows.map((r) => hexToRgba(userColor(r.who), 0.75));
    const borders = rows.map((r) => userColor(r.who));
    chart.data.labels                      = labels;
    chart.data.datasets[0].data            = values;
    chart.data.datasets[0].backgroundColor = colours;
    chart.data.datasets[0].borderColor     = borders;
    chart.update();
  }

  $: if (chart) rebuildChart($incomeAnalytics);

  onMount(() => {
    const ctx = canvas.getContext('2d');
    chart = new Chart(ctx, {
      type: 'bar',
      data: {
        labels: [],
        datasets: [{
          label:           `Income (${$currencySymbol})`,
          data:            [],
          backgroundColor: [],
          borderColor:     [],
          borderWidth:     2,
          borderRadius:    8,
          borderSkipped:   false,
        }],
      },
      options: {
        responsive:          true,
        maintainAspectRatio: false,
        indexAxis:           'y',
        animation:           { duration: 500, easing: 'easeInOutQuart' },
        plugins: {
          legend: { display: false },
          tooltip: {
            backgroundColor: 'rgba(15,15,25,0.9)',
            borderColor:     'rgba(99,102,241,0.4)',
            borderWidth:     1,
            titleColor:      '#e5e7eb',
            bodyColor:       '#9ca3af',
            padding:         10,
            callbacks: { label: (ctx) => ` ${$currencySymbol}${Number(ctx.raw).toFixed(2)}` },
          },
        },
        scales: {
          x: {
            ticks: { color: '#6b7280', font: { size: 11 }, callback: (v) => `${$currencySymbol}${v}` },
            grid:  { color: 'rgba(255,255,255,0.04)' },
            border: { color: 'rgba(255,255,255,0.08)' },
            beginAtZero: true,
          },
          y: {
            ticks:  { color: '#9ca3af', font: { size: 13, weight: '600' } },
            grid:   { display: false },
            border: { color: 'rgba(255,255,255,0.08)' },
          },
        },
      },
    });
    rebuildChart($incomeAnalytics);
  });

  onDestroy(() => { if (chart) chart.destroy(); });
</script>

<!-- ── Income summary mini-cards ───────────────────────────────────────────── -->
<div class="flex flex-wrap gap-3 mb-5">
  {#each $incomeAnalytics as person (person.who)}
    {@const color = $users.find((u) => u.name === person.who)?.color ?? '#6366f1'}
    <div class="flex-1 min-w-[120px] bg-neutral-800/50 rounded-xl border px-4 py-3"
         style="border-color: {color}30">
      <p class="text-xs font-medium text-neutral-500 uppercase tracking-wider mb-1">{person.who}</p>
      <p class="text-2xl font-bold tabular-nums" style="color: {color}">{fmt(person.total_cents)}</p>
      {#if person.is_carried}
        <p class="text-[10px] text-neutral-600 mt-0.5">↩ carried from last salary</p>
      {:else}
        <p class="text-[10px] text-neutral-600 mt-0.5">recorded this month</p>
      {/if}
    </div>
  {/each}
  {#if $incomeAnalytics.length === 0}
    <p class="text-neutral-600 text-sm">No income data yet.</p>
  {/if}
</div>

<!-- ── Chart ─────────────────────────────────────────────────────────────── -->
<div class="relative h-28">
  <canvas bind:this={canvas} id="income-by-person-chart"></canvas>
  {#if !hasData}
    <div class="absolute inset-0 flex flex-col items-center justify-center bg-neutral-900/80 rounded-xl">
      <p class="text-neutral-500 text-sm">No income data yet.</p>
      <p class="text-neutral-700 text-xs mt-1">Save salaries in the Splits tab to populate this chart.</p>
    </div>
  {/if}
</div>
