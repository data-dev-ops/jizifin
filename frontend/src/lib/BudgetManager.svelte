<script>
  /**
   * BudgetManager.svelte — budget limits (SCD2 by month) with the Auditor's status
   * report and trailing-spend proposals for the selected month.
   */

  import { audit, budgetApi } from './api.js';
  import { money, fmtMonth, fmtPct, toCents, centsInput } from './format.js';
  import { selectedMonth, ledgerVersion, budgets, expenseCategories, categoryName, activeMembers, accounts } from './stores.js';

  let activeSubtab = 'all';
  let scope = 'ALL'; // 'ALL' | 'MEMBER:<id>' | 'ACCOUNT:<id>'
  let report = { rows: [], summary: null };
  let proposals = [];
  let error = '';

  $: load($selectedMonth, scope, $ledgerVersion);

  function scopeFilter(token) {
    const [kind, id] = token.split(':');
    if (kind === 'MEMBER') return { members: [Number(id)] };
    if (kind === 'ACCOUNT') return { account: Number(id) };
    return {};
  }

  async function load(month, token) {
    error = '';
    try {
      const [status, props] = await Promise.all([audit.budgetStatus(month, scopeFilter(token)), audit.budgetProposals(month)]);
      [report, proposals] = [status.value, props.value];
    } catch (e) {
      error = e.message;
    }
  }

  const level = (pct) => (pct >= 90 ? 'red' : pct >= 70 ? 'yellow' : 'green');
  const BAR = { red: 'from-rose-500 to-rose-400', yellow: 'from-amber-400 to-amber-300', green: 'from-emerald-500 to-emerald-400' };
  const TEXT = { red: 'text-rose-600 dark:text-red-400', yellow: 'text-amber-600 dark:text-yellow-400', green: 'text-emerald-600 dark:text-emerald-400' };

  // ── Form ───────────────────────────────────────────────────────────────────
  let form = { id: null, category: null, valid_from: $selectedMonth, standing: true, limit: '' };
  let formError = '';
  let formSuccess = false;
  let matrixFilter = '';
  let confirmDeleteId = null;

  $: matrixRows = $budgets.filter((b) => $categoryName(b.category).toLowerCase().includes(matrixFilter.toLowerCase()));

  function edit(budget) {
    form = { id: budget.id, category: budget.category, valid_from: budget.valid_from, standing: budget.valid_to == null, limit: centsInput(budget.limit) };
    activeSubtab = 'add';
  }

  function adopt(proposal) {
    form = { id: null, category: proposal.category, valid_from: $selectedMonth, standing: true, limit: centsInput(proposal.proposed), source: 'accepted_proposal' };
    activeSubtab = 'add';
  }

  async function save() {
    formError = '';
    formSuccess = false;
    const payload = {
      category: form.category,
      valid_from: form.valid_from,
      valid_to: form.standing ? null : form.valid_from,
      limit: toCents(form.limit),
      source: form.source ?? 'manual',
    };
    try {
      if (form.id) await budgetApi.update(form.id, payload);
      else await budgetApi.create(payload);
      formSuccess = true;
      form = { id: null, category: null, valid_from: $selectedMonth, standing: true, limit: '' };
    } catch (e) {
      formError = e.message;
    }
  }

  async function remove(id) {
    try {
      await budgetApi.remove(id);
      confirmDeleteId = null;
    } catch (e) {
      error = e.message;
    }
  }
</script>

<div class="space-y-6">
  <header class="page-header">
    <div>
      <h1 class="page-title flex items-center gap-2.5"><span>🎯</span> Budget Tracker & Spending Limits</h1>
      <p class="page-subtitle">Category ceilings, live budget health and proposals from your last three months of spend.</p>
    </div>
  </header>

  {#if report.summary}
    {@const s = report.summary}
    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
      <div class="card p-4 sm:p-5">
        <p class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider mb-2">Total Monthly Limit</p>
        <p class="font-bold text-neutral-900 dark:text-white tabular-nums text-xl sm:text-2xl">{$money(s.total_limit, true)}</p>
        <p class="mt-3 pt-2.5 border-t border-neutral-200 dark:border-neutral-800/80 text-[11px] text-neutral-500">Across {s.configured} configured limits</p>
      </div>
      <div class="card p-4 sm:p-5">
        <p class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider mb-2">Total Spent</p>
        <p class="font-bold text-neutral-900 dark:text-white tabular-nums text-xl sm:text-2xl">{$money(s.total_spent, true)}</p>
        <p class="mt-3 pt-2.5 border-t border-neutral-200 dark:border-neutral-800/80 text-[11px] text-neutral-500">Overall usage: {fmtPct(s.usage_pct)}</p>
      </div>
      <div class="card p-4 sm:p-5">
        <p class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider mb-2">Remaining Buffer</p>
        <p class="font-bold tabular-nums text-xl sm:text-2xl {s.total_remaining > 0 ? 'text-emerald-600 dark:text-emerald-400' : 'text-rose-600 dark:text-red-400'}">{$money(s.total_remaining, true)}</p>
        <p class="mt-3 pt-2.5 border-t border-neutral-200 dark:border-neutral-800/80 text-[11px] text-neutral-500">{s.total_remaining > 0 ? 'Available buffer' : 'Cap exceeded'}</p>
      </div>
      <div class="card p-4 sm:p-5">
        <div class="flex items-center justify-between gap-2 mb-2">
          <p class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">Health Index</p>
          <span class="badge-emerald">{fmtPct(s.health_pct)} Safe</span>
        </div>
        <p class="font-bold text-neutral-900 dark:text-white tabular-nums text-xl sm:text-2xl">{s.safe} / {s.configured}</p>
        <p class="mt-3 pt-2.5 border-t border-neutral-200 dark:border-neutral-800/80 text-[11px] text-neutral-500">Categories below 90% usage</p>
      </div>
    </div>
  {/if}

  <div class="border-b border-neutral-200 dark:border-neutral-800 pb-3">
    <div class="flex items-center gap-2 overflow-x-auto">
      {#each [['all', '🌟 All-in-One Studio'], ['health', '📊 Budget Health'], ['matrix', `🎯 Configured Limits (${$budgets.length})`], ['proposals', `💡 Proposals (${proposals.length})`], ['add', '➕ Add / Update Limit']] as [id, label]}
        <button type="button" on:click={() => (activeSubtab = id)} class="px-3.5 py-1.5 rounded-xl text-xs font-semibold whitespace-nowrap {activeSubtab === id ? 'bg-indigo-600 text-white' : 'bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400'}">{label}</button>
      {/each}
    </div>
  </div>

  {#if error}<div class="bg-rose-50 dark:bg-red-950/40 border border-rose-200 dark:border-red-900/60 text-rose-700 dark:text-red-400 rounded-xl px-4 py-3 text-xs">{error}</div>{/if}

  {#if activeSubtab === 'all' || activeSubtab === 'health'}
    <div class="card p-3.5 flex items-center gap-1.5 flex-wrap">
      <span class="text-xs font-bold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mr-2">Spending Scope:</span>
      <button type="button" on:click={() => (scope = 'ALL')} class="px-3 py-1 rounded-lg text-xs font-semibold border {scope === 'ALL' ? 'bg-indigo-600 text-white border-indigo-400' : 'border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400'}">👥 Whole Household</button>
      {#each $activeMembers as m (m.id)}
        <button type="button" on:click={() => (scope = `MEMBER:${m.id}`)} class="px-3 py-1 rounded-lg text-xs font-semibold border {scope === `MEMBER:${m.id}` ? 'bg-indigo-600 text-white border-indigo-400' : 'border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400'}">👤 {m.name}</button>
      {/each}
      {#each $accounts as a (a.id)}
        <button type="button" on:click={() => (scope = `ACCOUNT:${a.id}`)} class="px-3 py-1 rounded-lg text-xs font-semibold border {scope === `ACCOUNT:${a.id}` ? 'bg-indigo-600 text-white border-indigo-400' : 'border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400'}">🏦 {a.name}</button>
      {/each}
    </div>

    {#if report.rows.length === 0}
      <div class="card empty-state-box py-10"><p class="text-neutral-700 dark:text-neutral-300 text-sm font-semibold">No budgets effective this month.</p></div>
    {:else}
      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
        {#each report.rows as row (row.category)}
          {@const l = level(row.pct_used)}
          <div class="card p-4 space-y-3 flex flex-col justify-between">
            <div>
              <span class="text-xs font-bold text-neutral-900 dark:text-white uppercase tracking-wide truncate">{$categoryName(row.category)}</span>
              <div class="flex items-baseline justify-between text-xs mt-2">
                <span class="text-sm font-bold tabular-nums text-neutral-900 dark:text-neutral-100">{$money(row.spent)}</span>
                <span class="text-neutral-500">cap: {$money(row.limit, true)}</span>
              </div>
              <div class="h-2 bg-neutral-200 dark:bg-neutral-950 rounded-full overflow-hidden mt-2.5">
                <div class="h-full rounded-full bg-gradient-to-r {BAR[l]}" style="width:{Math.min(row.pct_used, 100)}%"></div>
              </div>
              <div class="flex items-center justify-between mt-1 text-[11px]">
                <span class="text-neutral-500">{row.remaining > 0 ? `Buffer: ${$money(row.remaining, true)}` : '🚨 Over budget'}</span>
                <span class="{TEXT[l]} font-bold tabular-nums">{fmtPct(row.pct_used, 1)}</span>
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  {/if}

  {#if activeSubtab === 'all' || activeSubtab === 'matrix'}
    <div class="card p-5 space-y-4">
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-200 dark:border-neutral-800 pb-3">
        <h3 class="text-sm font-bold text-neutral-800 dark:text-neutral-200">Configured Limits</h3>
        <input type="text" bind:value={matrixFilter} placeholder="Search category…" class="input-field py-1 px-3 text-xs max-w-[180px]" />
      </div>
      {#each matrixRows as b (b.id)}
        <div class="card-sub p-2.5 flex flex-wrap items-center gap-3 text-xs">
          <span class="font-semibold text-neutral-900 dark:text-white flex-1">{$categoryName(b.category)}</span>
          <span class="text-neutral-500">{b.valid_to == null ? `standing from ${fmtMonth(b.valid_from)}` : fmtMonth(b.valid_from)}</span>
          {#if b.source === 'accepted_proposal'}<span class="badge-indigo text-[9px]">proposal</span>{/if}
          <span class="font-bold tabular-nums text-neutral-900 dark:text-white">{$money(b.limit)}</span>
          <button type="button" on:click={() => edit(b)} class="text-indigo-600 dark:text-indigo-400">Edit</button>
          {#if confirmDeleteId === b.id}
            <button type="button" on:click={() => remove(b.id)} class="px-2 py-0.5 rounded bg-rose-600 text-white">Confirm</button>
            <button type="button" on:click={() => (confirmDeleteId = null)} class="text-neutral-500">✕</button>
          {:else}
            <button type="button" on:click={() => (confirmDeleteId = b.id)} class="text-rose-600 dark:text-red-400">Delete</button>
          {/if}
        </div>
      {:else}
        <p class="text-xs text-neutral-500">No limits configured.</p>
      {/each}
    </div>
  {/if}

  {#if activeSubtab === 'all' || activeSubtab === 'proposals'}
    <div class="card p-5 space-y-3">
      <h3 class="text-sm font-bold text-neutral-800 dark:text-neutral-200">💡 Proposed limits (mean of the last 3 months, rounded up)</h3>
      {#each proposals as p (p.category)}
        <div class="card-sub p-2.5 flex flex-wrap items-center gap-3 text-xs">
          <span class="font-semibold text-neutral-900 dark:text-white flex-1">{$categoryName(p.category)}</span>
          <span class="text-neutral-500 tabular-nums">{p.history.map((c) => $money(c, true)).join(' · ')}</span>
          <span class="font-bold tabular-nums text-indigo-600 dark:text-indigo-400">{$money(p.proposed)}</span>
          <button type="button" on:click={() => adopt(p)} class="btn-secondary text-xs">Use</button>
        </div>
      {:else}
        <p class="text-xs text-neutral-500">No spending history to propose from.</p>
      {/each}
    </div>
  {/if}

  {#if activeSubtab === 'all' || activeSubtab === 'add'}
    <form class="card p-5 space-y-3 max-w-xl" on:submit|preventDefault={save}>
      <h3 class="text-sm font-bold text-neutral-800 dark:text-neutral-200">{form.id ? 'Update limit' : 'Add limit'}</h3>
      <label class="block text-xs text-neutral-600 dark:text-neutral-400">Category
        <select id="budget-category" bind:value={form.category} class="select-field mt-1">
          <option value={null}>— Select —</option>
          {#each $expenseCategories as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
        </select>
      </label>
      <div class="grid grid-cols-2 gap-3">
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">From month <input id="budget-month" type="month" bind:value={form.valid_from} class="input-field mt-1" /></label>
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">Limit <input id="budget-limit" type="text" inputmode="decimal" bind:value={form.limit} class="input-field mt-1 tabular-nums" /></label>
      </div>
      <label class="flex items-center gap-2 text-xs text-neutral-600 dark:text-neutral-400">
        <input id="budget-standing" type="checkbox" bind:checked={form.standing} class="accent-indigo-600" /> Standing (applies to every later month)
      </label>
      {#if formError}<p class="text-xs text-rose-600 dark:text-red-400">{formError}</p>{/if}
      {#if formSuccess}<p class="text-xs text-emerald-600 dark:text-emerald-400">✓ Saved.</p>{/if}
      <button id="budget-save" type="submit" class="btn-primary">Save limit</button>
    </form>
  {/if}
</div>
