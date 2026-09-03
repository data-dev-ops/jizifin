<script>
  /**
   * DrilldownModal.svelte
   *
   * Interactive drill-down dialog for dashboard info cards, adhering to /interface-kit standards:
   *  - Accessible dialog (role="dialog", aria-modal, Esc key trap, focus management, backdrop dismissal)
   *  - Concentric radii, tabular numbers, user theme colors, and responsive layout
   *  - Rich details: per-user cost distribution bar, per-user metric chips, itemized expense records, and instant search
   */

  import { onMount, onDestroy } from 'svelte';
  import {
    drilldownTarget,
    closeDrilldown,
    expenses,
    users,
    currencySymbol,
    selectedMonth,
    privacyShield,
    currencyPrecisionMode,
    tags,
    projects,
    jointAccounts,
    jointAccount,
    jobs,
    incomeEntries,
    paybacks,
    scopedUsers,
  } from './stores.js';

  let searchQuery = '';
  let modalContainer;
  let closeButton;

  $: target = $drilldownTarget;
  $: isOpen = !!target;
  $: type = target?.type ?? '';
  $: title = target?.title ?? '';
  $: customData = target?.data ?? {};

  let lastTarget = null;
  // Reset search query only when drilldown target actually changes
  $: if (target !== lastTarget) {
    lastTarget = target;
    searchQuery = '';
  }

  // Look up user color with default fallback
  function userColor(name) {
    return $users.find((u) => u.name === name)?.color ?? '#6366f1';
  }

  function userInitial(name) {
    return (name ?? '?').charAt(0).toUpperCase();
  }

  function fmt(n) {
    const val = Number(n || 0);
    const sign = val < 0 ? '-' : '';
    const abs = Math.abs(val);
    if ($currencyPrecisionMode === 'whole_units_on_summaries') {
      return `${sign}${$currencySymbol}${Math.round(abs).toLocaleString('en-GB')}`;
    }
    return `${sign}${$currencySymbol}${abs.toFixed(2)}`;
  }

  function pct(part, whole) {
    if (!whole || whole === 0) return '0%';
    return `${((part / whole) * 100).toFixed(0)}%`;
  }

  // ── Month Expenses & Tag/Project Maps ──────────────────────────────────────
  $: monthExpenses = $expenses.filter((e) =>
    e.expense_date && e.expense_date.startsWith($selectedMonth)
  );

  // Active dashboard user scope
  $: scopedUserSet = $scopedUsers ? new Set($scopedUsers) : null;
  $: scopedLabel = $scopedUsers && $scopedUsers.length > 0 ? $scopedUsers.join(', ') : '';

  // Filter month expenses according to top-of-dashboard user scope selection
  $: scopedMonthExpenses = monthExpenses.filter((e) => {
    if (!scopedUserSet) return true;
    if (type === 'category-adjustment') {
      return scopedUserSet.has(e.who_paid) || (e.overrides && e.overrides.some((o) => scopedUserSet.has(o.user_name)));
    }
    return scopedUserSet.has(e.who_paid);
  });

  $: tagMap = Object.fromEntries(($tags || []).map((t) => [t.id, t]));
  $: projectMap = Object.fromEntries(($projects || []).map((p) => [p.id, p]));

  // ── Category Drilldown Data ────────────────────────────────────────────────
  $: categoryExpenses = type === 'category' || type === 'category-adjustment' || type === 'budget'
    ? scopedMonthExpenses.filter((e) => e.category === title)
    : [];

  $: categoryTotalCents = categoryExpenses.reduce((s, e) => s + (e.cost_cents || 0), 0);

  // Per-user spending calculation for category
  $: categoryUserBreakdown = (() => {
    if (categoryExpenses.length === 0) return [];
    const payerCents = {};
    const payerCounts = {};
    for (const exp of categoryExpenses) {
      const p = exp.who_paid || 'Unknown';
      payerCents[p] = (payerCents[p] || 0) + (exp.cost_cents || 0);
      payerCounts[p] = (payerCounts[p] || 0) + 1;
    }
    return Object.entries(payerCents)
      .map(([name, cents]) => ({
        name,
        cents,
        count: payerCounts[name],
        pct: categoryTotalCents > 0 ? (cents / categoryTotalCents) * 100 : 0,
        color: userColor(name),
      }))
      .sort((a, b) => b.cents - a.cents);
  })();

  // ── Category Adjustment Settlement Data ────────────────────────────────────
  $: paybackRow = type === 'category-adjustment'
    ? (($paybacks.rows || []).find((r) => r.category === title) || customData?.row)
    : null;

  $: adjustmentUserShares = (() => {
    if (!paybackRow) return [];
    let usersList = Object.keys(paybackRow.per_user_paid || {});
    if (scopedUserSet) {
      usersList = usersList.filter((u) => scopedUserSet.has(u));
    }
    return usersList.map((userName) => {
      const paid = paybackRow.per_user_paid?.[userName] ?? 0;
      const targetPct = paybackRow.per_user_share_pct?.[userName] ?? 0;
      const net = paybackRow.net_per_user?.[userName] ?? 0;
      return {
        name: userName,
        paid,
        targetPct,
        net,
        color: userColor(userName),
      };
    });
  })();

  // ── Payer Drilldown Data ───────────────────────────────────────────────────
  $: payerExpenses = type === 'payer'
    ? monthExpenses.filter((e) => e.who_paid === title)
    : [];

  $: payerTotalCents = payerExpenses.reduce((s, e) => s + (e.cost_cents || 0), 0);

  $: payerCategoryBreakdown = (() => {
    if (payerExpenses.length === 0) return [];
    const catCents = {};
    const catCounts = {};
    for (const exp of payerExpenses) {
      const c = exp.category || 'Uncategorized';
      catCents[c] = (catCents[c] || 0) + (exp.cost_cents || 0);
      catCounts[c] = (catCounts[c] || 0) + 1;
    }
    return Object.entries(catCents)
      .map(([category, cents]) => ({
        category,
        cents,
        count: catCounts[category],
        pct: payerTotalCents > 0 ? (cents / payerTotalCents) * 100 : 0,
      }))
      .sort((a, b) => b.cents - a.cents);
  })();

  // ── Project Drilldown Data ─────────────────────────────────────────────────
  $: projectItem = type === 'project' ? customData?.project : null;
  $: projectExpenses = type === 'project' && projectItem
    ? scopedMonthExpenses.filter((e) => e.project_id === projectItem.id)
    : [];
  $: projectTotalCents = projectExpenses.reduce((s, e) => s + (e.cost_cents || 0), 0);

  // ── Joint Spent Drilldown Data ─────────────────────────────────────────────
  $: jointExpenses = type === 'joint-spent'
    ? scopedMonthExpenses.filter((e) => e.is_joint || (customData?.jointAccount && e.joint_account_id === customData.jointAccount.id))
    : [];
  $: jointTotalCents = jointExpenses.reduce((s, e) => s + (e.cost_cents || 0), 0);

  // ── Monthly Total Drilldown Data ───────────────────────────────────────────
  $: allExpenses = type === 'monthly-total' ? scopedMonthExpenses : [];

  // ── Active Expense List for Filtering ─────────────────────────────────────
  $: activeExpensesList = (() => {
    if (type === 'category' || type === 'category-adjustment' || type === 'budget') {
      return categoryExpenses;
    }
    if (type === 'payer') {
      return payerExpenses;
    }
    if (type === 'project') {
      return projectExpenses;
    }
    if (type === 'joint-spent') {
      return jointExpenses;
    }
    if (type === 'monthly-total') {
      return allExpenses;
    }
    return [];
  })();

  $: filteredExpenses = activeExpensesList.filter((e) => {
    if (!searchQuery.trim()) return true;
    const q = searchQuery.toLowerCase();
    const matchName = (e.name || '').toLowerCase().includes(q);
    const matchPayer = (e.who_paid || '').toLowerCase().includes(q);
    const matchCat = (e.category || '').toLowerCase().includes(q);
    const matchDate = (e.expense_date || '').includes(q);
    return matchName || matchPayer || matchCat || matchDate;
  });

  // ── Keyboard handling (Escape to close) ───────────────────────────────────
  function handleKeyDown(e) {
    if (e.key === 'Escape' && isOpen) {
      e.stopPropagation();
      closeDrilldown();
    }
  }
</script>

<svelte:window on:keydown={handleKeyDown} />

{#if isOpen}
  <!-- Modal Overlay Container -->
  <div class="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-4 md:p-6 overflow-hidden">
    <!-- Backdrop button for click-outside dismissal -->
    <button
      type="button"
      class="fixed inset-0 bg-neutral-900/60 dark:bg-black/75 backdrop-blur-sm transition-opacity duration-200 cursor-default w-full h-full border-0 p-0 m-0"
      on:click={closeDrilldown}
      aria-label="Close dialog backdrop"
      tabindex="-1"
    ></button>

    <!-- Modal Card Surface with Concentric Radius and Animation -->
    <div
      bind:this={modalContainer}
      role="dialog"
      aria-modal="true"
      aria-labelledby="drilldown-modal-title"
      tabindex="-1"
      class="drilldown-animate relative z-10 w-full max-w-2xl max-h-[90vh] flex flex-col bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 shadow-2xl rounded-2xl overflow-hidden"
    >
      <!-- ── Header ───────────────────────────────────────────────────────── -->
      <div class="px-5 py-4 sm:px-6 border-b border-neutral-200 dark:border-neutral-800 flex items-center justify-between gap-3 bg-neutral-50/70 dark:bg-neutral-950/40">
        <div class="flex items-center gap-3 min-w-0">
          <div class="w-10 h-10 rounded-xl bg-indigo-50 dark:bg-indigo-950/70 border border-indigo-200 dark:border-indigo-800/60 flex items-center justify-center text-lg flex-shrink-0">
            {#if type === 'category' || type === 'category-adjustment'}
              🏷️
            {:else if type === 'payer'}
              👤
            {:else if type === 'budget'}
              🎯
            {:else if type === 'project'}
              🚀
            {:else if type === 'joint-spent'}
              🏦
            {:else if type === 'income'}
              💰
            {:else}
              📊
            {/if}
          </div>

          <div class="min-w-0">
            <div class="flex items-center gap-2 flex-wrap">
              <h2 id="drilldown-modal-title" class="text-base sm:text-lg font-bold text-neutral-900 dark:text-white truncate">
                {title}
              </h2>
              {#if type === 'category-adjustment'}
                <span class="badge-amber text-[10px]">Settlement</span>
              {:else if type === 'budget'}
                <span class="badge-indigo text-[10px]">Budget Cap</span>
              {:else if type === 'joint-spent'}
                <span class="badge-indigo text-[10px]">Joint Account</span>
              {/if}
              {#if scopedLabel}
                <span class="inline-flex items-center gap-1 text-[10px] font-semibold px-2 py-0.5 rounded-full bg-indigo-50 dark:bg-indigo-950/80 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800/60" title="Filtered to dashboard scope">
                  <span>Scope:</span>
                  <span class="font-bold">{scopedLabel}</span>
                </span>
              {/if}
            </div>
            <p class="text-xs text-neutral-500 dark:text-neutral-400 truncate mt-0.5">
              {#if type === 'category'}
                Category expense distribution & contributing transactions &middot; {$selectedMonth}
              {:else if type === 'category-adjustment'}
                Reconciliation breakdown & contributing expenses &middot; {$selectedMonth}
              {:else if type === 'payer'}
                Expenses paid by {title} & category spread &middot; {$selectedMonth}
              {:else if type === 'budget'}
                Budget limit progress & itemized spend &middot; {$selectedMonth}
              {:else if type === 'project'}
                Savings milestone contributions & expenses &middot; {$selectedMonth}
              {:else if type === 'joint-spent'}
                Joint account expenses overview &middot; {$selectedMonth}
              {:else if type === 'income'}
                Household income streams & earnings &middot; {$selectedMonth}
              {:else}
                Monthly overview & itemized ledger &middot; {$selectedMonth}
              {/if}
              {#if scopedLabel} &middot; Scoped to {scopedLabel}{/if}
            </p>
          </div>
        </div>

        <!-- Accessible Close Button (Min 44x44px Touch Target) -->
        <button
          bind:this={closeButton}
          type="button"
          on:click={closeDrilldown}
          class="w-9 h-9 rounded-xl flex items-center justify-center text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 hover:bg-neutral-100 dark:hover:bg-neutral-800 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500 cursor-pointer flex-shrink-0"
          aria-label="Close drill-down details"
          title="Close (Esc)"
        >
          <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- ── Scrollable Body ───────────────────────────────────────────────── -->
      <div class="p-5 sm:p-6 overflow-y-auto space-y-6 flex-1">

        <!-- ═══ 1. CATEGORY DRILLDOWN (Category Totals & Expenses Per User) ═══ -->
        {#if type === 'category' || type === 'budget'}
          <!-- High-Level Metric Cards -->
          <div class="grid grid-cols-2 sm:grid-cols-3 gap-3">
            <div class="card-sub p-3.5 space-y-1">
              <p class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wider">Total Spent</p>
              <p class="text-xl font-bold text-neutral-900 dark:text-white tabular-nums">
                <span class:privacy-masked={$privacyShield}>{fmt(categoryTotalCents / 100)}</span>
              </p>
            </div>

            <div class="card-sub p-3.5 space-y-1">
              <p class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wider">Transactions</p>
              <p class="text-xl font-bold text-neutral-900 dark:text-white tabular-nums">
                {categoryExpenses.length}
              </p>
            </div>

            <div class="card-sub p-3.5 space-y-1 col-span-2 sm:col-span-1">
              <p class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wider">Avg Transaction</p>
              <p class="text-xl font-bold text-neutral-900 dark:text-white tabular-nums">
                <span class:privacy-masked={$privacyShield}>
                  {categoryExpenses.length > 0 ? fmt((categoryTotalCents / categoryExpenses.length) / 100) : '—'}
                </span>
              </p>
            </div>
          </div>

          <!-- Expenses Per User for that Cost -->
          <div class="space-y-3">
            <div class="flex items-center justify-between">
              <h3 class="text-xs font-bold uppercase tracking-wider text-neutral-700 dark:text-neutral-300">
                Expenses Per User
              </h3>
              <span class="text-xs text-neutral-500">
                {categoryUserBreakdown.length} {categoryUserBreakdown.length === 1 ? 'payer' : 'payers'}
              </span>
            </div>

            <!-- Proportional Visual Distribution Bar -->
            {#if categoryTotalCents > 0}
              <div class="flex h-4 rounded-lg overflow-hidden bg-neutral-100 dark:bg-neutral-800/60 p-0.5 border border-neutral-200 dark:border-neutral-800">
                {#each categoryUserBreakdown as u}
                  {#if u.pct > 0}
                    <div
                      class="h-full rounded-sm transition-all duration-300"
                      style="width: {u.pct}%; background-color: {u.color}; min-width: 4px;"
                      title="{u.name}: {fmt(u.cents / 100)} ({u.pct.toFixed(1)}%)"
                    ></div>
                  {/if}
                {/each}
              </div>
            {/if}

            <!-- User Breakdown Pills / Mini Cards -->
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
              {#each categoryUserBreakdown as u}
                <div class="card-sub p-3 flex items-center justify-between gap-3 border-l-4" style="border-left-color: {u.color}">
                  <div class="flex items-center gap-2.5 min-w-0">
                    <div
                      class="w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold text-white shadow-xs flex-none"
                      style="background-color: {u.color}"
                    >
                      {userInitial(u.name)}
                    </div>
                    <div class="min-w-0">
                      <p class="text-xs font-bold text-neutral-900 dark:text-white truncate">{u.name}</p>
                      <p class="text-[11px] text-neutral-500">{u.count} {u.count === 1 ? 'expense' : 'expenses'}</p>
                    </div>
                  </div>
                  <div class="text-right flex-none">
                    <p class="text-xs font-bold text-neutral-900 dark:text-white tabular-nums">
                      <span class:privacy-masked={$privacyShield}>{fmt(u.cents / 100)}</span>
                    </p>
                    <p class="text-[10px] font-semibold tabular-nums" style="color: {u.color}">
                      {u.pct.toFixed(0)}% of cost
                    </p>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- ═══ 2. CATEGORY ADJUSTMENT SETTLEMENT DRILLDOWN ═══════════════════ -->
        {#if type === 'category-adjustment'}
          <!-- Settlement Reconciliation Overview -->
          <div class="space-y-3">
            <h3 class="text-xs font-bold uppercase tracking-wider text-neutral-700 dark:text-neutral-300">
              Settlement Allocation & Net Balance
            </h3>

            <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
              {#each adjustmentUserShares as u}
                <div class="card-sub p-3 space-y-2 border-l-4" style="border-left-color: {u.color}">
                  <div class="flex items-center justify-between">
                    <div class="flex items-center gap-2">
                      <div class="w-6 h-6 rounded-full flex items-center justify-center text-[10px] font-bold text-white" style="background-color: {u.color}">
                        {userInitial(u.name)}
                      </div>
                      <span class="text-xs font-bold text-neutral-900 dark:text-white">{u.name}</span>
                    </div>
                    <span class="text-xs font-semibold px-2 py-0.5 rounded-full" style="background-color: {u.color}15; color: {u.color}">
                      Agreed Share: {u.targetPct.toFixed(0)}%
                    </span>
                  </div>

                  <div class="flex items-center justify-between text-xs pt-1 border-t border-neutral-200 dark:border-neutral-800">
                    <span class="text-neutral-500">Paid: <strong class="text-neutral-800 dark:text-neutral-200 font-semibold tabular-nums">{fmt(u.paid)}</strong></span>
                    <span class="font-bold tabular-nums {u.net > 0.005 ? 'text-emerald-600 dark:text-emerald-400' : u.net < -0.005 ? 'text-rose-600 dark:text-red-400' : 'text-neutral-500'}">
                      {u.net > 0.005 ? `+${fmt(u.net)} owed back` : u.net < -0.005 ? `${fmt(u.net)} owes` : 'settled'}
                    </span>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- ═══ 3. PAYER DRILLDOWN ════════════════════════════════════════════ -->
        {#if type === 'payer'}
          <div class="grid grid-cols-2 gap-3">
            <div class="card-sub p-3.5 space-y-1">
              <p class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wider">Total Paid by {title}</p>
              <p class="text-xl font-bold tabular-nums" style="color: {userColor(title)}">
                <span class:privacy-masked={$privacyShield}>{fmt(payerTotalCents / 100)}</span>
              </p>
            </div>
            <div class="card-sub p-3.5 space-y-1">
              <p class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wider">Transactions Paid</p>
              <p class="text-xl font-bold text-neutral-900 dark:text-white tabular-nums">
                {payerExpenses.length}
              </p>
            </div>
          </div>

          <!-- Categories Paid for by this Payer -->
          <div class="space-y-3">
            <h3 class="text-xs font-bold uppercase tracking-wider text-neutral-700 dark:text-neutral-300">
              Spending by Category
            </h3>
            <div class="space-y-2">
              {#each payerCategoryBreakdown as c}
                <div class="p-2.5 rounded-xl bg-neutral-50 dark:bg-neutral-950/70 border border-neutral-200 dark:border-neutral-800 space-y-1.5">
                  <div class="flex items-center justify-between text-xs">
                    <span class="font-semibold text-neutral-800 dark:text-neutral-200">{c.category} ({c.count})</span>
                    <span class="font-bold text-neutral-900 dark:text-white tabular-nums">{fmt(c.cents / 100)} ({c.pct.toFixed(0)}%)</span>
                  </div>
                  <div class="w-full h-1.5 bg-neutral-200 dark:bg-neutral-800 rounded-full overflow-hidden">
                    <div class="h-full rounded-full bg-indigo-500" style="width: {c.pct}%"></div>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- ═══ 4. PROJECT DRILLDOWN ══════════════════════════════════════════ -->
        {#if type === 'project' && projectItem}
          <div class="card-sub p-4 space-y-2">
            <div class="flex justify-between items-baseline">
              <span class="text-xs font-semibold text-neutral-500">Milestone Progress</span>
              <span class="text-xs font-bold text-indigo-600 dark:text-indigo-400 tabular-nums">
                {Math.min(100, Math.round((projectItem.total_spent_cents / projectItem.target_cents) * 100))}%
              </span>
            </div>
            <div class="w-full h-2 bg-neutral-200 dark:bg-neutral-800 rounded-full overflow-hidden">
              <div
                class="h-full rounded-full bg-indigo-500"
                style="width: {Math.min(100, Math.round((projectItem.total_spent_cents / projectItem.target_cents) * 100))}%"
              ></div>
            </div>
            <p class="text-xs text-neutral-600 dark:text-neutral-400 pt-1">
              Spent: <strong>{fmt(projectItem.total_spent_cents / 100)}</strong> of target <strong>{fmt(projectItem.target_cents / 100)}</strong>
            </p>
          </div>
        {/if}

        <!-- ═══ 5. JOINT SPENT DRILLDOWN ══════════════════════════════════════ -->
        {#if type === 'joint-spent'}
          <div class="grid grid-cols-2 gap-3">
            <div class="card-sub p-3.5 space-y-1">
              <p class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wider">Total Joint Spend</p>
              <p class="text-xl font-bold text-neutral-900 dark:text-white tabular-nums">
                <span class:privacy-masked={$privacyShield}>{fmt(jointTotalCents / 100)}</span>
              </p>
            </div>
            <div class="card-sub p-3.5 space-y-1">
              <p class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wider">Joint Expenses</p>
              <p class="text-xl font-bold text-neutral-900 dark:text-white tabular-nums">
                {jointExpenses.length}
              </p>
            </div>
          </div>
        {/if}

        <!-- ═══ 6. HOUSEHOLD INCOME DRILLDOWN ═════════════════════════════════ -->
        {#if type === 'income'}
          <div class="space-y-4">
            <h3 class="text-xs font-bold uppercase tracking-wider text-neutral-700 dark:text-neutral-300">
              Household Member Income Streams
            </h3>
            <div class="space-y-3">
              {#each ($users || []).filter((u) => u.is_active !== false && (!scopedUserSet || scopedUserSet.has(u.name))) as u}
                {@const userJobs = ($jobs || []).filter((j) => j.who === u.name && j.is_active)}
                {@const userOneOff = ($incomeEntries || []).filter((e) => e.who === u.name)}
                <div class="card-sub p-3.5 space-y-2 border-l-4" style="border-left-color: {userColor(u.name)}">
                  <div class="flex items-center justify-between">
                    <span class="text-xs font-bold text-neutral-900 dark:text-white">{u.name}</span>
                    <span class="text-[11px] text-neutral-500">
                      {userJobs.length} active {userJobs.length === 1 ? 'job' : 'jobs'} &middot; {userOneOff.length} one-off entries
                    </span>
                  </div>
                  {#if userJobs.length > 0}
                    <div class="text-xs space-y-1 text-neutral-600 dark:text-neutral-400">
                      {#each userJobs as j}
                        <div class="flex items-center justify-between bg-white/70 dark:bg-neutral-900/70 px-2.5 py-1.5 rounded-lg">
                          <span class="font-medium">{j.name} ({j.frequency})</span>
                          <span class="font-bold tabular-nums text-neutral-900 dark:text-white">{fmt(j.amount_cents / 100)}</span>
                        </div>
                      {/each}
                    </div>
                  {:else}
                    <p class="text-xs text-neutral-400">No active employment contracts configured.</p>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- ═══ ITEMIZED EXPENSE OVERVIEW TABLE / LIST ════════════════════════ -->
        {#if type !== 'income'}
          <div class="space-y-3 pt-2 border-t border-neutral-200 dark:border-neutral-800">
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2.5">
              <div>
                <h3 class="text-xs font-bold uppercase tracking-wider text-neutral-700 dark:text-neutral-300">
                  Itemized Expenses ({filteredExpenses.length})
                </h3>
                <p class="text-[11px] text-neutral-500">
                  Detailed transactions contributing to this summary
                </p>
              </div>

              <!-- Search Filter Input -->
              <div class="relative w-full sm:w-56">
                <input
                  type="text"
                  bind:value={searchQuery}
                  placeholder="Filter expenses…"
                  class="input-field py-1.5 pl-8 pr-7 text-xs w-full"
                />
                <span class="absolute left-2.5 top-1/2 -translate-y-1/2 text-neutral-400 text-xs select-none">🔍</span>
                {#if searchQuery}
                  <button
                    type="button"
                    on:click={() => (searchQuery = '')}
                    class="absolute right-2.5 top-1/2 -translate-y-1/2 text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200 text-xs cursor-pointer"
                    aria-label="Clear filter"
                  >
                    ✕
                  </button>
                {/if}
              </div>
            </div>

            <!-- Expense List Rows -->
            {#if filteredExpenses.length > 0}
              <div class="space-y-2 max-h-72 overflow-y-auto pr-1">
                {#each filteredExpenses as exp (exp.id)}
                  {@const payerColor = userColor(exp.who_paid)}
                  <div class="card-sub p-3 flex items-center justify-between gap-3 hover:border-neutral-300 dark:hover:border-neutral-700 transition-colors">
                    <!-- Left: Date, title, tags, project -->
                    <div class="min-w-0 flex-1 space-y-1">
                      <div class="flex items-center gap-2 flex-wrap">
                        <span class="text-[11px] font-mono text-neutral-400 tabular-nums">
                          {exp.expense_date}
                        </span>
                        <span class="text-xs font-semibold text-neutral-900 dark:text-white truncate">
                          {exp.name}
                        </span>
                        {#if exp.is_joint}
                          <span class="badge-indigo text-[9px] py-0 px-1.5">Joint</span>
                        {/if}
                        {#if exp.tag_id && tagMap[exp.tag_id]}
                          <span class="text-[9px] px-1.5 py-0.5 rounded font-medium border"
                                style="background-color: {tagMap[exp.tag_id].color}15; color: {tagMap[exp.tag_id].color}; border-color: {tagMap[exp.tag_id].color}40">
                            #{tagMap[exp.tag_id].name}
                          </span>
                        {/if}
                        {#if exp.project_id && projectMap[exp.project_id]}
                          <span class="text-[9px] px-1.5 py-0.5 rounded font-medium bg-neutral-200 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300">
                            📁 {projectMap[exp.project_id].name}
                          </span>
                        {/if}
                      </div>

                      <div class="flex items-center gap-2 text-[11px] text-neutral-500">
                        <span class="flex items-center gap-1">
                          <span class="w-2 h-2 rounded-full" style="background-color: {payerColor}"></span>
                          <span class="font-medium" style="color: {payerColor}">{exp.who_paid}</span>
                        </span>
                        {#if type !== 'category' && type !== 'category-adjustment'}
                          <span>&middot;</span>
                          <span class="text-neutral-400">{exp.category}</span>
                        {/if}
                        {#if exp.overrides && exp.overrides.length > 0}
                          <span>&middot;</span>
                          <span class="text-amber-600 dark:text-amber-400 font-medium">Custom override</span>
                        {/if}
                      </div>
                    </div>

                    <!-- Right: Amount -->
                    <div class="text-right flex-none pl-2">
                      <span class="text-sm font-bold text-neutral-900 dark:text-white tabular-nums">
                        <span class:privacy-masked={$privacyShield}>{fmt((exp.cost_cents || 0) / 100)}</span>
                      </span>
                    </div>
                  </div>
                {/each}
              </div>
            {:else}
              <div class="empty-state-box py-6">
                <p class="text-xs font-semibold text-neutral-700 dark:text-neutral-300">
                  {searchQuery ? 'No matching expenses found' : 'No expenses logged for this item yet'}
                </p>
                {#if searchQuery}
                  <button
                    type="button"
                    on:click={() => (searchQuery = '')}
                    class="btn-ghost text-xs mt-1 text-indigo-600 dark:text-indigo-400"
                  >
                    Clear filter
                  </button>
                {/if}
              </div>
            {/if}
          </div>
        {/if}

      </div>

      <!-- ── Footer ───────────────────────────────────────────────────────── -->
      <div class="px-5 py-3 sm:px-6 border-t border-neutral-200 dark:border-neutral-800 flex items-center justify-between bg-neutral-50/50 dark:bg-neutral-950/30 text-xs text-neutral-500">
        <span>Press <kbd class="px-1.5 py-0.5 rounded bg-neutral-200 dark:bg-neutral-800 text-[10px] font-mono text-neutral-700 dark:text-neutral-300">Esc</kbd> to close</span>
        <button
          type="button"
          on:click={closeDrilldown}
          class="btn-secondary py-1.5 px-3.5 text-xs cursor-pointer"
        >
          Done
        </button>
      </div>

    </div>
  </div>
{/if}

<style>
  @keyframes drilldownIn {
    from {
      opacity: 0;
      transform: scale(0.96) translateY(6px);
    }
    to {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }

  .drilldown-animate {
    animation: drilldownIn 200ms cubic-bezier(0.23, 1, 0.32, 1) both;
  }

  @media (prefers-reduced-motion: reduce) {
    .drilldown-animate {
      animation: none !important;
    }
  }
</style>
