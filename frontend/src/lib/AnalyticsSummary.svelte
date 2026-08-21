<script>
  /**
   * AnalyticsSummary.svelte
   *
   * Household Financial Pulse & Category Breakdown:
   *   1. Top Scope Switcher: Radio buttons for All Household / Individual Users / Joint Accounts
   *   2. High-level Household Financial Pulse (Total Income, Total Spend, Net Saved & Savings Rate %)
   *   3. Individual payer spending cards
   *   4. Joint Account status pulse (when joint module enabled)
   *   5. Category spend visualization (Doughnut / Bar chart switcher)
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
  } from './stores.js';
  import { fetchAnalytics, fetchIncomeByPerson } from './api.js';
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

  /** Creates (or recreates) the Chart.js instance for the given chart type. */
  function createChart(type) {
    if (!doughnutCanvas) return;
    if (doughnutChart) { doughnutChart.destroy(); doughnutChart = null; }
    const ctx = doughnutCanvas.getContext('2d');
    const isBar = (type === 'bar');
    doughnutChart = new Chart(ctx, {
      type: isBar ? 'bar' : 'doughnut',
      data: {
        labels:   categories.map((c) => c.category),
        datasets: [
          {
            data:            categories.map((c) => c.total_amount),
            backgroundColor: PALETTE,
            borderColor:     isBar ? 'transparent' : '#0a0a14',
            borderWidth:     isBar ? 0 : 3,
            hoverOffset:     isBar ? 0 : 6,
            borderRadius:    isBar ? 6 : 0,
          },
        ],
      },
      options: {
        responsive:          true,
        maintainAspectRatio: false,
        ...(isBar ? {} : { cutout: '70%' }),
        indexAxis: isBar ? 'y' : undefined,
        animation: { duration: 600, easing: 'easeInOutQuart' },
        scales: isBar ? {
          x: {
            grid: { color: 'rgba(255,255,255,0.04)' },
            ticks: { color: '#6b7280', font: { family: 'Inter, system-ui, sans-serif', size: 11 } },
          },
          y: {
            grid: { display: false },
            ticks: { color: '#9ca3af', font: { family: 'Inter, system-ui, sans-serif', size: 11 } },
          },
        } : undefined,
        plugins: {
          legend: isBar ? { display: false } : {
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
              label: (ctx) => ` ${$currencySymbol}${Number(ctx.raw).toFixed(2)}`,
            },
          },
        },
      },
    });
  }

  onMount(() => {
    createChart($chartStyle);
  });

  /** Recreate chart when user toggles chart style in settings. */
  let prevChartStyle = $chartStyle;
  $: if ($chartStyle !== prevChartStyle) {
    prevChartStyle = $chartStyle;
    createChart($chartStyle);
  }

  onDestroy(() => {
    unsubscribe();
    if (doughnutChart) doughnutChart.destroy();
  });

  function fmt(n) {
    return `${$currencySymbol}${Number(n).toFixed(2)}`;
  }

  function pct(part, whole) {
    if (!whole || whole === 0) return '—';
    return `${((part / whole) * 100).toFixed(0)}%`;
  }

  // ── Household Income & Net Cash Flow Calculations ─────────────────────────
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

  $: activeUsers = $users.filter((u) => u.is_active !== false);

  // ── Multi-Scope filter reactivity ─────────────────────────────────────────
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

  $: {
    let usersParam = null;
    if (!isEveryoneSelected) {
      const userSet = new Set();
      for (const token of selectedTokens) {
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
      if (userSet.size > 0) {
        usersParam = Array.from(userSet);
      }
    }
    fetchAnalytics($selectedMonth, usersParam);
    fetchIncomeByPerson($selectedMonth, usersParam);
  }

  $: filteredActiveUsers = activeUsers.filter((u) => {
    if (isEveryoneSelected) return true;
    const userSet = new Set();
    for (const token of selectedTokens) {
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
    return userSet.has(u.name);
  });

  $: userIncomeList = filteredActiveUsers.map((u) => {
    const userJobs = $jobs.filter((j) => j.who === u.name && isJobActiveInMonth(j, $selectedMonth));
    let baseSalaryCents = userJobs.reduce((sum, j) => sum + toMonthlyEquivalent(j.amount_cents, j.frequency), 0);
    if (userJobs.length === 0) {
      const row = $incomeAnalytics.find((r) => r.who === u.name);
      if (row) baseSalaryCents = row.total_cents;
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
</script>

<!-- ── Dashboard Scope Switcher (Multi-Select Filter) ───────────────────────── -->
<div class="card p-3 sm:p-4 mb-6 border-neutral-800 bg-neutral-900/80 backdrop-blur">
  <div class="flex flex-col md:flex-row md:items-center justify-between gap-3">
    <div class="flex items-center gap-2">
      <span class="text-sm font-semibold text-neutral-200">Dashboard View:</span>
      <span class="text-xs text-neutral-500 hidden sm:inline">
        {#if isEveryoneSelected}
          Showing all household data
        {:else}
          Filtered to: {filteredActiveUsers.map((u) => u.name).join(', ') || 'Selected members'}
        {/if}
      </span>
    </div>

    <div class="flex flex-wrap items-center gap-1.5 p-1 bg-neutral-950/70 rounded-xl border border-neutral-800/80" role="group" aria-label="Dashboard Scope">
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
        <span class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all {isEveryoneSelected ? 'bg-indigo-600 text-white shadow-sm ring-1 ring-indigo-400' : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800/50'}">
          <span>👥</span>
          <span>Everyone</span>
          {#if isEveryoneSelected}
            <span class="text-[10px] text-white">✓</span>
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
          <span class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all {isChecked ? 'bg-indigo-600 text-white shadow-sm ring-1 ring-indigo-400' : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800/50'}">
            <span class="w-2 h-2 rounded-full flex-none" style="background-color: {u.color}"></span>
            <span>👤 {u.name}</span>
            {#if isChecked}
              <span class="text-[10px] text-white">✓</span>
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
              <span class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all {isChecked ? 'bg-indigo-600 text-white shadow-sm ring-1 ring-indigo-400' : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800/50'}">
                <span>🏦</span>
                <span>{ja.name}</span>
                {#if ja.member_names && ja.member_names.length > 0}
                  <span class="text-[10px] opacity-75">({ja.member_names.join(' & ')})</span>
                {/if}
                {#if isChecked}
                  <span class="text-[10px] text-white">✓</span>
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
            <span class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all {isChecked ? 'bg-indigo-600 text-white shadow-sm ring-1 ring-indigo-400' : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800/50'}">
              <span>🏦</span>
              <span>{$jointAccount.name}</span>
              {#if isChecked}
                <span class="text-[10px] text-white">✓</span>
              {/if}
            </span>
          </label>
        {/if}
      {/if}
    </div>
  </div>
</div>

<!-- ── Household Financial Pulse (Top Summary Cards) ────────────────────────── -->
<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">

  <!-- 1. Total Household Income -->
  <div class="card p-4 sm:p-5 min-w-0 flex flex-col justify-between border-neutral-800 bg-neutral-900/90">
    <div>
      <div class="flex items-center justify-between gap-2 mb-2">
        <p class="text-xs font-semibold text-neutral-400 uppercase tracking-wider">Household Income</p>
        <span class="badge-indigo">Income</span>
      </div>
      <p class="font-bold text-white tabular-nums truncate text-[clamp(1.25rem,3.5vw,1.75rem)]">
        {hasIncomeData ? fmt(totalIncomeEuros) : '—'}
      </p>
    </div>
    <div class="mt-3 pt-2.5 border-t border-neutral-800/80 flex items-center justify-between text-[11px]">
      {#if hasIncomeData}
        <div class="flex items-center gap-2 truncate">
          {#each userIncomeList as u}
            <span class="truncate" style="color: {u.color}">{u.name}: {fmt(u.totalCents / 100)}</span>
          {/each}
        </div>
      {:else}
        <span class="text-neutral-500">Configure salary on Income tab</span>
      {/if}
    </div>
  </div>

  <!-- 2. Monthly Total Spend (Matches existing ID/test structure) -->
  <div class="card p-4 sm:p-5 min-w-0 flex flex-col justify-between border-neutral-800 bg-neutral-900/90">
    <div>
      <div class="flex items-center justify-between gap-2 mb-2">
        <p class="text-xs font-semibold text-neutral-400 uppercase tracking-wider">Monthly Total</p>
        <span class="badge-amber">Expenses</span>
      </div>
      <p class="font-bold text-white tabular-nums truncate text-[clamp(1.25rem,3.5vw,1.75rem)]">{fmt(total)}</p>
    </div>
    <div class="mt-3 pt-2.5 border-t border-neutral-800/80 flex items-center justify-between text-[11px] text-neutral-500">
      <span>This calendar month</span>
      <span class="text-neutral-400 font-medium">{$analytics.monthly_total?.expense_count ?? 0} logged</span>
    </div>
  </div>

  <!-- 3. Net Savings / Cash Flow -->
  <div class="card p-4 sm:p-5 min-w-0 flex flex-col justify-between border-neutral-800 bg-neutral-900/90">
    <div>
      <div class="flex items-center justify-between gap-2 mb-2">
        <p class="text-xs font-semibold text-neutral-400 uppercase tracking-wider">Net Cash Flow</p>
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
      <p class="font-bold tabular-nums truncate text-[clamp(1.25rem,3.5vw,1.75rem)] {hasIncomeData ? (netCashFlow >= 0 ? 'text-emerald-400' : 'text-red-400') : 'text-neutral-300'}">
        {hasIncomeData ? (netCashFlow >= 0 ? `+${fmt(netCashFlow)}` : fmt(netCashFlow)) : (total > 0 ? `-${fmt(total)}` : '—')}
      </p>
    </div>
    <div class="mt-3 pt-2.5 border-t border-neutral-800/80 flex items-center justify-between text-[11px] text-neutral-500">
      <span>{hasIncomeData ? (netCashFlow >= 0 ? 'Surplus retained' : 'Over monthly income') : 'Income not recorded'}</span>
    </div>
  </div>

  <!-- 4. Household Savings Rate -->
  <div class="card p-4 sm:p-5 min-w-0 flex flex-col justify-between border-neutral-800 bg-neutral-900/90">
    <div>
      <div class="flex items-center justify-between gap-2 mb-2">
        <p class="text-xs font-semibold text-neutral-400 uppercase tracking-wider">Savings Rate</p>
        <span class="badge-emerald">{hasIncomeData ? `${savingsRatePct.toFixed(0)}%` : 'Target 20%+'}</span>
      </div>
      <p class="font-bold text-white tabular-nums truncate text-[clamp(1.25rem,3.5vw,1.75rem)]">
        {hasIncomeData ? `${savingsRatePct.toFixed(1)}%` : '—'}
      </p>
    </div>
    <div class="mt-3 pt-2.5 border-t border-neutral-800/80 flex items-center justify-between text-[11px] text-neutral-500">
      <span>{hasIncomeData ? (savingsRatePct >= 20 ? 'Strong savings velocity' : 'Moderate savings rate') : 'Track income to calculate'}</span>
    </div>
  </div>

</div>

<!-- ── Per-Payer Detailed Cards Grid (Backward Compatible for test suites) ─────── -->
<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 mb-6">
  {#each payerRows as row}
    {@const color = userColor(row.who_paid)}
    <div class="card p-4 sm:p-5 min-w-0 transition-all hover:border-neutral-700" style="border-color:{color}40">
      <div class="flex items-center justify-between gap-2 mb-2">
        <p class="text-xs font-semibold uppercase tracking-wider" style="color:{color}">{row.who_paid}</p>
        <span class="text-[10px] font-bold px-2 py-0.5 rounded-full" style="background-color:{color}15; color:{color}; border:1px solid {color}40">
          {pct(row.total_amount, total)} of total
        </span>
      </div>
      <p class="font-bold tabular-nums truncate text-[clamp(1.25rem,4vw,1.875rem)]" style="color:{color}">{fmt(row.total_amount)}</p>
      <p class="text-xs text-neutral-500 mt-1">{pct(row.total_amount, total)} of total spend</p>
    </div>
  {/each}
</div>

{#if $jointAccountEnabled && ($jointDashboard || $jointAccount)}
  {@const dash = $jointDashboard}
  {@const ja = $jointAccount}
  <div class="card border-indigo-500/30 bg-indigo-500/5 p-5 sm:p-6 mb-6 space-y-4">
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
      <div>
        <div class="flex items-center gap-2">
          <span class="text-lg">🏦</span>
          <h2 class="text-base font-bold text-white">Joint Account — Overview</h2>
        </div>
        <p class="text-xs text-neutral-400 mt-0.5">Shared household spending & monthly target progression</p>
      </div>
      {#if ja}
        <div class="flex items-center gap-2 text-xs bg-neutral-900/90 px-3.5 py-2 rounded-xl border border-indigo-500/30 flex-none">
          <span class="text-neutral-400">Balance:</span>
          <span class="font-bold text-white tabular-nums">{fmt(ja.balance_cents / 100)}</span>
        </div>
      {/if}
    </div>

    {#if dash}
      <!-- Projected vs Actual progress -->
      <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
        <div class="card-sub space-y-1">
          <p class="text-[11px] font-medium text-neutral-400 uppercase tracking-wider">Spent ({dash.month})</p>
          <p class="text-lg font-bold text-white tabular-nums">{fmt(dash.actual_total_cents / 100)}</p>
          <p class="text-[11px] text-neutral-500">Target: {fmt(dash.expected_total_cents / 100)}</p>
        </div>
        <div class="card-sub border-indigo-500/40 bg-indigo-950/30 space-y-1">
          <p class="text-[11px] font-semibold text-indigo-300 uppercase tracking-wider">Min Deposit for Next Period</p>
          <p class="text-lg font-bold text-indigo-200 tabular-nums">{fmt(dash.target_deposit_cents / 100)}</p>
          <p class="text-[11px] text-indigo-400/80">Covers expected costs + {dash.safety_margin_pct}% margin</p>
        </div>
        <div class="card-sub space-y-1">
          <p class="text-[11px] font-medium text-neutral-400 uppercase tracking-wider">Deposits Received</p>
          <p class="text-lg font-bold text-white tabular-nums">{fmt(dash.total_deposits_cents / 100)}</p>
          {#if dash.deposit_status === 'target_met' || dash.total_deposits_cents >= dash.target_deposit_cents}
            <p class="text-[11px] font-semibold text-emerald-400">✓ Target met</p>
          {:else if dash.deposit_status === 'pending'}
            <p class="text-[11px] font-semibold text-indigo-400 flex items-center gap-1">
              <span>⏳ Pending</span>
              {#if dash.pending_due_day}
                <span class="text-neutral-400 font-normal">(due day {dash.pending_due_day})</span>
              {/if}
            </p>
          {:else}
            <p class="text-[11px] font-semibold text-amber-400">⚠ Below target</p>
          {/if}
        </div>
      </div>

      {#if ja && ja.balance_cents < dash.target_deposit_cents}
        <div class="px-3.5 py-2.5 bg-indigo-950/40 border border-indigo-800/40 rounded-xl flex items-center justify-between text-xs text-indigo-300">
          <span>💡 <strong>Next Period Minimum Deposit:</strong> Deposit at least <strong>{fmt(dash.target_deposit_cents / 100)}</strong> before next period to fully cover expected joint costs.</span>
        </div>
      {/if}
    {/if}
  </div>
{/if}

<!-- ── Category doughnut ──────────────────────────────────────────────────── -->
<div class="card p-5 sm:p-6">
  <div class="flex items-center justify-between mb-4">
    <div>
      <h2 class="text-sm font-semibold text-neutral-200">Spend by Category</h2>
      <p class="text-xs text-neutral-500 mt-0.5">Household expense breakdown</p>
    </div>
  </div>

  <div class="relative h-56" class:hidden={categories.length === 0}>
    <canvas bind:this={doughnutCanvas} id="category-doughnut-chart"></canvas>
  </div>

  {#if categories.length === 0}
    <div class="empty-state-box my-4">
      <div class="w-12 h-12 rounded-2xl bg-neutral-800/80 flex items-center justify-center mb-2">
        <svg class="w-6 h-6 text-neutral-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
          <path stroke-linecap="round" stroke-linejoin="round"
            d="M10.5 6a7.5 7.5 0 107.5 7.5h-7.5V6z" />
          <path stroke-linecap="round" stroke-linejoin="round"
            d="M13.5 10.5H21A7.5 7.5 0 0013.5 3v7.5z" />
        </svg>
      </div>
      <p class="text-neutral-300 text-sm font-semibold">No category data yet.</p>
      <p class="text-neutral-500 text-xs max-w-xs mt-1">
        Log an expense on the Expenses tab and the breakdown will appear here.
      </p>
    </div>
  {:else}
    <!-- Category breakdown list -->
    <div class="mt-5 grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2.5">
      {#each categories as row, i}
        <div class="card-sub p-2.5 sm:p-3 flex items-center justify-between gap-2 text-xs">
          <div class="flex items-center gap-2 min-w-0 overflow-hidden">
            <span
              class="w-2.5 h-2.5 rounded-full flex-none"
              style="background:{PALETTE[i % PALETTE.length]}"
            ></span>
            <span class="text-neutral-200 font-medium truncate">{row.category}</span>
            <span class="text-neutral-500 text-[10px] flex-none">({row.expense_count})</span>
          </div>
          <span class="text-white font-bold tabular-nums flex-none">{fmt(row.total_amount)}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>
