<script>
  /**
   * AnalyticsSummary.svelte — household command center. Renders server values only:
   * `/views/month` (spend by category/member with shares), Auditor cash flow, income,
   * budget status and joint dashboard, `/views/projects` progress.
   */

  import { onMount, onDestroy, tick } from 'svelte';
  import Chart from 'chart.js/auto';
  import PaybackVisual from './PaybackVisual.svelte';
  import RealtimeChart from './RealtimeChart.svelte';
  import IncomeChart from './IncomeChart.svelte';
  import DrilldownModal from './DrilldownModal.svelte';
  import { views, audit } from './api.js';
  import { money, fmtPct } from './format.js';
  import { chartColors, isDark as isDarkTheme, PALETTE } from './chartTheme.js';
  import {
    activeMembers, accounts, memberById, categoryName, projectById, tagById, chartStyle,
    jointAccountEnabled, dashboardScope, parseDashboardScope, scopedMembers, selectedMonth,
    ledgerVersion, tags, theme, privacyShield, dashboardWidgets, openDrilldown, activeAccountId,
  } from './stores.js';

  let activeSubtab = 'pulse';

  function switchSubtab(subtab) {
    activeSubtab = subtab;
    if (subtab === 'pulse' || subtab === 'complete') tick().then(() => createChart(currentChartType));
  }

  // ── Server data ────────────────────────────────────────────────────────────
  let month = { total: 0, count: 0, by_category: [], by_member: [] };
  let cash = null;
  let income = {};
  let budgetStatus = [];
  let tagTotals = [];
  let progress = [];
  let dash = null;
  let loadError = null;

  $: account = $accounts.find((a) => a.id === ($activeAccountId ?? $accounts[0]?.id));
  $: load($selectedMonth, $scopedMembers ?? [], account?.id, $ledgerVersion);

  async function load(m, members, accountId) {
    loadError = null;
    try {
      const [mv, cf, inc, budgets, tv, pv] = await Promise.all([
        views.month(m, { members }), audit.cashFlow(m, members), audit.income(m),
        audit.budgetStatus(m), views.tags(), views.projects(),
      ]);
      [month, cash, income, budgetStatus, tagTotals, progress] = [mv, cf.value, inc.value, budgets.value.rows, tv, pv];
      dash = accountId && $jointAccountEnabled ? (await audit.jointDashboard(accountId, m)).value : null;
      updateChart();
    } catch (err) {
      loadError = err.message;
    }
  }

  $: scopedIncome = Object.values(income).filter((i) => !$scopedMembers || $scopedMembers.includes(i.member));
  $: hasIncomeData = cash?.income > 0;
  $: categoryRows = month.by_category.map((b) => ({ ...b, label: $categoryName(b.key) }));
  $: filteredCategoryRows = categoryRows.filter((c) => c.label.toLowerCase().includes(categoryFilterText.toLowerCase()));
  $: budgetRows = budgetStatus.filter((r) => r.limit > 0);
  const color = (id) => $memberById.get(id)?.color ?? '#6366f1';

  // ── Scope ──────────────────────────────────────────────────────────────────
  $: selectedTokens = parseDashboardScope($dashboardScope);
  $: isEveryoneSelected = selectedTokens.includes('ALL');

  function toggleToken(token) {
    const current = selectedTokens.filter((t) => t !== 'ALL');
    const next = current.includes(token) ? current.filter((t) => t !== token) : [...current, token];
    dashboardScope.set(next.length ? next.join(',') : 'ALL');
  }

  // ── Chart ──────────────────────────────────────────────────────────────────
  let doughnutCanvas;
  let chartInstance = null;
  let chartMetric = 'category';
  let currentChartType = $chartStyle === 'bar' ? 'bar' : 'doughnut';
  let categoryFilterText = '';

  /** Chart series: labels, values in currency units (display only), colours, drilldown keys. */
  function chartData() {
    if (chartMetric === 'payer') {
      return { rows: month.by_member, label: (b) => $memberById.get(b.key)?.name ?? `#${b.key}`, color: (b) => color(b.key), type: 'payer' };
    }
    if (chartMetric === 'tag') {
      const rows = tagTotals.map((t) => ({ key: t.tag, total: t.total }));
      return { rows, label: (b) => $tagById.get(b.key)?.name ?? `#${b.key}`, color: (b) => $tagById.get(b.key)?.color ?? '#f59e0b', type: 'tag' };
    }
    return { rows: month.by_category, label: (b) => $categoryName(b.key), color: (_, i) => PALETTE[i % PALETTE.length], type: 'category' };
  }

  function series() {
    const d = chartData();
    return { ...d, labels: d.rows.map(d.label), data: d.rows.map((b) => b.total / 100), colors: d.rows.map(d.color) };
  }

  function updateChart() {
    if (!chartInstance?.canvas?.ownerDocument) return;
    const { labels, data, colors } = series();
    chartInstance.data.labels = labels;
    chartInstance.data.datasets[0].data = data;
    chartInstance.data.datasets[0].backgroundColor = colors;
    chartInstance.update();
  }

  function chartOptions(type, c) {
    const bar = type === 'bar' || type === 'bar-h';
    const font = { family: 'Plus Jakarta Sans, Inter, system-ui, sans-serif', size: 11 };
    const axis = { grid: { color: c.grid }, ticks: { color: c.tick, font } };
    return {
      responsive: true,
      maintainAspectRatio: false,
      cutout: type === 'doughnut' ? '70%' : undefined,
      indexAxis: type === 'bar-h' ? 'y' : 'x',
      scales: bar ? { x: axis, y: axis } : undefined,
      onClick: (_, elements) => drill(elements),
      plugins: {
        legend: bar ? { display: false } : { position: 'bottom', labels: { color: c.tick, font, boxWidth: 10, boxHeight: 10, padding: 12 } },
        tooltip: { ...c.tooltip, callbacks: { label: (ctx) => ` ${$money(Math.round(ctx.raw * 100))}` } },
      },
    };
  }

  function drill(elements) {
    if (!elements?.length) return;
    const d = chartData();
    const row = d.rows[elements[0].index];
    if (row && d.type !== 'tag') openDrilldown(d.type, d.label(row), { id: row.key });
  }

  function createChart(type) {
    if (!doughnutCanvas) return;
    chartInstance?.destroy();
    const c = chartColors();
    const border = isDarkTheme() ? '#080c14' : '#ffffff';
    const bar = type === 'bar' || type === 'bar-h';
    const { labels, data, colors } = series();
    chartInstance = new Chart(doughnutCanvas.getContext('2d'), {
      type: bar ? 'bar' : type === 'polar' ? 'polarArea' : type,
      data: { labels, datasets: [{ data, backgroundColor: colors, borderColor: bar ? 'transparent' : border, borderWidth: bar ? 0 : 3, borderRadius: bar ? 6 : 0 }] },
      options: chartOptions(type, c),
    });
  }

  function setChartStyleType(type) {
    currentChartType = type;
    createChart(type);
  }

  function setMetricType(metric) {
    chartMetric = metric;
    createChart(currentChartType);
  }

  $: if ($theme) createChart(currentChartType);

  onMount(() => createChart(currentChartType));
  onDestroy(() => chartInstance?.destroy());

  function keyed(handler) {
    return (e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handler(); } };
  }
</script>

<div class="space-y-6">

  <!-- ── 1. GLOBAL SCOPE SWITCHER ─────────────────────────────────────────── -->
  <div class="card p-3.5 sm:p-4">
    <div class="flex flex-col lg:flex-row lg:items-center justify-between gap-3">
      <div class="flex items-center gap-2">
        <span class="text-xs font-bold text-neutral-600 dark:text-neutral-300 uppercase tracking-wider">Dashboard Scope:</span>
        <span class="text-xs text-neutral-500 hidden sm:inline">
          {#if isEveryoneSelected}
            Showing entire household overview
          {:else}
            Filtered to: <strong class="text-indigo-600 dark:text-indigo-300">{($scopedMembers ?? []).map((id) => $memberById.get(id)?.name).join(', ')}</strong>
          {/if}
        </span>
      </div>

      <div class="flex flex-wrap items-center gap-1.5 p-1 bg-neutral-100 dark:bg-neutral-950/80 rounded-xl border border-neutral-200 dark:border-neutral-800/90" role="group" aria-label="Dashboard Scope">
        <label class="cursor-pointer">
          <input type="checkbox" name="dashboard-scope" value="ALL" checked={isEveryoneSelected} on:change={() => dashboardScope.set('ALL')} class="sr-only" />
          <span class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all {isEveryoneSelected ? 'bg-indigo-600 text-white shadow-sm ring-1 ring-indigo-400' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200 hover:bg-neutral-200 dark:hover:bg-neutral-800/50'}">
            <span>👥</span><span>Everyone</span>
          </span>
        </label>

        {#each $activeMembers as u (u.id)}
          {@const isChecked = selectedTokens.includes(`MEMBER:${u.id}`)}
          <label class="cursor-pointer">
            <input type="checkbox" name="dashboard-scope" value="MEMBER:{u.id}" checked={isChecked} on:change={() => toggleToken(`MEMBER:${u.id}`)} class="sr-only" />
            <span class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all {isChecked ? 'bg-indigo-600 text-white shadow-sm ring-1 ring-indigo-400' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200 hover:bg-neutral-200 dark:hover:bg-neutral-800/50'}">
              <span class="w-2 h-2 rounded-full flex-none" style="background-color: {u.color}"></span>
              <span>👤 {u.name}</span>
            </span>
          </label>
        {/each}

        {#if $jointAccountEnabled}
          {#each $accounts as ja (ja.id)}
            {@const isChecked = selectedTokens.includes(`ACCOUNT:${ja.id}`)}
            <label class="cursor-pointer">
              <input type="checkbox" name="dashboard-scope" value="ACCOUNT:{ja.id}" checked={isChecked} on:change={() => toggleToken(`ACCOUNT:${ja.id}`)} class="sr-only" />
              <span class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all {isChecked ? 'bg-indigo-600 text-white shadow-sm ring-1 ring-indigo-400' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200 hover:bg-neutral-200 dark:hover:bg-neutral-800/50'}">
                <span>🏦</span><span>{ja.name}</span>
              </span>
            </label>
          {/each}
        {/if}
      </div>
    </div>
  </div>

  {#if loadError}
    <p class="text-xs text-rose-600 dark:text-red-400">{loadError}</p>
  {/if}

  <!-- ── 2. DASHBOARD SUBTABS NAVIGATION ───────────────────────────────────── -->
  <div class="flex items-center gap-2 overflow-x-auto border-b border-neutral-200 dark:border-neutral-800 pb-3 scrollbar-none">
    <button
      type="button"
      on:click={() => switchSubtab('pulse')}
      class="px-3.5 py-1.5 rounded-xl text-xs font-semibold transition-all flex items-center gap-1.5 cursor-pointer whitespace-nowrap {activeSubtab === 'pulse' ? 'bg-indigo-600 text-white shadow-xs' : 'bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
    >
      <span>📊</span>
      <span>Financial Pulse & Spend</span>
    </button>

    <button
      type="button"
      on:click={() => switchSubtab('settle')}
      class="px-3.5 py-1.5 rounded-xl text-xs font-semibold transition-all flex items-center gap-1.5 cursor-pointer whitespace-nowrap {activeSubtab === 'settle' ? 'bg-indigo-600 text-white shadow-xs' : 'bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
    >
      <span>⚖️</span>
      <span>Reimbursements & Settle Up</span>
    </button>

    {#if $jointAccountEnabled}
      <button
        type="button"
        on:click={() => switchSubtab('joint')}
        class="px-3.5 py-1.5 rounded-xl text-xs font-semibold transition-all flex items-center gap-1.5 cursor-pointer whitespace-nowrap {activeSubtab === 'joint' ? 'bg-indigo-600 text-white shadow-xs' : 'bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
      >
        <span>🏦</span>
        <span>Joint Account & Deposits</span>
      </button>
    {/if}

    <button
      type="button"
      on:click={() => switchSubtab('budgets')}
      class="px-3.5 py-1.5 rounded-xl text-xs font-semibold transition-all flex items-center gap-1.5 cursor-pointer whitespace-nowrap {activeSubtab === 'budgets' ? 'bg-indigo-600 text-white shadow-xs' : 'bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
    >
      <span>🎯</span>
      <span>Category Budgets & Health</span>
    </button>

    <button
      type="button"
      on:click={() => switchSubtab('projections')}
      class="px-3.5 py-1.5 rounded-xl text-xs font-semibold transition-all flex items-center gap-1.5 cursor-pointer whitespace-nowrap {activeSubtab === 'projections' ? 'bg-indigo-600 text-white shadow-xs' : 'bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
    >
      <span>📈</span>
      <span>Live Timeline & Projects</span>
    </button>

    <button
      type="button"
      on:click={() => switchSubtab('complete')}
      class="px-3.5 py-1.5 rounded-xl text-xs font-semibold transition-all flex items-center gap-1.5 cursor-pointer whitespace-nowrap {activeSubtab === 'complete' ? 'bg-indigo-600 text-white shadow-xs' : 'bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
    >
      <span>🌐</span>
      <span>Complete Overview</span>
    </button>
  </div>

  <!-- ═══ SUBTAB 1: Financial Pulse & Spend ═════════════════════════════════ -->
  {#if activeSubtab === 'pulse' || activeSubtab === 'complete'}
    <div class="space-y-6 animate-fadeIn">
      {#if activeSubtab === 'complete'}
        <div class="flex items-center gap-2 pt-2 border-b border-neutral-200/80 dark:border-neutral-800/80 pb-2">
          <span class="text-sm">📊</span>
          <h2 class="text-xs font-bold uppercase tracking-wider text-neutral-700 dark:text-neutral-300">Financial Pulse & Spend</h2>
        </div>
      {/if}

      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">

        <!-- 1. Household income (Auditor) -->
        <div
          class="card p-4 sm:p-5 flex flex-col justify-between hover:border-indigo-300 dark:hover:border-indigo-700/60 hover:shadow-md transition-all cursor-pointer group"
          role="button" tabindex="0"
          on:click={() => openDrilldown('income', 'Household Income')}
          on:keydown={keyed(() => openDrilldown('income', 'Household Income'))}
          aria-label="View household income breakdown"
        >
          <div>
            <div class="flex items-center justify-between gap-2 mb-2">
              <p class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider group-hover:text-indigo-600 dark:group-hover:text-indigo-400 transition-colors">Household Income</p>
              <span class="badge-indigo">Income</span>
            </div>
            <p class="font-bold text-neutral-900 dark:text-white tabular-nums truncate text-[clamp(1.25rem,3.5vw,1.75rem)]">
              <span class:privacy-masked={$privacyShield}>{hasIncomeData ? $money(cash.income, true) : '—'}</span>
            </p>
          </div>
          <div class="mt-3 pt-2.5 border-t border-neutral-200 dark:border-neutral-800/80 flex items-center justify-between text-[11px]">
            {#if hasIncomeData}
              <div class="flex items-center gap-2 truncate">
                {#each scopedIncome as i (i.member)}
                  <span class="truncate" style="color: {color(i.member)}"><span class:privacy-masked={$privacyShield}>{$memberById.get(i.member)?.name}: {$money(i.total, true)}</span></span>
                {/each}
              </div>
            {:else}
              <span class="text-neutral-500">Configure salary on Income tab</span>
            {/if}
          </div>
        </div>

        <!-- 2. Monthly total spend -->
        {#if $dashboardWidgets.monthlyTotal !== false}
          <div
            class="card p-4 sm:p-5 flex flex-col justify-between hover:border-amber-300 dark:hover:border-amber-700/60 hover:shadow-md transition-all cursor-pointer group"
            role="button" tabindex="0"
            on:click={() => openDrilldown('monthly-total', `Monthly Total (${$selectedMonth})`)}
            on:keydown={keyed(() => openDrilldown('monthly-total', `Monthly Total (${$selectedMonth})`))}
            aria-label="View all monthly expenses"
          >
            <div>
              <div class="flex items-center justify-between gap-2 mb-2">
                <p class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider group-hover:text-amber-600 dark:group-hover:text-amber-400 transition-colors">Monthly Total</p>
                <span class="badge-amber">Expenses</span>
              </div>
              <p class="font-bold text-neutral-900 dark:text-white tabular-nums truncate text-[clamp(1.25rem,3.5vw,1.75rem)]">
                <span class:privacy-masked={$privacyShield}>{$money(month.total, true)}</span>
              </p>
            </div>
            <div class="mt-3 pt-2.5 border-t border-neutral-200 dark:border-neutral-800/80 flex items-center justify-between text-[11px] text-neutral-500">
              <span>This calendar month</span>
              <span class="text-neutral-700 dark:text-neutral-400 font-medium">{month.count} logged</span>
            </div>
          </div>
        {/if}

        <!-- 3–4. Cash flow and savings rate (Auditor) -->
        {#if $dashboardWidgets.cashFlowSavings !== false && cash}
          <div class="card p-4 sm:p-5 flex flex-col justify-between">
            <div>
              <div class="flex items-center justify-between gap-2 mb-2">
                <p class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">Net Cash Flow</p>
                {#if !hasIncomeData}
                  <span class="badge-neutral">Spend Only</span>
                {:else if cash.net >= 0}
                  <span class="badge-emerald">+{fmtPct(cash.savings_rate_pct)} Saved</span>
                {:else}
                  <span class="badge-red">Deficit</span>
                {/if}
              </div>
              <p class="font-bold tabular-nums truncate text-[clamp(1.25rem,3.5vw,1.75rem)] {cash.net >= 0 ? 'text-emerald-600 dark:text-emerald-400' : 'text-rose-600 dark:text-red-400'}">
                <span class:privacy-masked={$privacyShield}>{cash.net >= 0 ? '+' : ''}{$money(cash.net, true)}</span>
              </p>
            </div>
            <div class="mt-3 pt-2.5 border-t border-neutral-200 dark:border-neutral-800/80 text-[11px] text-neutral-500">
              {hasIncomeData ? (cash.net >= 0 ? 'Surplus retained' : 'Over monthly income') : 'Income not recorded'}
            </div>
          </div>

          <div class="card p-4 sm:p-5 flex flex-col justify-between">
            <div>
              <div class="flex items-center justify-between gap-2 mb-2">
                <p class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">Savings Rate</p>
                <span class="badge-emerald">{hasIncomeData ? fmtPct(cash.savings_rate_pct) : 'Target 20%+'}</span>
              </div>
              <p class="font-bold text-neutral-900 dark:text-white tabular-nums truncate text-[clamp(1.25rem,3.5vw,1.75rem)]">
                <span class:privacy-masked={$privacyShield}>{hasIncomeData ? fmtPct(cash.savings_rate_pct, 1) : '—'}</span>
              </p>
            </div>
            <div class="mt-3 pt-2.5 border-t border-neutral-200 dark:border-neutral-800/80 text-[11px] text-neutral-500">
              {hasIncomeData ? 'Net cash flow over effective income' : 'Track income to calculate'}
            </div>
          </div>
        {/if}
      </div>

      <!-- Per-member spend cards -->
      {#if $dashboardWidgets.payerBreakdown !== false}
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {#each month.by_member as row (row.key)}
            {@const c = color(row.key)}
            {@const label = $memberById.get(row.key)?.name ?? `#${row.key}`}
            <div
              class="card p-4 sm:p-5 transition-all hover:shadow-md cursor-pointer group"
              style="border-color:{c}50"
              role="button" tabindex="0"
              on:click={() => openDrilldown('payer', label, { id: row.key })}
              on:keydown={keyed(() => openDrilldown('payer', label, { id: row.key }))}
              aria-label="View expenses paid by {label}"
            >
              <div class="flex items-center justify-between gap-2 mb-2">
                <p class="text-xs font-semibold uppercase tracking-wider" style="color:{c}">{label}</p>
                <span class="text-[10px] font-bold px-2 py-0.5 rounded-full" style="background-color:{c}15; color:{c}; border:1px solid {c}40">
                  {fmtPct(row.share_pct)} of total
                </span>
              </div>
              <p class="font-bold tabular-nums truncate text-[clamp(1.25rem,4vw,1.875rem)]" style="color:{c}">
                <span class:privacy-masked={$privacyShield}>{$money(row.total, true)}</span>
              </p>
              <p class="text-xs text-neutral-500 mt-1">{row.count} expenses</p>
            </div>
          {/each}
        </div>
      {/if}

      {#if $dashboardWidgets.categoryChart !== false}
      <div class="grid grid-cols-1 lg:grid-cols-5 gap-6">
        <div class="lg:col-span-3 card p-5 sm:p-6 space-y-4">
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
            <div>
              <h2 class="text-sm font-semibold text-neutral-800 dark:text-neutral-200">
                {chartMetric === 'payer' ? 'Spending by Payer' : chartMetric === 'tag' ? 'Spending by Tag (all time)' : 'Spend by Category'}
              </h2>
              <p class="text-xs text-neutral-500 mt-0.5">Visual distribution of logged expenses</p>
            </div>
            <!-- Chart Customizer Controls -->
            <div class="flex items-center gap-2 flex-wrap">
              <!-- Metric focus switcher -->
              <div class="flex items-center bg-neutral-100 dark:bg-neutral-950 p-0.5 rounded-lg border border-neutral-200 dark:border-neutral-800 text-[11px]">
                <button
                  type="button"
                  on:click={() => setMetricType('category')}
                  class="px-2 py-1 rounded font-medium transition-colors {chartMetric === 'category' ? 'bg-indigo-600 text-white' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
                >
                  Categories
                </button>
                <button
                  type="button"
                  on:click={() => setMetricType('payer')}
                  class="px-2 py-1 rounded font-medium transition-colors {chartMetric === 'payer' ? 'bg-indigo-600 text-white' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
                >
                  Payers
                </button>
                {#if $tags.length > 0}
                  <button
                    type="button"
                    on:click={() => setMetricType('tag')}
                    class="px-2 py-1 rounded font-medium transition-colors {chartMetric === 'tag' ? 'bg-indigo-600 text-white' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
                  >
                    Tags
                  </button>
                {/if}
              </div>

              <!-- Chart type switcher -->
              <div class="flex items-center bg-neutral-100 dark:bg-neutral-950 p-0.5 rounded-lg border border-neutral-200 dark:border-neutral-800 text-[11px]">
                <button
                  type="button"
                  on:click={() => setChartStyleType('doughnut')}
                  class="px-2 py-1 rounded font-medium transition-colors {currentChartType === 'doughnut' ? 'bg-neutral-200 dark:bg-neutral-800 text-neutral-900 dark:text-white' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
                  title="Doughnut Chart"
                >
                  🍩
                </button>
                <button
                  type="button"
                  on:click={() => setChartStyleType('bar')}
                  class="px-2 py-1 rounded font-medium transition-colors {currentChartType === 'bar' ? 'bg-neutral-200 dark:bg-neutral-800 text-neutral-900 dark:text-white' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
                  title="Vertical Bar Chart"
                >
                  📊
                </button>
                <button
                  type="button"
                  on:click={() => setChartStyleType('bar-h')}
                  class="px-2 py-1 rounded font-medium transition-colors {currentChartType === 'bar-h' ? 'bg-neutral-200 dark:bg-neutral-800 text-neutral-900 dark:text-white' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
                  title="Horizontal Bar Chart"
                >
                  📑
                </button>
                <button
                  type="button"
                  on:click={() => setChartStyleType('polar')}
                  class="px-2 py-1 rounded font-medium transition-colors {currentChartType === 'polar' ? 'bg-neutral-200 dark:bg-neutral-800 text-neutral-900 dark:text-white' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
                  title="Polar Area Chart"
                >
                  🧭
                </button>
              </div>
            </div>
          </div>

          <div class="relative h-64 sm:h-72" class:hidden={month.by_category.length === 0}>
            <canvas bind:this={doughnutCanvas} id="category-doughnut-chart"></canvas>
          </div>

          {#if month.by_category.length === 0}
            <div class="empty-state-box my-4">
              <div class="w-12 h-12 rounded-2xl bg-neutral-200/80 dark:bg-neutral-800/80 flex items-center justify-center mb-2">
                <svg class="w-6 h-6 text-neutral-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M10.5 6a7.5 7.5 0 107.5 7.5h-7.5V6z" />
                  <path stroke-linecap="round" stroke-linejoin="round" d="M13.5 10.5H21A7.5 7.5 0 0013.5 3v7.5z" />
                </svg>
              </div>
              <p class="text-neutral-700 dark:text-neutral-300 text-sm font-semibold">No category data yet.</p>
              <p class="text-neutral-500 text-xs max-w-xs mt-1">
                Log an expense on the Expenses tab and the breakdown will appear here.
              </p>
            </div>
          {/if}
        </div>

        <div class="lg:col-span-2 card p-5 space-y-3">
          <div class="flex items-center justify-between">
            <div>
              <h3 class="text-xs font-bold text-neutral-700 dark:text-neutral-300 uppercase tracking-wider">Categories ({categoryRows.length})</h3>
              <p class="text-[10px] text-neutral-400 mt-0.5">Click any category for member breakdown & expenses</p>
            </div>
            <input type="text" bind:value={categoryFilterText} placeholder="Search category…" class="input-field py-1 px-2.5 text-xs max-w-[140px]" />
          </div>

          <div class="space-y-2 max-h-[300px] overflow-y-auto pr-1">
            {#each filteredCategoryRows as row, i (row.key)}
              <div
                class="p-2.5 rounded-xl bg-neutral-50 dark:bg-neutral-950/80 border border-neutral-200 dark:border-neutral-800/80 space-y-1.5 hover:border-indigo-300 dark:hover:border-indigo-700/60 transition-all cursor-pointer group"
                role="button" tabindex="0"
                on:click={() => openDrilldown('category', row.label, { id: row.key })}
                on:keydown={keyed(() => openDrilldown('category', row.label, { id: row.key }))}
                aria-label="View member breakdown and expenses for {row.label}"
              >
                <div class="flex items-center justify-between text-xs">
                  <div class="flex items-center gap-1.5 min-w-0">
                    <span class="w-2.5 h-2.5 rounded-full flex-none" style="background:{PALETTE[i % PALETTE.length]}"></span>
                    <span class="text-neutral-800 dark:text-neutral-200 font-medium truncate group-hover:text-indigo-600 dark:group-hover:text-indigo-400 transition-colors">{row.label}</span>
                    <span class="text-[10px] text-neutral-500">({row.count})</span>
                  </div>
                  <span class="text-neutral-900 dark:text-white font-bold tabular-nums">{$money(row.total)}</span>
                </div>
                <div class="w-full h-1.5 bg-neutral-200 dark:bg-neutral-800 rounded-full overflow-hidden">
                  <div class="h-full rounded-full bg-indigo-500 transition-all duration-500" style="width: {row.share_pct ?? 0}%"></div>
                </div>
              </div>
            {/each}
          </div>
        </div>
      </div>
      {/if}
    </div>
  {/if}

  <!-- ═══ SUBTAB 2: Reimbursements & Settle Up ══════════════════════════════ -->
  {#if activeSubtab === 'settle' || activeSubtab === 'complete'}
    <div class="space-y-6 animate-fadeIn">
      {#if activeSubtab === 'complete'}
        <div class="flex items-center gap-2 pt-6 border-b border-neutral-200/80 dark:border-neutral-800/80 pb-2">
          <span class="text-sm">⚖️</span>
          <h2 class="text-xs font-bold uppercase tracking-wider text-neutral-700 dark:text-neutral-300">Reimbursements & Settle Up</h2>
        </div>
      {/if}
      <PaybackVisual />
    </div>
  {/if}

  <!-- ═══ SUBTAB 3: Joint Account (Auditor dashboard) ═══════════════════════ -->
  {#if (activeSubtab === 'joint' || activeSubtab === 'complete') && $jointAccountEnabled}
    <div class="space-y-6 animate-fadeIn">
      {#if activeSubtab === 'complete'}
        <div class="flex items-center gap-2 pt-6 border-b border-neutral-200/80 dark:border-neutral-800/80 pb-2">
          <span class="text-sm">🏦</span>
          <h2 class="text-xs font-bold uppercase tracking-wider text-neutral-700 dark:text-neutral-300">Joint Account & Deposits</h2>
        </div>
      {/if}
      {#if dash && account}
        <div class="card border-indigo-500/30 bg-indigo-50/50 dark:bg-indigo-500/5 p-5 sm:p-6 space-y-5">
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
            <div>
              <div class="flex items-center gap-2">
                <span class="text-xl">🏦</span>
                <h2 class="text-base font-bold text-neutral-900 dark:text-white">{account.name} Health</h2>
              </div>
              <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Joint spending, deposit target and safety buffer</p>
            </div>
            <div class="flex items-center gap-2 text-xs bg-white dark:bg-neutral-900/90 px-3.5 py-2 rounded-xl border border-indigo-500/30 flex-none shadow-sm">
              <span class="text-neutral-500 dark:text-neutral-400">Current Balance:</span>
              <span class="font-bold text-neutral-900 dark:text-white tabular-nums">{$money(dash.balance)}</span>
            </div>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <div
              class="card-sub space-y-1 p-4 hover:border-indigo-300 dark:hover:border-indigo-700/60 transition-all cursor-pointer group"
              role="button" tabindex="0"
              on:click={() => openDrilldown('joint-spent', `${account.name} Spend`, { id: account.id })}
              on:keydown={keyed(() => openDrilldown('joint-spent', `${account.name} Spend`, { id: account.id }))}
              aria-label="View joint expenses"
            >
              <p class="text-[11px] font-medium text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">Spent ({dash.month})</p>
              <p class="text-xl font-bold text-neutral-900 dark:text-white tabular-nums">{$money(dash.actual_total)}</p>
              <p class="text-[11px] text-neutral-500">Expected: {$money(dash.expected_total)}</p>
            </div>

            <div class="card-sub border-indigo-300 dark:border-indigo-500/40 bg-indigo-100/60 dark:bg-indigo-950/40 space-y-1 p-4">
              <p class="text-[11px] font-semibold text-indigo-700 dark:text-indigo-300 uppercase tracking-wider">Deposit Target</p>
              <p class="text-xl font-bold text-indigo-900 dark:text-indigo-200 tabular-nums">{$money(dash.target)}</p>
              <p class="text-[11px] text-indigo-600 dark:text-indigo-400/80">Expected costs + {dash.safety_margin_pct}% safety margin</p>
            </div>

            <div class="card-sub space-y-1 p-4">
              <p class="text-[11px] font-medium text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">Deposits Received</p>
              <p class="text-xl font-bold text-neutral-900 dark:text-white tabular-nums">{$money(dash.deposited_total)}</p>
              {#if dash.status.status === 'target_met'}
                <p class="text-[11px] font-semibold text-emerald-600 dark:text-emerald-400">✓ Target met</p>
              {:else if dash.status.status === 'pending'}
                <p class="text-[11px] font-semibold text-indigo-600 dark:text-indigo-400">⏳ Pending <span class="text-neutral-500 font-normal">(due day {dash.status.due_day})</span></p>
              {:else}
                <p class="text-[11px] font-semibold text-amber-600 dark:text-amber-400">⚠ Below target</p>
              {/if}
            </div>
          </div>

          {#if dash.top_up > 0}
            <div class="px-4 py-3 bg-indigo-100/80 dark:bg-indigo-950/50 border border-indigo-300 dark:border-indigo-800/60 rounded-xl text-xs text-indigo-800 dark:text-indigo-300">
              💡 <strong>Deposit Adjustment Needed:</strong> Top up at least <strong>{$money(dash.top_up)}</strong> to maintain the agreed {dash.safety_margin_pct}% safety margin.
            </div>
          {/if}
        </div>
      {:else}
        <div class="empty-state-box py-8"><p class="text-neutral-500 text-sm">No joint account configured.</p></div>
      {/if}
    </div>
  {/if}

  <!-- ═══ SUBTAB 4: Budgets (Auditor) ═══════════════════════════════════════ -->
  {#if activeSubtab === 'budgets' || activeSubtab === 'complete'}
    <div class="space-y-6 animate-fadeIn">
      {#if activeSubtab === 'complete'}
        <div class="flex items-center gap-2 pt-6 border-b border-neutral-200/80 dark:border-neutral-800/80 pb-2">
          <span class="text-sm">🎯</span>
          <h2 class="text-xs font-bold uppercase tracking-wider text-neutral-700 dark:text-neutral-300">Category Budgets & Health</h2>
        </div>
      {/if}
      <div class="card p-5 sm:p-6 space-y-4">
        <div class="flex items-center justify-between">
          <div>
            <h2 class="text-sm font-semibold text-neutral-800 dark:text-neutral-200">Monthly Budget Limits & Spending Health</h2>
            <p class="text-xs text-neutral-500 mt-0.5">Actual expenses vs category budget caps</p>
          </div>
          <span class="badge-indigo">{$selectedMonth}</span>
        </div>

        {#if budgetRows.length > 0}
          <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
            {#each budgetRows as row (row.category)}
              {@const level = row.pct_used >= 90 ? 'red' : row.pct_used >= 70 ? 'yellow' : 'green'}
              {@const label = $categoryName(row.category)}
              <div
                class="card-sub p-3.5 space-y-2 hover:border-indigo-300 dark:hover:border-indigo-700/60 transition-all cursor-pointer group"
                role="button" tabindex="0"
                on:click={() => openDrilldown('budget', label, { id: row.category, row })}
                on:keydown={keyed(() => openDrilldown('budget', label, { id: row.category, row }))}
                aria-label="View expenses for budget {label}"
              >
                <p class="text-xs text-neutral-800 dark:text-neutral-300 font-semibold uppercase truncate">{label}</p>
                <div class="flex items-baseline justify-between text-xs">
                  <p class="font-bold text-neutral-900 dark:text-neutral-200 tabular-nums">
                    {$money(row.spent)} <span class="text-neutral-500 font-normal">/ {$money(row.limit)}</span>
                  </p>
                  <span class="font-bold tabular-nums {level === 'red' ? 'text-rose-600 dark:text-red-400' : level === 'yellow' ? 'text-amber-600 dark:text-yellow-400' : 'text-emerald-600 dark:text-emerald-400'}">{fmtPct(row.pct_used)}</span>
                </div>
                <div class="w-full h-2 bg-neutral-200 dark:bg-neutral-800 rounded-full overflow-hidden">
                  <div class="h-full rounded-full transition-all duration-500 {level === 'red' ? 'bg-red-500' : level === 'yellow' ? 'bg-yellow-400' : 'bg-emerald-500'}" style="width: {Math.min(row.pct_used, 100)}%"></div>
                </div>
                <p class="text-[10px] text-neutral-500">
                  {row.remaining > 0 ? `Remaining buffer: ${$money(row.remaining)}` : '⚠ Budget limit exceeded'}
                </p>
              </div>
            {/each}
          </div>
        {:else}
          <div class="empty-state-box py-8">
            <p class="text-neutral-700 dark:text-neutral-300 text-sm font-semibold">No category budgets configured.</p>
            <p class="text-neutral-500 text-xs mt-1">Configure limits on the Budgets tab.</p>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- ═══ SUBTAB 5: Live Timeline & Projects ════════════════════════════════ -->
  {#if activeSubtab === 'projections' || activeSubtab === 'complete'}
    <div class="space-y-6 animate-fadeIn">
      {#if activeSubtab === 'complete'}
        <div class="flex items-center gap-2 pt-6 border-b border-neutral-200/80 dark:border-neutral-800/80 pb-2">
          <span class="text-sm">📈</span>
          <h2 class="text-xs font-bold uppercase tracking-wider text-neutral-700 dark:text-neutral-300">Live Timeline & Projects</h2>
        </div>
      {/if}
      <div class="card p-5 sm:p-6 space-y-4">
        <div class="flex items-center justify-between">
          <div>
            <h2 class="text-sm font-semibold text-neutral-800 dark:text-neutral-200">Expense Timeline</h2>
            <p class="text-xs text-neutral-500 mt-0.5">Monthly spend by category</p>
          </div>
          <span class="flex items-center gap-1.5 text-xs text-emerald-600 dark:text-emerald-400 font-medium">
            <span class="w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></span>
            Live
          </span>
        </div>
        <RealtimeChart />
      </div>

      <div class="card p-5 sm:p-6 space-y-4">
        <div>
          <h2 class="text-sm font-semibold text-neutral-800 dark:text-neutral-200">Income by Person</h2>
          <p class="text-xs text-neutral-500 mt-0.5">Effective monthly salary & one-off earnings</p>
        </div>
        <IncomeChart />
      </div>

      {#if progress.length > 0}
        <div class="card p-5 sm:p-6 space-y-4">
          <div>
            <h2 class="text-sm font-semibold text-neutral-800 dark:text-neutral-200">Target Savings Projects</h2>
            <p class="text-xs text-neutral-500 mt-0.5">Milestone goals progress</p>
          </div>
          <div class="space-y-3">
            {#each progress as p (p.project)}
              {@const name = $projectById.get(p.project)?.name ?? `#${p.project}`}
              {@const pct = p.progress_pct ?? 0}
              {@const done = p.completion.status === 'completed'}
              <div
                class="card-sub p-3.5 flex items-center gap-4 hover:border-indigo-300 dark:hover:border-indigo-700/60 transition-all cursor-pointer group"
                role="button" tabindex="0"
                on:click={() => openDrilldown('project', name, { id: p.project })}
                on:keydown={keyed(() => openDrilldown('project', name, { id: p.project }))}
                aria-label="View expenses for project {name}"
              >
                <div class="flex-1 min-w-0">
                  <div class="flex justify-between items-baseline mb-1">
                    <span class="text-xs font-semibold text-neutral-800 dark:text-neutral-200 truncate group-hover:text-indigo-600 dark:group-hover:text-indigo-400 transition-colors">{name}</span>
                    <span class="text-xs tabular-nums {done ? 'text-emerald-600 dark:text-emerald-400 font-bold' : 'text-neutral-500 dark:text-neutral-400'} ml-2 flex-none">{fmtPct(pct)}</span>
                  </div>
                  <div class="w-full h-2 bg-neutral-200 dark:bg-neutral-800 rounded-full overflow-hidden">
                    <div class="h-full rounded-full bg-gradient-to-r {done ? 'from-emerald-500 to-emerald-400' : 'from-indigo-500 to-violet-500'}" style="width: {pct}%"></div>
                  </div>
                </div>
                <p class="text-xs font-bold text-neutral-900 dark:text-neutral-100 tabular-nums text-right flex-none">
                  {$money(p.spent)} <span class="text-neutral-500 font-normal">/ {$money(p.target)}</span>
                </p>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}

  <DrilldownModal />

</div>
