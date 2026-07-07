<script>
  /**
   * AnalyticsSummary.svelte
   *
   * Displays three summary cards (monthly total, Jim's spend, Zina's spend)
   * and a doughnut chart breaking costs down by category.
   * Subscribes to the analytics store — refreshes automatically after any
   * fetchAnalytics() call (which ExpenseForm triggers on each submission).
   */

  import { onMount, onDestroy } from 'svelte';
  import { analytics, users } from './stores.js';
  import Chart from 'chart.js/auto';

  // ── Doughnut chart state & refs ───────────────────────────────────────────
  let doughnutCanvas;
  let doughnutChart = null;

  // ── Derived display values ────────────────────────────────────────────────
  let total       = 0;
  let payerRows   = [];
  let categories  = [];

  /** Look up a user's colour from the users store, with a sensible fallback. */
  function userColor(name) {
    return $users.find((u) => u.name === name)?.color ?? '#6366f1';
  }

  const unsubscribe = analytics.subscribe((v) => {
    total      = v.monthly_total?.total_amount ?? 0;
    payerRows  = v.by_payer;  // [{ who_paid, total_amount, expense_count }]
    categories = v.by_category;
    updateDoughnut();
  });

  // ── Doughnut chart config ──────────────────────────────────────────────────
  const PALETTE = [
    'rgba(99,  102, 241, 0.85)', // indigo
    'rgba(139,  92, 246, 0.85)', // violet
    'rgba( 14, 165, 233, 0.85)', // sky
    'rgba(236,  72, 153, 0.85)', // pink
    'rgba( 16, 185, 129, 0.85)', // emerald
    'rgba(251, 191,  36, 0.85)', // amber
  ];

  function updateDoughnut() {
    if (!doughnutChart) return;
    doughnutChart.data.labels             = categories.map((c) => c.category);
    doughnutChart.data.datasets[0].data   = categories.map((c) => c.total_amount);
    doughnutChart.update();
  }

  onMount(() => {
    if (!doughnutCanvas) return;
    const ctx = doughnutCanvas.getContext('2d');
    doughnutChart = new Chart(ctx, {
      type: 'doughnut',
      data: {
        labels:   categories.map((c) => c.category),
        datasets: [
          {
            data:            categories.map((c) => c.total_amount),
            backgroundColor: PALETTE,
            borderColor:     '#0a0a14',
            borderWidth:     3,
            hoverOffset:     6,
          },
        ],
      },
      options: {
        responsive:          true,
        maintainAspectRatio: false,
        cutout:              '70%',
        animation:           { duration: 600, easing: 'easeInOutQuart' },
        plugins: {
          legend: {
            position: 'bottom',
            labels: {
              color:     '#9ca3af',
              font:      { family: 'Inter, system-ui, sans-serif', size: 11 },
              boxWidth:  10,
              boxHeight: 10,
              padding:   12,
            },
          },
          tooltip: {
            backgroundColor: 'rgba(15,15,25,0.9)',
            borderColor:     'rgba(99,102,241,0.4)',
            borderWidth:     1,
            titleColor:      '#e5e7eb',
            bodyColor:       '#9ca3af',
            callbacks: {
              label: (ctx) => ` €${Number(ctx.raw).toFixed(2)}`,
            },
          },
        },
      },
    });

    // Trigger an initial draw if data is already loaded
    updateDoughnut();
  });

  onDestroy(() => {
    unsubscribe();
    if (doughnutChart) doughnutChart.destroy();
  });

  function fmt(n) {
    return `€${Number(n).toFixed(2)}`;
  }

  function pct(part, whole) {
    if (!whole || whole === 0) return '—';
    return `${((part / whole) * 100).toFixed(0)}%`;
  }
</script>

<!-- ── Summary cards ──────────────────────────────────────────────────────── -->
<div class="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-6">

  <!-- Monthly total -->
  <div class="bg-neutral-900 rounded-2xl border border-neutral-800 px-5 py-4 min-w-0">
    <p class="text-xs font-medium text-neutral-500 uppercase tracking-wider mb-2">Monthly Total</p>
    <p class="font-bold text-white tabular-nums truncate" style="font-size: clamp(1.25rem, 4vw, 1.875rem)">{fmt(total)}</p>
    <p class="text-xs text-neutral-600 mt-1">This calendar month</p>
  </div>

  <!-- Dynamic per-payer cards — one card per user who has paid in the selected month -->
  {#each payerRows as row}
    {@const color = userColor(row.who_paid)}
    <div class="bg-neutral-900 rounded-2xl border px-5 py-4 min-w-0" style="border-color:{color}40">
      <p class="text-xs font-medium uppercase tracking-wider mb-2" style="color:{color}">{row.who_paid}</p>
      <p class="font-bold tabular-nums truncate" style="font-size: clamp(1.25rem, 4vw, 1.875rem); color:{color}">{fmt(row.total_amount)}</p>
      <p class="text-xs text-neutral-600 mt-1">{pct(row.total_amount, total)} of total spend</p>
    </div>
  {/each}

</div>

<!-- ── Category doughnut ──────────────────────────────────────────────────── -->
<div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-6">
  <h2 class="text-sm font-semibold text-neutral-300 mb-4">Spend by Category</h2>

  <div class="relative h-52" class:hidden={categories.length === 0}>
    <canvas bind:this={doughnutCanvas} id="category-doughnut-chart"></canvas>
  </div>

  {#if categories.length === 0}
    <div class="flex flex-col items-center justify-center py-10 text-center">
      <p class="text-neutral-500 text-sm">No category data yet for this month.</p>
    </div>
  {:else}
    <!-- Category breakdown list -->
    <div class="mt-4 space-y-2">
      {#each categories as row, i}
        <div class="flex items-center justify-between gap-2 text-sm min-w-0">
          <div class="flex items-center gap-2 min-w-0 overflow-hidden">
            <span
              class="w-2.5 h-2.5 rounded-full flex-none"
              style="background:{PALETTE[i % PALETTE.length]}"
            ></span>
            <span class="text-neutral-300 truncate">{row.category}</span>
            <span class="text-neutral-600 text-xs flex-none">({row.expense_count})</span>
          </div>
          <span class="text-neutral-200 font-semibold tabular-nums flex-none">{fmt(row.total_amount)}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>
