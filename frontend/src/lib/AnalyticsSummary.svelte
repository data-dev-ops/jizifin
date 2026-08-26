<script>
  /**
   * AnalyticsSummary.svelte
   *
   * Unified Financial Command Center Dashboard with full Light/Dark mode theming:
   *  1. Global Scope Switcher: Multi-select users / joint accounts / household everyone
   *  2. Subtab-Driven Views:
   *     - 📊 Financial Pulse & Spend (KPIs, payer cards, flexible multi-chart panel, category table)
   *     - ⚖️ Reimbursements & Settle Up (Debts graph, category adjustments, month locking)
   *     - 🏦 Joint Accounts & Deposit Adjustments (Targets, deposits, safety margin)
   *     - 🎯 Budgets & Health (Category limits vs actual, over-budget alerts)
   *     - 📈 Cash Flow & Projections (Live timeline, income chart, savings projects)
   */

  import { onMount, onDestroy } from 'svelte';
  import {
    analytics,
    users,
    currencySymbol,
    chartStyle,
    jointAccountEnabled,
    jointDashboard,
    jointAccount,
    jointAccounts,
    dashboardScope,
    incomeEntries,
    incomeAnalytics,
    jobs,
    selectedMonth,
    tags,
    projects,
    theme,
    privacyShield,
    currencyPrecisionMode,
    dashboardWidgets,
  } from './stores.js';
  import { fetchAnalytics, fetchIncomeByPerson, fetchPaybacks, fetchBudgetAnalytics } from './api.js';
  import Chart from 'chart.js/auto';
  import PaybackVisual from './PaybackVisual.svelte';
  import RealtimeChart from './RealtimeChart.svelte';
  import IncomeChart from './IncomeChart.svelte';

  // ── Active Dashboard Subtab ────────────────────────────────────────────────
  let activeSubtab = 'pulse'; // 'pulse' | 'settle' | 'joint' | 'budgets' | 'projections'

  // ── Chart configuration & state ───────────────────────────────────────────
  let doughnutCanvas;
  let chartInstance = null;
  let chartMetric = 'category'; // 'category' | 'payer' | 'tag'
  let currentChartType = $chartStyle === 'bar' ? 'bar' : 'doughnut'; // 'doughnut' | 'bar' | 'polarArea' | 'pie'

  // ── Derived values from analytics store ───────────────────────────────────
  let total = 0;
  let payerRows = [];
  let categories = [];
  let categoryFilterText = '';
  let budgetStatus = [];

  function getIsDark() {
    if (typeof document === 'undefined') return true;
    return document.documentElement.classList.contains('dark');
  }

  /** Look up a user's colour from the users store, with a sensible fallback. */
  function userColor(name) {
    return $users.find((u) => u.name === name)?.color ?? '#6366f1';
  }

  const unsubscribe = analytics.subscribe((v) => {
    total = v.monthly_total?.total_amount ?? 0;
    payerRows = v.by_payer || [];
    categories = v.by_category || [];
    updateChart();
  });

  // ── Chart Palette ─────────────────────────────────────────────────────────
  const PALETTE = [
    'rgba(99, 102, 241, 0.85)',  // indigo
    'rgba(139, 92, 246, 0.85)',  // violet
    'rgba(14, 165, 233, 0.85)',  // sky
    'rgba(236, 72, 153, 0.85)',  // pink
    'rgba(16, 185, 129, 0.85)',  // emerald
    'rgba(245, 158, 11, 0.85)',  // amber
    'rgba(244, 63, 94, 0.85)',   // rose
    'rgba(20, 184, 166, 0.85)',  // teal
  ];

  function getChartData() {
    if (chartMetric === 'payer') {
      return {
        labels: payerRows.map((r) => r.who_paid),
        data: payerRows.map((r) => r.total_amount),
        colors: payerRows.map((r) => userColor(r.who_paid)),
      };
    }
    if (chartMetric === 'tag') {
      const activeTags = $tags.filter((t) => (t.total_amount ?? 0) > 0);
      return {
        labels: activeTags.map((t) => t.name),
        data: activeTags.map((t) => t.total_amount),
        colors: activeTags.map((t) => t.color || '#f59e0b'),
      };
    }
    // Default: category
    return {
      labels: categories.map((c) => c.category),
      data: categories.map((c) => c.total_amount),
      colors: PALETTE,
    };
  }

  function updateChart() {
    if (!chartInstance) return;
    const { labels, data, colors } = getChartData();
    chartInstance.data.labels = labels;
    chartInstance.data.datasets[0].data = data;
    chartInstance.data.datasets[0].backgroundColor = colors;
    chartInstance.update();
  }

  function createChart(type) {
    if (!doughnutCanvas) return;
    if (chartInstance) {
      chartInstance.destroy();
      chartInstance = null;
    }
    const isDark = getIsDark();
    const ctx = doughnutCanvas.getContext('2d');
    const { labels, data, colors } = getChartData();
    const isBar = type === 'bar';
    const isHorizontalBar = type === 'bar-h';

    chartInstance = new Chart(ctx, {
      type: isHorizontalBar ? 'bar' : isBar ? 'bar' : type === 'polar' ? 'polarArea' : type === 'pie' ? 'pie' : 'doughnut',
      data: {
        labels,
        datasets: [
          {
            data,
            backgroundColor: colors,
            borderColor: isBar || isHorizontalBar ? 'transparent' : isDark ? '#080c14' : '#ffffff',
            borderWidth: isBar || isHorizontalBar ? 0 : 3,
            hoverOffset: isBar || isHorizontalBar ? 0 : 6,
            borderRadius: isBar || isHorizontalBar ? 6 : 0,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        cutout: type === 'doughnut' ? '70%' : undefined,
        indexAxis: isHorizontalBar ? 'y' : 'x',
        animation: { duration: 600, easing: 'easeInOutQuart' },
        scales: isBar || isHorizontalBar ? {
          x: {
            grid: { color: isDark ? 'rgba(255,255,255,0.05)' : 'rgba(0,0,0,0.05)' },
            ticks: { color: isDark ? '#9ca3af' : '#64748b', font: { family: 'Plus Jakarta Sans, Inter, system-ui, sans-serif', size: 11 } },
          },
          y: {
            grid: { color: isDark ? 'rgba(255,255,255,0.05)' : 'rgba(0,0,0,0.05)' },
            ticks: { color: isDark ? '#9ca3af' : '#64748b', font: { family: 'Plus Jakarta Sans, Inter, system-ui, sans-serif', size: 11 } },
          },
        } : undefined,
        plugins: {
          legend: isBar || isHorizontalBar ? { display: false } : {
            position: 'bottom',
            labels: {
              color: isDark ? '#9ca3af' : '#64748b',
              font: { family: 'Plus Jakarta Sans, Inter, system-ui, sans-serif', size: 11 },
              boxWidth: 10,
              boxHeight: 10,
              padding: 12,
            },
          },
          tooltip: {
            backgroundColor: isDark ? 'rgba(15, 23, 42, 0.95)' : 'rgba(255, 255, 255, 0.95)',
            borderColor: isDark ? 'rgba(99, 102, 241, 0.4)' : 'rgba(99, 102, 241, 0.25)',
            borderWidth: 1,
            titleColor: isDark ? '#f1f5f9' : '#0f172a',
            bodyColor: isDark ? '#cbd5e1' : '#475569',
            callbacks: {
              label: (ctx) => ` ${$currencySymbol}${Number(ctx.raw).toFixed(2)}`,
            },
          },
        },
      },
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

  function fmt(n, isSummary = false) {
    const val = Number(n || 0);
    if (isSummary && $currencyPrecisionMode === 'whole_units_on_summaries') {
      return `${$currencySymbol}${Math.round(val).toLocaleString('en-GB')}`;
    }
    return `${$currencySymbol}${val.toFixed(2)}`;
  }

  function pct(part, whole) {
    if (!whole || whole === 0) return '—';
    return `${((part / whole) * 100).toFixed(0)}%`;
  }

  // ── Global Scope Selection & Multi-Filter Logic ───────────────────────────
  function parseScope(scopeStr) {
    if (!scopeStr || scopeStr === 'ALL') return ['ALL'];
    const parts = scopeStr.split(',').map((s) => s.trim()).filter(Boolean);
    return parts.length > 0 ? parts : ['ALL'];
  }

  $: selectedTokens = parseScope($dashboardScope);
  $: isEveryoneSelected = selectedTokens.includes('ALL');

  function selectEveryone() {
    dashboardScope.set('ALL');
  }

  function toggleUser(userName) {
    const token = `USER:${userName}`;
    let current = selectedTokens.filter((t) => t !== 'ALL');
    if (current.includes(token)) {
      current = current.filter((t) => t !== token);
    } else {
      current = [...current, token];
    }
    if (current.length === 0) {
      dashboardScope.set('ALL');
    } else {
      dashboardScope.set(current.join(','));
    }
  }

  function toggleJoint(jointId) {
    const token = `JOINT:${jointId}`;
    let current = selectedTokens.filter((t) => t !== 'ALL');
    if (current.includes(token)) {
      current = current.filter((t) => t !== token);
    } else {
      current = [...current, token];
    }
    if (current.length === 0) {
      dashboardScope.set('ALL');
    } else {
      dashboardScope.set(current.join(','));
    }
  }

  // ── Scope resolution helper — shared by fetch block and filteredActiveUsers ──
  function resolveSelectedUserSet(tokens, everyone) {
    if (everyone) return null;
    const userSet = new Set();
    for (const token of tokens) {
      if (token.startsWith('USER:')) {
        userSet.add(token.slice(5));
      } else if (token.startsWith('JOINT:')) {
        const jId = parseInt(token.slice(6), 10);
        const targetJa = ($jointAccounts || []).find((a) => a.id === jId) || ($jointAccount?.id === jId ? $jointAccount : null);
        if (targetJa?.member_names?.length) {
          targetJa.member_names.forEach((m) => userSet.add(m));
        }
      }
    }
    return userSet.size > 0 ? Array.from(userSet) : null;
  }

  // Reactive data fetching on scope or month change.
  // Debounced 15 ms to batch rapid store updates during initial load
  // (jointAccounts + jointAccount both settle before we fire API calls).
  let _fetchDebounceTimer = null;
  $: {
    const _month = $selectedMonth;
    const _tokens = selectedTokens;
    const _everyone = isEveryoneSelected;
    // Touch reactive deps for jointAccounts/jointAccount so Svelte tracks them
    void $jointAccounts;
    void $jointAccount;
    clearTimeout(_fetchDebounceTimer);
    _fetchDebounceTimer = setTimeout(() => {
      const usersParam = resolveSelectedUserSet(_tokens, _everyone);
      fetchAnalytics(_month, usersParam);
      fetchIncomeByPerson(_month, usersParam);
      fetchPaybacks(_month, usersParam);
      fetchBudgetAnalytics(_month).then((rows) => { budgetStatus = rows || []; }).catch(() => {});
    }, 15);
  }

  $: activeUsers = $users.filter((u) => u.is_active !== false);

  $: filteredActiveUsers = activeUsers.filter((u) => {
    if (isEveryoneSelected) return true;
    const selected = resolveSelectedUserSet(selectedTokens, false);
    return selected ? selected.includes(u.name) : true;
  });

  // ── Household Income & Net Cash Flow Calculations —————————————————————
  function toMonthlyEquivalent(amountCents, freq) {
    if (freq === 'weekly') return Math.round((amountCents * 52) / 12);
    if (freq === 'biweekly') return Math.round((amountCents * 26) / 12);
    if (freq === 'annual') return Math.round(amountCents / 12);
    return amountCents;
  }

  function isJobActiveInMonth(job, monthStr) {
    if (!monthStr || !job.is_active) return false;
    const startMonth = `${monthStr}-01`;
    const endMonth = `${monthStr}-31`;
    return job.start_date <= endMonth && (!job.end_date || job.end_date >= startMonth);
  }

  $: userIncomeList = filteredActiveUsers.map((u) => {
    const userJobs = $jobs.filter((j) => j.who === u.name && isJobActiveInMonth(j, $selectedMonth));
    let baseSalaryCents = userJobs.reduce((sum, j) => sum + toMonthlyEquivalent(j.amount_cents, j.frequency), 0);
    const row = $incomeAnalytics.find((r) => r.who === u.name);
    if (row && (row.has_override || userJobs.length === 0)) {
      baseSalaryCents = row.base_salary_cents ?? row.total_cents;
    }
    const oneOffCents = $incomeEntries
      .filter((e) => e.who === u.name && e.category !== 'SALARY')
      .reduce((sum, e) => sum + e.amount_cents, 0);
    return {
      name: u.name,
      color: userColor(u.name),
      baseSalaryCents,
      oneOffCents,
      totalCents: baseSalaryCents + oneOffCents,
    };
  });

  $: totalIncomeEuros = userIncomeList.reduce((sum, u) => sum + u.totalCents, 0) / 100;
  $: hasIncomeData = totalIncomeEuros > 0;
  $: netCashFlow = totalIncomeEuros - total;
  $: savingsRatePct = totalIncomeEuros > 0 ? ((netCashFlow / totalIncomeEuros) * 100) : 0;

  $: filteredCategoryRows = categories.filter((c) =>
    c.category.toLowerCase().includes(categoryFilterText.toLowerCase())
  );

  // Re-apply theme colours to existing chart instead of destroying and recreating it.
  // Full recreate is only needed for chart-type changes.
  $: if ($theme && chartInstance) {
    const isDark = getIsDark();
    const gridColor = isDark ? 'rgba(255,255,255,0.05)' : 'rgba(0,0,0,0.05)';
    const tickColor = isDark ? '#9ca3af' : '#64748b';
    const tooltipBg = isDark ? 'rgba(15, 23, 42, 0.95)' : 'rgba(255, 255, 255, 0.95)';
    const tooltipBorder = isDark ? 'rgba(99, 102, 241, 0.4)' : 'rgba(99, 102, 241, 0.25)';
    const titleColor = isDark ? '#f1f5f9' : '#0f172a';
    const bodyColor = isDark ? '#cbd5e1' : '#475569';
    const legendColor = isDark ? '#9ca3af' : '#64748b';
    const borderColor = isDark ? '#080c14' : '#ffffff';
    // Update colours without destroying the instance
    if (chartInstance.options.plugins?.legend?.labels) {
      chartInstance.options.plugins.legend.labels.color = legendColor;
    }
    if (chartInstance.options.plugins?.tooltip) {
      chartInstance.options.plugins.tooltip.backgroundColor = tooltipBg;
      chartInstance.options.plugins.tooltip.borderColor = tooltipBorder;
      chartInstance.options.plugins.tooltip.titleColor = titleColor;
      chartInstance.options.plugins.tooltip.bodyColor = bodyColor;
    }
    ['x', 'y'].forEach((axis) => {
      if (chartInstance.options.scales?.[axis]) {
        chartInstance.options.scales[axis].grid.color = gridColor;
        chartInstance.options.scales[axis].ticks.color = tickColor;
      }
    });
    if (chartInstance.data?.datasets?.[0]) {
      chartInstance.data.datasets[0].borderColor = borderColor;
    }
    chartInstance.update();
  }

  onMount(() => {
    createChart(currentChartType);
  });

  onDestroy(() => {
    clearTimeout(_fetchDebounceTimer);
    unsubscribe();
    if (chartInstance) chartInstance.destroy();
  });
</script>

<div class="space-y-6">

  <!-- ── 1. GLOBAL SCOPE SWITCHER (Always Active Across All Subtabs) ────────── -->
  <div class="card p-3.5 sm:p-4">
    <div class="flex flex-col lg:flex-row lg:items-center justify-between gap-3">
      <div class="flex items-center gap-2">
        <span class="text-xs font-bold text-neutral-600 dark:text-neutral-300 uppercase tracking-wider">Dashboard Scope:</span>
        <span class="text-xs text-neutral-500 hidden sm:inline">
          {#if isEveryoneSelected}
            Showing entire household overview
          {:else}
            Filtered to: <strong class="text-indigo-600 dark:text-indigo-300">{filteredActiveUsers.map((u) => u.name).join(', ') || 'Selected members'}</strong>
          {/if}
        </span>
      </div>

      <div class="flex flex-wrap items-center gap-1.5 p-1 bg-neutral-100 dark:bg-neutral-950/80 rounded-xl border border-neutral-200 dark:border-neutral-800/90" role="group" aria-label="Dashboard Scope">
        <!-- 1. All Household / Everyone Option (Exclusive) -->
        <label class="cursor-pointer">
          <input
            type="checkbox"
            name="dashboard-scope"
            value="ALL"
            checked={isEveryoneSelected}
            on:change={selectEveryone}
            class="sr-only"
          />
          <span class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all {isEveryoneSelected ? 'bg-indigo-600 text-white shadow-sm ring-1 ring-indigo-400' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200 hover:bg-neutral-200 dark:hover:bg-neutral-800/50'}">
            <span>👥</span>
            <span>Everyone</span>
            {#if isEveryoneSelected}
              <span class="text-[10px] text-white font-bold">✓</span>
            {/if}
          </span>
        </label>

        <!-- 2. Per-User Multi-Select Options -->
        {#each activeUsers as u}
          {@const isChecked = selectedTokens.includes(`USER:${u.name}`)}
          <label class="cursor-pointer">
            <input
              type="checkbox"
              name="dashboard-scope"
              value="USER:{u.name}"
              checked={isChecked}
              on:change={() => toggleUser(u.name)}
              class="sr-only"
            />
            <span class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all {isChecked ? 'bg-indigo-600 text-white shadow-sm ring-1 ring-indigo-400' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200 hover:bg-neutral-200 dark:hover:bg-neutral-800/50'}">
              <span class="w-2 h-2 rounded-full flex-none" style="background-color: {u.color}"></span>
              <span>👤 {u.name}</span>
              {#if isChecked}
                <span class="text-[10px] text-white font-bold">✓</span>
              {/if}
            </span>
          </label>
        {/each}

        <!-- 3. Joint Accounts Multi-Select Options -->
        {#if $jointAccountEnabled && (($jointAccounts && $jointAccounts.length > 0) || $jointAccount)}
          {#if $jointAccounts && $jointAccounts.length > 0}
            {#each $jointAccounts as ja}
              {@const isChecked = selectedTokens.includes(`JOINT:${ja.id}`)}
              <label class="cursor-pointer">
                <input
                  type="checkbox"
                  name="dashboard-scope"
                  value="JOINT:{ja.id}"
                  checked={isChecked}
                  on:change={() => toggleJoint(ja.id)}
                  class="sr-only"
                />
                <span class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all {isChecked ? 'bg-indigo-600 text-white shadow-sm ring-1 ring-indigo-400' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200 hover:bg-neutral-200 dark:hover:bg-neutral-800/50'}">
                  <span>🏦</span>
                  <span>{ja.name}</span>
                  {#if ja.member_names && ja.member_names.length > 0}
                    <span class="text-[10px] opacity-75">({ja.member_names.join('&')})</span>
                  {/if}
                  {#if isChecked}
                    <span class="text-[10px] text-white font-bold">✓</span>
                  {/if}
                </span>
              </label>
            {/each}
          {:else if $jointAccount}
            {@const isChecked = selectedTokens.includes('JOINT:1')}
            <label class="cursor-pointer">
              <input
                type="checkbox"
                name="dashboard-scope"
                value="JOINT:1"
                checked={isChecked}
                on:change={() => toggleJoint(1)}
                class="sr-only"
              />
              <span class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all {isChecked ? 'bg-indigo-600 text-white shadow-sm ring-1 ring-indigo-400' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200 hover:bg-neutral-200 dark:hover:bg-neutral-800/50'}">
                <span>🏦</span>
                <span>{$jointAccount.name}</span>
                {#if isChecked}
                  <span class="text-[10px] text-white font-bold">✓</span>
                {/if}
              </span>
            </label>
          {/if}
        {/if}
      </div>
    </div>
  </div>

  <!-- ── 2. DASHBOARD SUBTABS NAVIGATION ───────────────────────────────────── -->
  <div class="flex items-center gap-2 overflow-x-auto border-b border-neutral-200 dark:border-neutral-800 pb-1 scrollbar-none">
    <button
      type="button"
      on:click={() => (activeSubtab = 'pulse')}
      class="px-4 py-2 rounded-xl text-xs font-bold transition-all flex items-center gap-1.5 cursor-pointer whitespace-nowrap {activeSubtab === 'pulse' ? 'bg-indigo-600 text-white shadow-sm' : 'bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
    >
      <span>📊</span>
      <span>Financial Pulse & Spend</span>
    </button>

    <button
      type="button"
      on:click={() => (activeSubtab = 'settle')}
      class="px-4 py-2 rounded-xl text-xs font-bold transition-all flex items-center gap-1.5 cursor-pointer whitespace-nowrap {activeSubtab === 'settle' ? 'bg-indigo-600 text-white shadow-sm' : 'bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
    >
      <span>⚖️</span>
      <span>Reimbursements & Settle Up</span>
    </button>

    {#if $jointAccountEnabled}
      <button
        type="button"
        on:click={() => (activeSubtab = 'joint')}
        class="px-4 py-2 rounded-xl text-xs font-bold transition-all flex items-center gap-1.5 cursor-pointer whitespace-nowrap {activeSubtab === 'joint' ? 'bg-indigo-600 text-white shadow-sm' : 'bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
      >
        <span>🏦</span>
        <span>Joint Account & Deposits</span>
      </button>
    {/if}

    <button
      type="button"
      on:click={() => (activeSubtab = 'budgets')}
      class="px-4 py-2 rounded-xl text-xs font-bold transition-all flex items-center gap-1.5 cursor-pointer whitespace-nowrap {activeSubtab === 'budgets' ? 'bg-indigo-600 text-white shadow-sm' : 'bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
    >
      <span>🎯</span>
      <span>Category Budgets & Health</span>
    </button>

    <button
      type="button"
      on:click={() => (activeSubtab = 'projections')}
      class="px-4 py-2 rounded-xl text-xs font-bold transition-all flex items-center gap-1.5 cursor-pointer whitespace-nowrap {activeSubtab === 'projections' ? 'bg-indigo-600 text-white shadow-sm' : 'bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
    >
      <span>📈</span>
      <span>Live Timeline & Projects</span>
    </button>
  </div>

  <!-- ── 3. SUBTAB CONTENTS ───────────────────────────────────────────────── -->

  <!-- ═══ SUBTAB 1: Financial Pulse & Spend ═════════════════════════════════ -->
  {#if activeSubtab === 'pulse'}
    <div class="space-y-6 animate-fadeIn">

      <!-- High-Level Financial Pulse KPI Cards -->
      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">

        <!-- 1. Total Household Income -->
        <div class="card p-4 sm:p-5 flex flex-col justify-between">
          <div>
            <div class="flex items-center justify-between gap-2 mb-2">
              <p class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">Household Income</p>
              <span class="badge-indigo">Income</span>
            </div>
            <p class="font-bold text-neutral-900 dark:text-white tabular-nums truncate text-[clamp(1.25rem,3.5vw,1.75rem)]">
              <span class:privacy-masked={$privacyShield}>{hasIncomeData ? fmt(totalIncomeEuros, true) : '—'}</span>
            </p>
          </div>
          <div class="mt-3 pt-2.5 border-t border-neutral-200 dark:border-neutral-800/80 flex items-center justify-between text-[11px]">
            {#if hasIncomeData}
              <div class="flex items-center gap-2 truncate">
                {#each userIncomeList as u}
                  <span class="truncate" style="color: {u.color}"><span class:privacy-masked={$privacyShield}>{u.name}: {fmt(u.totalCents / 100, true)}</span></span>
                {/each}
              </div>
            {:else}
              <span class="text-neutral-500">Configure salary on Income tab</span>
            {/if}
          </div>
        </div>

        <!-- 2. Monthly Total Spend (Matches test expectations) -->
        {#if $dashboardWidgets.monthlyTotal !== false}
          <div class="card p-4 sm:p-5 flex flex-col justify-between">
            <div>
              <div class="flex items-center justify-between gap-2 mb-2">
                <p class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">Monthly Total</p>
                <span class="badge-amber">Expenses</span>
              </div>
              <p class="font-bold text-neutral-900 dark:text-white tabular-nums truncate text-[clamp(1.25rem,3.5vw,1.75rem)]">
                <span class:privacy-masked={$privacyShield}>{fmt(total, true)}</span>
              </p>
            </div>
            <div class="mt-3 pt-2.5 border-t border-neutral-200 dark:border-neutral-800/80 flex items-center justify-between text-[11px] text-neutral-500">
              <span>This calendar month</span>
              <span class="text-neutral-700 dark:text-neutral-400 font-medium">{$analytics.monthly_total?.expense_count ?? 0} logged</span>
            </div>
          </div>
        {/if}

        <!-- 3. Net Savings / Cash Flow -->
        {#if $dashboardWidgets.cashFlowSavings !== false}
          <div class="card p-4 sm:p-5 flex flex-col justify-between">
            <div>
              <div class="flex items-center justify-between gap-2 mb-2">
                <p class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">Net Cash Flow</p>
                {#if hasIncomeData}
                  {#if netCashFlow >= 0}
                    <span class="badge-emerald">+{savingsRatePct.toFixed(0)}% Saved</span>
                  {:else}
                    <span class="badge-red">Deficit</span>
                  {/if}
                {:else}
                  <span class="badge-neutral">Spend Only</span>
                {/if}
              </div>
              <p class="font-bold tabular-nums truncate text-[clamp(1.25rem,3.5vw,1.75rem)] {hasIncomeData ? (netCashFlow >= 0 ? 'text-emerald-600 dark:text-emerald-400' : 'text-rose-600 dark:text-red-400') : 'text-neutral-800 dark:text-neutral-300'}">
                <span class:privacy-masked={$privacyShield}>{hasIncomeData ? (netCashFlow >= 0 ? `+${fmt(netCashFlow, true)}` : fmt(netCashFlow, true)) : (total > 0 ? `-${fmt(total, true)}` : '—')}</span>
              </p>
            </div>
            <div class="mt-3 pt-2.5 border-t border-neutral-200 dark:border-neutral-800/80 flex items-center justify-between text-[11px] text-neutral-500">
              <span>{hasIncomeData ? (netCashFlow >= 0 ? 'Surplus retained' : 'Over monthly income') : 'Income not recorded'}</span>
            </div>
          </div>

          <!-- 4. Household Savings Rate -->
          <div class="card p-4 sm:p-5 flex flex-col justify-between">
            <div>
              <div class="flex items-center justify-between gap-2 mb-2">
                <p class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">Savings Rate</p>
                <span class="badge-emerald">{hasIncomeData ? `${savingsRatePct.toFixed(0)}%` : 'Target 20%+'}</span>
              </div>
              <p class="font-bold text-neutral-900 dark:text-white tabular-nums truncate text-[clamp(1.25rem,3.5vw,1.75rem)]">
                <span class:privacy-masked={$privacyShield}>{hasIncomeData ? `${savingsRatePct.toFixed(1)}%` : '—'}</span>
              </p>
            </div>
            <div class="mt-3 pt-2.5 border-t border-neutral-200 dark:border-neutral-800/80 flex items-center justify-between text-[11px] text-neutral-500">
              <span>{hasIncomeData ? (savingsRatePct >= 20 ? 'Strong savings velocity' : 'Moderate savings rate') : 'Track income to calculate'}</span>
            </div>
          </div>
        {/if}

      </div>

      <!-- Per-Payer Detailed Cards Grid -->
      {#if $dashboardWidgets.payerBreakdown !== false}
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {#each payerRows as row}
            {@const color = userColor(row.who_paid)}
            <div class="card p-4 sm:p-5 transition-all hover:border-neutral-300 dark:hover:border-neutral-700" style="border-color:{color}50">
              <div class="flex items-center justify-between gap-2 mb-2">
                <p class="text-xs font-semibold uppercase tracking-wider" style="color:{color}">{row.who_paid}</p>
                <span class="text-[10px] font-bold px-2 py-0.5 rounded-full" style="background-color:{color}15; color:{color}; border:1px solid {color}40">
                  {pct(row.total_amount, total)} of total
                </span>
              </div>
              <p class="font-bold tabular-nums truncate text-[clamp(1.25rem,4vw,1.875rem)]" style="color:{color}">
                <span class:privacy-masked={$privacyShield}>{fmt(row.total_amount, true)}</span>
              </p>
              <p class="text-xs text-neutral-500 mt-1">{pct(row.total_amount, total)} of total spend</p>
            </div>
          {/each}
        </div>
      {/if}

      <!-- ── Flexible Multi-Chart & Category Analytics Area ─────────────────── -->
      {#if $dashboardWidgets.categoryChart !== false}
      <div class="grid grid-cols-1 lg:grid-cols-5 gap-6">

        <!-- Left: Interactive Chart Viewer -->
        <div class="lg:col-span-3 card p-5 sm:p-6 space-y-4">
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
            <div>
              <h2 class="text-sm font-semibold text-neutral-800 dark:text-neutral-200">
                {chartMetric === 'payer' ? 'Spending by Payer' : chartMetric === 'tag' ? 'Spending by Tag' : 'Spend by Category'}
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

          <div class="relative h-64 sm:h-72" class:hidden={categories.length === 0}>
            <canvas bind:this={doughnutCanvas} id="category-doughnut-chart"></canvas>
          </div>

          {#if categories.length === 0}
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

        <!-- Right: Category Spend Breakdown Table -->
        <div class="lg:col-span-2 card p-5 space-y-3">
          <div class="flex items-center justify-between">
            <h3 class="text-xs font-bold text-neutral-700 dark:text-neutral-300 uppercase tracking-wider">Categories ({categories.length})</h3>
            <input
              type="text"
              bind:value={categoryFilterText}
              placeholder="Search category…"
              class="input-field py-1 px-2.5 text-xs max-w-[140px]"
            />
          </div>

          <div class="space-y-2 max-h-[300px] overflow-y-auto pr-1">
            {#each filteredCategoryRows as row, i}
              {@const share = total > 0 ? (row.total_amount / total) * 100 : 0}
              <div class="p-2.5 rounded-xl bg-neutral-50 dark:bg-neutral-950/80 border border-neutral-200 dark:border-neutral-800/80 space-y-1.5">
                <div class="flex items-center justify-between text-xs">
                  <div class="flex items-center gap-1.5 min-w-0">
                    <span class="w-2.5 h-2.5 rounded-full flex-none" style="background:{PALETTE[i % PALETTE.length]}"></span>
                    <span class="text-neutral-800 dark:text-neutral-200 font-medium truncate">{row.category}</span>
                    <span class="text-[10px] text-neutral-500">({row.expense_count})</span>
                  </div>
                  <span class="text-neutral-900 dark:text-white font-bold tabular-nums">{fmt(row.total_amount)}</span>
                </div>
                <div class="w-full h-1.5 bg-neutral-200 dark:bg-neutral-800 rounded-full overflow-hidden">
                  <div class="h-full rounded-full bg-indigo-500 transition-all duration-500" style="width: {share}%"></div>
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
  {#if activeSubtab === 'settle'}
    <div class="space-y-6 animate-fadeIn">
      <PaybackVisual />
    </div>
  {/if}

  <!-- ═══ SUBTAB 3: Joint Account & Deposit Adjustments ═════════════════════ -->
  {#if activeSubtab === 'joint' && $jointAccountEnabled}
    <div class="space-y-6 animate-fadeIn">
      {#if $jointDashboard || $jointAccount}
        {@const dash = $jointDashboard}
        {@const ja = $jointAccount}
        <div class="card border-indigo-500/30 bg-indigo-50/50 dark:bg-indigo-500/5 p-5 sm:p-6 space-y-5">
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
            <div>
              <div class="flex items-center gap-2">
                <span class="text-xl">🏦</span>
                <h2 class="text-base font-bold text-neutral-900 dark:text-white">Household Joint Account Health</h2>
              </div>
              <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">
                Accurate measures in joint spending, target deposit calculations, and point-in-time safety buffer
              </p>
            </div>
            {#if ja}
              <div class="flex items-center gap-2 text-xs bg-white dark:bg-neutral-900/90 px-3.5 py-2 rounded-xl border border-indigo-500/30 flex-none shadow-sm">
                <span class="text-neutral-500 dark:text-neutral-400">Current Balance:</span>
                <span class="font-bold text-neutral-900 dark:text-white tabular-nums">{fmt(ja.balance_cents / 100)}</span>
              </div>
            {/if}
          </div>

          {#if dash}
            <!-- Projected vs Actual Progress Cards -->
            <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
              <div class="card-sub space-y-1 p-4">
                <p class="text-[11px] font-medium text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">Spent ({dash.month})</p>
                <p class="text-xl font-bold text-neutral-900 dark:text-white tabular-nums">{fmt(dash.actual_total_cents / 100)}</p>
                <p class="text-[11px] text-neutral-500">Target: {fmt(dash.expected_total_cents / 100)}</p>
              </div>

              <div class="card-sub border-indigo-300 dark:border-indigo-500/40 bg-indigo-100/60 dark:bg-indigo-950/40 space-y-1 p-4">
                <p class="text-[11px] font-semibold text-indigo-700 dark:text-indigo-300 uppercase tracking-wider">Min Deposit for Next Period</p>
                <p class="text-xl font-bold text-indigo-900 dark:text-indigo-200 tabular-nums">{fmt(dash.target_deposit_cents / 100)}</p>
                <p class="text-[11px] text-indigo-600 dark:text-indigo-400/80">Expected costs + {dash.safety_margin_pct}% safety margin</p>
              </div>

              <div class="card-sub space-y-1 p-4">
                <p class="text-[11px] font-medium text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">Deposits Received</p>
                <p class="text-xl font-bold text-neutral-900 dark:text-white tabular-nums">{fmt(dash.total_deposits_cents / 100)}</p>
                {#if dash.deposit_status === 'target_met' || dash.total_deposits_cents >= dash.target_deposit_cents}
                  <p class="text-[11px] font-semibold text-emerald-600 dark:text-emerald-400">✓ Target met</p>
                {:else if dash.deposit_status === 'pending'}
                  <p class="text-[11px] font-semibold text-indigo-600 dark:text-indigo-400 flex items-center gap-1">
                    <span>⏳ Pending</span>
                    {#if dash.pending_due_day}
                      <span class="text-neutral-500 dark:text-neutral-400 font-normal">(due day {dash.pending_due_day})</span>
                    {/if}
                  </p>
                {:else}
                  <p class="text-[11px] font-semibold text-amber-600 dark:text-amber-400">⚠ Below target</p>
                {/if}
              </div>
            </div>

            {#if ja && ja.balance_cents < dash.target_deposit_cents}
              <div class="px-4 py-3 bg-indigo-100/80 dark:bg-indigo-950/50 border border-indigo-300 dark:border-indigo-800/60 rounded-xl flex items-center justify-between text-xs text-indigo-800 dark:text-indigo-300">
                <span>💡 <strong>Deposit Adjustment Needed:</strong> Top up at least <strong>{fmt((dash.target_deposit_cents - ja.balance_cents) / 100)}</strong> to maintain the agreed {dash.safety_margin_pct}% safety margin.</span>
              </div>
            {/if}
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  <!-- ═══ SUBTAB 4: Category Budgets & Health ════════════════════════════════ -->
  {#if activeSubtab === 'budgets'}
    <div class="space-y-6 animate-fadeIn">
      <div class="card p-5 sm:p-6 space-y-4">
        <div class="flex items-center justify-between">
          <div>
            <h2 class="text-sm font-semibold text-neutral-800 dark:text-neutral-200">Monthly Budget Limits & Spending Health</h2>
            <p class="text-xs text-neutral-500 mt-0.5">Live tracking of actual expenses vs monthly category budget caps</p>
          </div>
          <span class="badge-indigo">{$selectedMonth}</span>
        </div>

        {#if budgetStatus.length > 0}
          <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
            {#each budgetStatus.filter((r) => r.limit_cents > 0) as row}
              {@const color = row.pct_used >= 90 ? 'red' : row.pct_used >= 70 ? 'yellow' : 'green'}
              {@const barColor = color === 'red' ? 'bg-red-500' : color === 'yellow' ? 'bg-yellow-400' : 'bg-emerald-500'}
              {@const textColor = color === 'red' ? 'text-rose-600 dark:text-red-400' : color === 'yellow' ? 'text-amber-600 dark:text-yellow-400' : 'text-emerald-600 dark:text-emerald-400'}
              {@const isStanding = !row.budget_month || row.budget_month === 'ALL'}
              {@const remainingCents = Math.max(0, row.limit_cents - row.actual_cents)}

              <div class="card-sub p-3.5 space-y-2">
                <div class="flex items-start justify-between gap-1">
                  <p class="text-xs text-neutral-800 dark:text-neutral-300 font-semibold uppercase truncate">{row.category}</p>
                  {#if isStanding}
                    <span class="text-[9px] font-semibold uppercase tracking-wide text-neutral-600 dark:text-neutral-400 bg-neutral-200 dark:bg-neutral-800 rounded px-1.5 py-0.5 leading-none">standing</span>
                  {:else}
                    <span class="text-[9px] font-semibold uppercase tracking-wide text-indigo-700 dark:text-indigo-400 bg-indigo-100 dark:bg-indigo-950/80 rounded px-1.5 py-0.5 leading-none">this month</span>
                  {/if}
                </div>

                <div class="flex items-baseline justify-between text-xs">
                  <p class="font-bold text-neutral-900 dark:text-neutral-200 tabular-nums">
                    {fmt(row.actual_cents / 100)} <span class="text-neutral-500 font-normal">/ {fmt(row.limit_cents / 100)}</span>
                  </p>
                  <span class="font-bold {textColor} tabular-nums">{row.pct_used.toFixed(0)}%</span>
                </div>

                <div class="w-full h-2 bg-neutral-200 dark:bg-neutral-800 rounded-full overflow-hidden">
                  <div class="h-full rounded-full {barColor} transition-all duration-500" style="width: {Math.min(row.pct_used, 100)}%"></div>
                </div>

                <p class="text-[10px] text-neutral-500">
                  {remainingCents > 0 ? `Remaining buffer: ${fmt(remainingCents / 100)}` : '⚠ Budget limit exceeded'}
                </p>
              </div>
            {/each}
          </div>
        {:else}
          <div class="empty-state-box py-8">
            <p class="text-neutral-700 dark:text-neutral-300 text-sm font-semibold">No category budgets configured.</p>
            <p class="text-neutral-500 text-xs mt-1">Configure limits on the Categories / Recurring tab.</p>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- ═══ SUBTAB 5: Live Timeline & Projects ════════════════════════════════ -->
  {#if activeSubtab === 'projections'}
    <div class="space-y-6 animate-fadeIn">

      <!-- Live Expense Timeline Chart -->
      <div class="card p-5 sm:p-6 space-y-4">
        <div class="flex items-center justify-between">
          <div>
            <h2 class="text-sm font-semibold text-neutral-800 dark:text-neutral-200">Expense Timeline</h2>
            <p class="text-xs text-neutral-500 mt-0.5">Real-time daily transaction velocity for {$selectedMonth}</p>
          </div>
          <span class="flex items-center gap-1.5 text-xs text-emerald-600 dark:text-emerald-400 font-medium">
            <span class="w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></span>
            Live
          </span>
        </div>
        <RealtimeChart />
      </div>

      <!-- Income By Person Chart -->
      <div class="card p-5 sm:p-6 space-y-4">
        <div class="flex items-center justify-between">
          <div>
            <h2 class="text-sm font-semibold text-neutral-800 dark:text-neutral-200">Income by Person</h2>
            <p class="text-xs text-neutral-500 mt-0.5">Effective monthly salary & one-off earnings</p>
          </div>
        </div>
        <IncomeChart />
      </div>

      <!-- Savings Projects Overview -->
      {#if $projects.length > 0}
        <div class="card p-5 sm:p-6 space-y-4">
          <div class="flex items-center justify-between">
            <div>
              <h2 class="text-sm font-semibold text-neutral-800 dark:text-neutral-200">Target Savings Projects</h2>
              <p class="text-xs text-neutral-500 mt-0.5">Milestone goals progress</p>
            </div>
          </div>
          <div class="space-y-3">
            {#each $projects as project (project.id)}
              {@const progress = Math.min(100, Math.round((project.total_spent_cents / project.target_cents) * 100))}
              {@const isComplete = project.total_spent_cents >= project.target_cents}
              <div class="card-sub p-3.5 flex items-center gap-4">
                <div class="flex-1 min-w-0">
                  <div class="flex justify-between items-baseline mb-1">
                    <span class="text-xs font-semibold text-neutral-800 dark:text-neutral-200 truncate">{project.name}</span>
                    <span class="text-xs tabular-nums {isComplete ? 'text-emerald-600 dark:text-emerald-400 font-bold' : 'text-neutral-500 dark:text-neutral-400'} ml-2 flex-none">{progress}%</span>
                  </div>
                  <div class="w-full h-2 bg-neutral-200 dark:bg-neutral-800 rounded-full overflow-hidden">
                    <div
                      class="h-full rounded-full bg-gradient-to-r {isComplete ? 'from-emerald-500 to-emerald-400' : progress >= 60 ? 'from-indigo-500 to-violet-500' : progress >= 30 ? 'from-sky-600 to-indigo-500' : 'from-sky-700 to-sky-500'}"
                      style="width: {progress}%"
                    ></div>
                  </div>
                </div>
                <div class="text-right flex-none">
                  <p class="text-xs font-bold text-neutral-900 dark:text-neutral-100 tabular-nums">
                    {fmt(project.total_spent_cents / 100)}
                    <span class="text-neutral-500 font-normal">/ {fmt(project.target_cents / 100)}</span>
                  </p>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}

    </div>
  {/if}

</div>
