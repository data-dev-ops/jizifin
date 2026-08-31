<script>
  /**
   * BudgetManager.svelte
   *
   * Dedicated Budget Management Tab with full Light/Dark mode theming:
   *  - Top KPIs: Total Monthly Budget, Actual Spent, Remaining Buffer, Health Score
   *  - Scope Switcher: Household Total, Per-User Budgets, Joint Account Budgets
   *  - Subtabs: 🌟 All-in-One Dashboard | 📊 Health & Progress | 🎯 Limits Matrix | ➕ Add Limit
   *  - Granular Per-User & Joint Account Budget Filtering
   */

  import { onMount } from 'svelte';
  import { budgets, splits, selectedMonth, currencySymbol, users, jointAccounts, jointAccount } from './stores.js';
  import * as api from './api.js';
  import { upsertBudget, deleteBudget, fetchBudgetAnalytics } from './api.js';

  // ── Active subtab view ─────────────────────────────────────────────────────
  let activeSubtab = 'all'; // 'all' | 'health' | 'matrix' | 'add'

  // ── Scope Filter for Health Overview ───────────────────────────────────────
  let selectedScope = 'ALL'; // 'ALL' | 'USER:<name>' | 'JOINT'

  let budgetStatus = []; // BudgetStatusRow[]
  let editMap = {};      // category+month → pending cents input
  let saving = {};
  let error = '';
  let newForm = { category: '', month: 'ALL', limit_euros: '' };
  let addError = '';
  let addSuccess = false;
  let addSaving = false;
  let matrixFilter = '';

  // Per-row delete confirmation
  let confirmDeleteKey = null;

  $: monthStr = $selectedMonth;
  $: activeUsers = $users.filter((u) => u.is_active !== false);

  async function loadStatus() {
    try {
      let whoPaid = null;
      let isJoint = null;
      if (selectedScope.startsWith('USER:')) {
        whoPaid = selectedScope.slice(5);
      } else if (selectedScope === 'JOINT') {
        isJoint = true;
      }
      budgetStatus = await api.fetchBudgetAnalytics(monthStr, whoPaid, isJoint);
    } catch (e) {
      error = e.message;
    }
  }

  onMount(loadStatus);
  $: monthStr, selectedScope, loadStatus();

  function statusColor(pct) {
    if (pct >= 90) return 'red';
    if (pct >= 70) return 'yellow';
    return 'green';
  }

  function editKey(cat, month) { return `${cat}::${month}`; }

  function startEdit(b) {
    editMap[editKey(b.category, b.month)] = (b.limit_cents / 100).toFixed(2);
  }

  async function saveEdit(b) {
    const key = editKey(b.category, b.month);
    const limit_cents = Math.round(parseFloat(editMap[key]) * 100);
    if (isNaN(limit_cents) || limit_cents < 0) return;
    saving[key] = true;
    try {
      await upsertBudget({ category: b.category, month: b.month, limit_cents });
      delete editMap[key];
      editMap = editMap;
      await loadStatus();
    } catch (e) {
      error = e.message;
    } finally {
      saving[key] = false;
    }
  }

  function requestBudgetDelete(b) {
    confirmDeleteKey = editKey(b.category, b.month);
  }

  function cancelBudgetDelete() {
    confirmDeleteKey = null;
  }

  async function confirmBudgetDelete(b) {
    try {
      await deleteBudget(b.category, b.month);
      confirmDeleteKey = null;
      await loadStatus();
    } catch (e) {
      error = e.message;
    }
  }

  function setQuickAmount(eur) {
    const cur = parseFloat(newForm.limit_euros) || 0;
    newForm.limit_euros = (cur + eur).toFixed(2);
  }

  function setQuickMonth(m) {
    newForm.month = m;
  }

  async function handleAdd() {
    addError = '';
    addSuccess = false;
    const limit_cents = Math.round(parseFloat(newForm.limit_euros) * 100);
    if (!newForm.category || isNaN(limit_cents) || limit_cents < 0) {
      addError = 'Choose a category and enter a valid amount.';
      return;
    }
    addSaving = true;
    try {
      await upsertBudget({ category: newForm.category, month: newForm.month || 'ALL', limit_cents });
      addSuccess = true;
      newForm = { category: '', month: 'ALL', limit_euros: '' };
      await loadStatus();
    } catch (e) {
      addError = e.message;
    } finally {
      addSaving = false;
    }
  }

  // ── Calculated KPIs ────────────────────────────────────────────────────────
  $: totalBudgetedLimitCents = budgetStatus.reduce((sum, r) => sum + (r.limit_cents || 0), 0);
  $: totalActualSpentCents = budgetStatus.reduce((sum, r) => sum + (r.actual_cents || 0), 0);
  $: totalRemainingBufferCents = Math.max(0, totalBudgetedLimitCents - totalActualSpentCents);
  $: safeCategoriesCount = budgetStatus.filter((r) => r.limit_cents > 0 && r.pct_used < 90).length;
  $: totalBudgetedCount = budgetStatus.filter((r) => r.limit_cents > 0).length;
  $: healthIndex = totalBudgetedCount > 0 ? Math.round((safeCategoriesCount / totalBudgetedCount) * 100) : 100;
</script>

<div class="space-y-6">

  <!-- ── Page Header ──────────────────────────────────────────────────────── -->
  <header class="page-header">
    <div>
      <h1 class="page-title flex items-center gap-2.5">
        <span>🎯</span> Budget Tracker & Spending Limits
      </h1>
      <p class="page-subtitle">
        Configure category spending ceilings, monitor live budget health, and prevent overspending.
      </p>
    </div>
  </header>

  <!-- ── Top Overview Banner & KPI Cards ──────────────────────────────────── -->
  <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">

    <!-- 1. Total Monthly Limit -->
    <div class="card p-4 sm:p-5 flex flex-col justify-between">
      <div>
        <div class="flex items-center justify-between gap-2 mb-2">
          <p class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">Total Monthly Limit</p>
          <span class="badge-indigo">Budgets</span>
        </div>
        <p class="font-bold text-neutral-900 dark:text-white tabular-nums text-xl sm:text-2xl">
          {$currencySymbol}{(totalBudgetedLimitCents / 100).toFixed(0)}
        </p>
      </div>
      <div class="mt-3 pt-2.5 border-t border-neutral-200 dark:border-neutral-800/80 text-[11px] text-neutral-500">
        <span>Across {totalBudgetedCount} configured category limits</span>
      </div>
    </div>

    <!-- 2. Actual Spent -->
    <div class="card p-4 sm:p-5 flex flex-col justify-between">
      <div>
        <div class="flex items-center justify-between gap-2 mb-2">
          <p class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">Spent ({monthStr})</p>
          <span class="badge-amber">Actual</span>
        </div>
        <p class="font-bold text-neutral-900 dark:text-white tabular-nums text-xl sm:text-2xl">
          {$currencySymbol}{(totalActualSpentCents / 100).toFixed(0)}
        </p>
      </div>
      <div class="mt-3 pt-2.5 border-t border-neutral-200 dark:border-neutral-800/80 text-[11px] text-neutral-500 flex justify-between">
        <span>Overall Usage:</span>
        <span class="font-semibold text-neutral-700 dark:text-neutral-300">
          {totalBudgetedLimitCents > 0 ? Math.round((totalActualSpentCents / totalBudgetedLimitCents) * 100) : 0}% used
        </span>
      </div>
    </div>

    <!-- 3. Remaining Buffer -->
    <div class="card p-4 sm:p-5 flex flex-col justify-between">
      <div>
        <div class="flex items-center justify-between gap-2 mb-2">
          <p class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">Remaining Buffer</p>
          <span class="badge-emerald">Safety Margin</span>
        </div>
        <p class="font-bold tabular-nums text-xl sm:text-2xl {totalRemainingBufferCents > 0 ? 'text-emerald-600 dark:text-emerald-400' : 'text-rose-600 dark:text-red-400'}">
          {$currencySymbol}{(totalRemainingBufferCents / 100).toFixed(0)}
        </p>
      </div>
      <div class="mt-3 pt-2.5 border-t border-neutral-200 dark:border-neutral-800/80 text-[11px] text-neutral-500">
        <span>{totalRemainingBufferCents > 0 ? 'Available unallocated buffer' : 'Cap exceeded'}</span>
      </div>
    </div>

    <!-- 4. Budget Health Score -->
    <div class="card p-4 sm:p-5 flex flex-col justify-between">
      <div>
        <div class="flex items-center justify-between gap-2 mb-2">
          <p class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">Health Index</p>
          <span class="badge-emerald">{healthIndex}% Safe</span>
        </div>
        <p class="font-bold text-neutral-900 dark:text-white tabular-nums text-xl sm:text-2xl">
          {safeCategoriesCount} / {totalBudgetedCount}
        </p>
      </div>
      <div class="mt-3 pt-2.5 border-t border-neutral-200 dark:border-neutral-800/80 text-[11px] text-neutral-500">
        <span>Categories within safe spending limits</span>
      </div>
    </div>

  </div>

  <!-- ── Subtab Navigation Bar ─────────────────────────────────────────────── -->
  <div class="flex items-center justify-between border-b border-neutral-200 dark:border-neutral-800 pb-3 flex-wrap gap-3">
    <div class="flex items-center gap-2 overflow-x-auto">
      <button
        type="button"
        on:click={() => (activeSubtab = 'all')}
        class="px-3.5 py-1.5 rounded-xl text-xs font-semibold transition-all flex items-center gap-1.5 cursor-pointer whitespace-nowrap {activeSubtab === 'all' ? 'bg-indigo-600 text-white shadow-xs' : 'bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
      >
        <span>🌟</span>
        <span>All-in-One Studio</span>
      </button>

      <button
        type="button"
        on:click={() => (activeSubtab = 'health')}
        class="px-3.5 py-1.5 rounded-xl text-xs font-semibold transition-all flex items-center gap-1.5 cursor-pointer whitespace-nowrap {activeSubtab === 'health' ? 'bg-indigo-600 text-white shadow-xs' : 'bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
      >
        <span>📊</span>
        <span>Budget Health & Progress</span>
      </button>

      <button
        type="button"
        on:click={() => (activeSubtab = 'matrix')}
        class="px-3.5 py-1.5 rounded-xl text-xs font-semibold transition-all flex items-center gap-1.5 cursor-pointer whitespace-nowrap {activeSubtab === 'matrix' ? 'bg-indigo-600 text-white shadow-xs' : 'bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
      >
        <span>🎯</span>
        <span>Configured Limits ({$budgets.length})</span>
      </button>

      <button
        type="button"
        on:click={() => (activeSubtab = 'add')}
        class="px-3.5 py-1.5 rounded-xl text-xs font-semibold transition-all flex items-center gap-1.5 cursor-pointer whitespace-nowrap {activeSubtab === 'add' ? 'bg-indigo-600 text-white shadow-xs' : 'bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
      >
        <span>➕</span>
        <span>Add / Update Limit</span>
      </button>
    </div>

    <!-- Quick Month Context -->
    <div class="flex items-center gap-2">
      <span class="text-xs text-neutral-500 font-mono">Viewing month:</span>
      <span class="badge-indigo">{monthStr}</span>
    </div>
  </div>

  {#if error}
    <div class="bg-rose-50 dark:bg-red-950/40 border border-rose-200 dark:border-red-900/60 text-rose-700 dark:text-red-400 rounded-xl px-4 py-3 text-xs">{error}</div>
  {/if}

  <!-- ════ SECTION 1: Budget Health & Progress Grid ═══════════════════════════ -->
  {#if activeSubtab === 'all' || activeSubtab === 'health'}
    <div class="space-y-5 animate-fadeIn">

      <!-- Scope Filter Chips (Per-User / Joint Account / All) -->
      <div class="card p-3.5 flex items-center justify-between flex-wrap gap-3">
        <div class="flex items-center gap-2">
          <span class="text-xs font-bold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider">Spending Scope:</span>
        </div>

        <div class="flex items-center gap-1.5 flex-wrap">
          <button
            type="button"
            on:click={() => (selectedScope = 'ALL')}
            class="px-3 py-1 rounded-lg text-xs font-semibold border transition-all cursor-pointer {selectedScope === 'ALL' ? 'bg-indigo-600 text-white border-indigo-400 shadow-sm' : 'bg-neutral-100 dark:bg-neutral-900 border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
          >
            👥 Whole Household
          </button>

          {#each activeUsers as u}
            <button
              type="button"
              on:click={() => (selectedScope = `USER:${u.name}`)}
              class="inline-flex items-center gap-1.5 px-3 py-1 rounded-lg text-xs font-semibold border transition-all cursor-pointer {selectedScope === `USER:${u.name}` ? 'bg-indigo-600 text-white border-indigo-400 shadow-sm' : 'bg-neutral-100 dark:bg-neutral-900 border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
            >
              <span class="w-2 h-2 rounded-full" style="background-color: {u.color}"></span>
              <span>👤 {u.name}</span>
            </button>
          {/each}

          {#if ($jointAccounts || []).length > 0 || $jointAccount}
            <button
              type="button"
              on:click={() => (selectedScope = 'JOINT')}
              class="px-3 py-1 rounded-lg text-xs font-semibold border transition-all cursor-pointer {selectedScope === 'JOINT' ? 'bg-indigo-600 text-white border-indigo-400 shadow-sm' : 'bg-neutral-100 dark:bg-neutral-900 border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
            >
              🏦 Joint Account
            </button>
          {/if}
        </div>
      </div>

      {#if budgetStatus.length === 0}
        <div class="card empty-state-box py-10">
          <div class="w-12 h-12 rounded-2xl bg-neutral-100 dark:bg-neutral-800 flex items-center justify-center text-2xl mb-3">🎯</div>
          <p class="text-neutral-700 dark:text-neutral-300 text-sm font-semibold">No budget data for this month.</p>
          <p class="text-neutral-500 text-xs mt-1 max-w-sm">
            Set up category budget limits using the Add / Update form to start tracking spending targets.
          </p>
        </div>
      {:else}
        <!-- Grid of Category Budget Health Cards -->
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
          {#each budgetStatus as row}
            {@const color = statusColor(row.pct_used)}
            {@const barPct = Math.min(row.pct_used, 100)}
            {@const barClass = color === 'red' ? 'from-rose-500 to-rose-400' : color === 'yellow' ? 'from-amber-400 to-amber-300' : 'from-emerald-500 to-emerald-400'}
            {@const textClass = color === 'red' ? 'text-rose-600 dark:text-red-400' : color === 'yellow' ? 'text-amber-600 dark:text-yellow-400' : 'text-emerald-600 dark:text-emerald-400'}
            {@const isStanding = !row.budget_month || row.budget_month === 'ALL'}
            {@const remainingCents = Math.max(0, (row.limit_cents || 0) - row.actual_cents)}

            <div class="card p-4 space-y-3 hover:border-neutral-300 dark:hover:border-neutral-700 transition-all flex flex-col justify-between">
              <div>
                <div class="flex items-start justify-between gap-1 mb-1.5">
                  <span class="text-xs font-bold text-neutral-900 dark:text-white uppercase tracking-wide truncate">{row.category}</span>
                  {#if isStanding}
                    <span class="text-[9px] font-semibold uppercase tracking-wide text-neutral-600 dark:text-neutral-400 bg-neutral-200 dark:bg-neutral-800 rounded px-1.5 py-0.5 leading-none">standing</span>
                  {:else}
                    <span class="text-[9px] font-semibold uppercase tracking-wide text-indigo-700 dark:text-indigo-400 bg-indigo-100 dark:bg-indigo-950/80 border border-indigo-200 dark:border-indigo-800/60 rounded px-1.5 py-0.5 leading-none">this month</span>
                  {/if}
                </div>

                <div class="flex items-baseline justify-between text-xs mt-2">
                  <span class="text-sm font-bold tabular-nums text-neutral-900 dark:text-neutral-100">
                    {$currencySymbol}{(row.actual_cents / 100).toFixed(2)}
                  </span>
                  {#if row.limit_cents > 0}
                    <span class="text-neutral-500 font-normal">
                      cap: {$currencySymbol}{(row.limit_cents / 100).toFixed(0)}
                    </span>
                  {:else}
                    <span class="text-neutral-500 text-[11px]">(no limit)</span>
                  {/if}
                </div>

                {#if row.limit_cents > 0}
                  <div class="h-2 bg-neutral-200 dark:bg-neutral-950 rounded-full overflow-hidden mt-2.5 shadow-inner">
                    <div
                      class="h-full rounded-full bg-gradient-to-r {barClass} transition-all duration-500"
                      style="width:{barPct}%"
                    ></div>
                  </div>
                  <div class="flex items-center justify-between mt-1 text-[11px]">
                    <span class="text-neutral-500">
                      {remainingCents > 0 ? `Buffer: ${$currencySymbol}${(remainingCents / 100).toFixed(0)}` : '🚨 Over budget'}
                    </span>
                    <span class="{textClass} font-bold tabular-nums">{row.pct_used.toFixed(1)}%</span>
                  </div>
                {/if}
              </div>

              <!-- Quick action -->
              <div class="pt-2 border-t border-neutral-200 dark:border-neutral-800/80 flex items-center justify-between">
                <button
                  type="button"
                  on:click={() => {
                    newForm.category = row.category;
                    newForm.month = 'ALL';
                    newForm.limit_euros = (row.limit_cents / 100).toFixed(2);
                    activeSubtab = 'add';
                  }}
                  class="text-[11px] font-semibold text-indigo-600 dark:text-indigo-400 hover:underline transition-colors cursor-pointer"
                >
                  Adjust Limit →
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}

    </div>
  {/if}

  <!-- ════ SECTION 2 & 3: Limits Matrix & Add Form ════════════════════════════ -->
  {#if activeSubtab === 'all' || activeSubtab === 'matrix' || activeSubtab === 'add'}
    <div class="grid grid-cols-1 xl:grid-cols-5 gap-6 animate-fadeIn">

      <!-- Left Column: Configured Limits Matrix Table (xl:col-span-3) -->
      {#if activeSubtab === 'all' || activeSubtab === 'matrix'}
        <div class="{activeSubtab === 'matrix' ? 'xl:col-span-5' : 'xl:col-span-3'} card p-5 space-y-4">
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-200 dark:border-neutral-800 pb-3">
            <div>
              <h3 class="text-sm font-bold text-neutral-800 dark:text-neutral-200">Configured Limits Matrix</h3>
              <p class="text-xs text-neutral-500 mt-0.5">All standing defaults and month-specific limits</p>
            </div>

            <input
              type="text"
              bind:value={matrixFilter}
              placeholder="Search category…"
              class="input-field py-1 px-3 text-xs max-w-[180px]"
            />
          </div>

          {#if $budgets.length === 0}
            <div class="empty-state-box py-8">
              <p class="text-neutral-700 dark:text-neutral-300 text-sm font-semibold">No limits configured yet.</p>
              <p class="text-neutral-500 text-xs mt-1">Use the Add Limit form to set up your first category limit.</p>
            </div>
          {:else}
            <div class="overflow-x-auto rounded-xl border border-neutral-200 dark:border-neutral-800">
              <table class="w-full text-xs border-collapse">
                <thead>
                  <tr class="bg-neutral-50 dark:bg-neutral-950/80 border-b border-neutral-200 dark:border-neutral-800">
                    <th class="text-left font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider px-4 py-3">Category</th>
                    <th class="text-left font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider px-4 py-3">Month</th>
                    <th class="text-left font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider px-4 py-3">Limit ({$currencySymbol})</th>
                    <th class="px-4 py-3 text-right">Actions</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-neutral-200/70 dark:divide-neutral-800/70">
                  {#each $budgets.filter((b) => b.category.toLowerCase().includes(matrixFilter.toLowerCase())) as b}
                    {@const key = editKey(b.category, b.month)}
                    <tr class="hover:bg-neutral-50 dark:hover:bg-neutral-800/40 transition-colors group">
                      <td class="px-4 py-3 font-semibold text-neutral-800 dark:text-neutral-200">{b.category}</td>
                      <td class="px-4 py-3 text-neutral-500 dark:text-neutral-400">
                        {#if b.month === 'ALL'}
                          <span class="inline-flex items-center px-2 py-0.5 rounded-md bg-neutral-100 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300 font-medium">ALL</span>
                        {:else}
                          <span class="inline-flex items-center px-2 py-0.5 rounded-md bg-indigo-50 dark:bg-indigo-950/80 border border-indigo-200 dark:border-indigo-800/60 text-indigo-700 dark:text-indigo-300 font-medium">{b.month}</span>
                        {/if}
                      </td>
                      <td class="px-4 py-3 tabular-nums">
                        {#if key in editMap}
                          <span class="inline-flex items-center gap-1.5">
                            <input
                              class="w-24 input-field py-1 px-2 text-xs"
                              type="number" min="0" step="0.01"
                              bind:value={editMap[key]}
                              on:keydown={(e) => { if (e.key === 'Enter') saveEdit(b); if (e.key === 'Escape') { delete editMap[key]; editMap = editMap; } }}
                            />
                            <button
                              type="button"
                              class="px-2 py-1 rounded-lg text-xs font-semibold bg-emerald-600 hover:bg-emerald-500 text-white transition-colors cursor-pointer"
                              on:click={() => saveEdit(b)} disabled={saving[key]}
                            >✓</button>
                          </span>
                        {:else}
                          <button
                            type="button"
                            class="px-2.5 py-1 rounded-lg border border-neutral-300 dark:border-neutral-700 bg-white dark:bg-neutral-900 font-bold text-sky-600 dark:text-sky-400 hover:border-indigo-500 transition-colors cursor-pointer shadow-sm"
                            on:click={() => startEdit(b)}
                            title="Click to edit limit"
                          >
                            {$currencySymbol}{(b.limit_cents / 100).toFixed(2)}
                          </button>
                        {/if}
                      </td>
                      <td class="px-4 py-3 text-right whitespace-nowrap">
                        {#if confirmDeleteKey === key}
                          <span class="inline-flex items-center gap-1.5">
                            <span class="text-xs text-neutral-500 dark:text-neutral-400">Remove?</span>
                            <button
                              type="button"
                              on:click={() => confirmBudgetDelete(b)}
                              class="px-2 py-0.5 rounded text-xs font-semibold bg-rose-600 hover:bg-rose-500 text-white transition-colors cursor-pointer"
                            >Yes</button>
                            <button
                              type="button"
                              on:click={cancelBudgetDelete}
                              class="px-2 py-0.5 rounded text-xs font-semibold bg-neutral-200 dark:bg-neutral-700 hover:bg-neutral-300 dark:hover:bg-neutral-600 text-neutral-800 dark:text-neutral-200 transition-colors cursor-pointer"
                            >No</button>
                          </span>
                        {:else}
                          <button
                            type="button"
                            on:click={() => requestBudgetDelete(b)}
                            title="Remove"
                            class="p-1.5 rounded-lg text-neutral-400 hover:text-rose-600 dark:hover:text-red-400 hover:bg-rose-50 dark:hover:bg-red-950/40 transition-all cursor-pointer"
                          >
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                            </svg>
                          </button>
                        {/if}
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}
        </div>
      {/if}

      <!-- Right Column: Add / Update Budget Limit Form (xl:col-span-2) -->
      {#if activeSubtab === 'all' || activeSubtab === 'add'}
        <div class="{activeSubtab === 'add' ? 'xl:col-span-5 max-w-2xl mx-auto' : 'xl:col-span-2'} card p-5 space-y-4">
          <div class="border-b border-neutral-200 dark:border-neutral-800 pb-3">
            <h3 class="text-sm font-bold text-neutral-900 dark:text-white flex items-center gap-2">
              <span>➕</span> Set / Update Limit
            </h3>
            <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">
              Standing default or month-specific override
            </p>
          </div>

          {#if addError}
            <div class="bg-rose-50 dark:bg-red-950/40 border border-rose-200 dark:border-red-800 text-rose-700 dark:text-red-400 rounded-xl px-3.5 py-2 text-xs">{addError}</div>
          {/if}
          {#if addSuccess}
            <div class="bg-emerald-50 dark:bg-emerald-950/40 border border-emerald-200 dark:border-emerald-800 text-emerald-700 dark:text-emerald-300 rounded-xl px-3.5 py-2 text-xs">
              ✓ Limit saved successfully.
            </div>
          {/if}

          <div class="space-y-4">
            <!-- Category Selector -->
            <div>
              <label for="bcat" class="block text-xs font-semibold text-neutral-700 dark:text-neutral-300 uppercase tracking-wider mb-1.5">
                Category
              </label>
              <select
                id="bcat"
                bind:value={newForm.category}
                class="select-field"
              >
                <option value="">— select category —</option>
                {#each $splits as s}
                  <option value={s.category}>{s.category}</option>
                {/each}
              </select>
            </div>

            <!-- Month / Horizon Selector -->
            <div>
              <label for="bmonth" class="block text-xs font-semibold text-neutral-700 dark:text-neutral-300 uppercase tracking-wider mb-1.5">
                Month
              </label>
              <input
                id="bmonth"
                type="text"
                bind:value={newForm.month}
                placeholder="ALL or YYYY-MM"
                class="input-field"
              />
              <div class="flex items-center gap-1.5 mt-2 flex-wrap">
                <button
                  type="button"
                  on:click={() => setQuickMonth('ALL')}
                  class="px-2 py-0.5 rounded text-[11px] font-medium border transition-colors cursor-pointer {newForm.month === 'ALL' ? 'bg-indigo-600 text-white border-indigo-400' : 'bg-neutral-100 dark:bg-neutral-800 border-neutral-200 dark:border-neutral-700 text-neutral-700 dark:text-neutral-300 hover:bg-neutral-200 dark:hover:bg-neutral-700'}"
                >
                  Standing (ALL)
                </button>
                <button
                  type="button"
                  on:click={() => setQuickMonth(monthStr)}
                  class="px-2 py-0.5 rounded text-[11px] font-medium border transition-colors cursor-pointer {newForm.month === monthStr ? 'bg-indigo-600 text-white border-indigo-400' : 'bg-neutral-100 dark:bg-neutral-800 border-neutral-200 dark:border-neutral-700 text-neutral-700 dark:text-neutral-300 hover:bg-neutral-200 dark:hover:bg-neutral-700'}"
                >
                  This Month ({monthStr})
                </button>
              </div>
            </div>

            <!-- Limit Amount & Quick Add Chips -->
            <div>
              <label for="blimit" class="block text-xs font-semibold text-neutral-700 dark:text-neutral-300 uppercase tracking-wider mb-1.5">
                Limit ({$currencySymbol})
              </label>
              <div class="relative">
                <span class="absolute left-3.5 top-1/2 -translate-y-1/2 text-neutral-500 font-bold text-sm">{$currencySymbol}</span>
                <input
                  id="blimit"
                  type="number"
                  min="0"
                  step="0.01"
                  bind:value={newForm.limit_euros}
                  placeholder="0.00"
                  class="input-field pl-8 font-semibold tabular-nums"
                />
              </div>

              <div class="flex items-center gap-1 mt-2 flex-wrap">
                {#each [50, 100, 250, 500] as eur}
                  <button
                    type="button"
                    on:click={() => setQuickAmount(eur)}
                    class="px-2 py-0.5 rounded bg-neutral-100 dark:bg-neutral-800 hover:bg-neutral-200 dark:hover:bg-neutral-700 border border-neutral-200 dark:border-neutral-700 text-[11px] text-neutral-700 dark:text-neutral-300 transition-colors cursor-pointer"
                  >
                    +{$currencySymbol}{eur}
                  </button>
                {/each}
              </div>
            </div>

            <button
              type="button"
              on:click={handleAdd}
              disabled={addSaving}
              class="btn-primary w-full py-2.5 mt-2"
            >
              {addSaving ? '…' : 'Save Limit'}
            </button>
          </div>
        </div>
      {/if}

    </div>
  {/if}

</div>
