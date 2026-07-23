<script>
  import { onMount } from 'svelte';
  import { budgets, splits, selectedMonth, currencySymbol } from './stores.js';
  import * as api from './api.js';
  import { upsertBudget, deleteBudget, fetchBudgetAnalytics } from './api.js';

  let budgetStatus = []; // BudgetStatusRow[]
  let editMap = {};      // category+month → pending cents input
  let saving = {};
  let error = '';
  let newForm = { category: '', month: 'ALL', limit_euros: '' };
  let addError = '';
  let addSaving = false;

  // Per-row delete confirmation
  let confirmDeleteKey = null;

  $: monthStr = $selectedMonth;

  async function loadStatus() {
    try {
      budgetStatus = await api.fetchBudgetAnalytics(monthStr);
    } catch (e) {
      error = e.message;
    }
  }

  onMount(loadStatus);
  $: monthStr, loadStatus();

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

  async function handleAdd() {
    addError = '';
    const limit_cents = Math.round(parseFloat(newForm.limit_euros) * 100);
    if (!newForm.category || isNaN(limit_cents) || limit_cents < 0) {
      addError = 'Choose a category and enter a valid amount.';
      return;
    }
    addSaving = true;
    try {
      await upsertBudget({ category: newForm.category, month: newForm.month, limit_cents });
      newForm = { category: '', month: 'ALL', limit_euros: '' };
      await loadStatus();
    } catch (e) {
      addError = e.message;
    } finally {
      addSaving = false;
    }
  }
</script>

<div>
  <h2 class="text-base font-semibold text-neutral-200 mb-1">Budget Manager</h2>
  <p class="text-xs text-neutral-500 mb-5">
    Set monthly spending limits per category. Use <code class="bg-neutral-800 text-sky-400 rounded px-1 py-0.5 text-[11px]">ALL</code> for a standing default;
    a YYYY-MM entry overrides it for that specific month.
  </p>

  {#if error}
    <div class="bg-red-950/40 border border-red-900/60 text-red-400 rounded-lg px-3 py-2 text-xs mb-4">{error}</div>
  {/if}

  <!-- ── Status bars for selected month ─────────────────── -->
  <div class="mb-6">
    <h3 class="text-xs font-semibold text-neutral-500 uppercase tracking-wider mb-3">Spending vs. Limits — {monthStr}</h3>
    {#if budgetStatus.length === 0}
      <div class="text-center text-neutral-600 text-sm py-6 border border-dashed border-neutral-800 rounded-xl">
        No budget data for this month.
      </div>
    {:else}
      <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-3">
        {#each budgetStatus as row}
          {@const color = statusColor(row.pct_used)}
          {@const barPct = Math.min(row.pct_used, 100)}
          {@const barClass = color === 'red' ? 'from-red-500 to-red-400' : color === 'yellow' ? 'from-yellow-500 to-yellow-400' : 'from-emerald-500 to-emerald-400'}
          {@const textClass = color === 'red' ? 'text-red-400' : color === 'yellow' ? 'text-yellow-400' : 'text-emerald-400'}
          <div class="bg-neutral-950 border border-neutral-800 rounded-xl p-3">
            <div class="flex items-center justify-between mb-1">
              <span class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wide truncate">{row.category}</span>
              <span class="text-xs font-semibold tabular-nums text-neutral-200">
                {$currencySymbol}{(row.actual_cents / 100).toFixed(0)}
                {#if row.limit_cents > 0}
                  <span class="text-neutral-600 font-normal">/ {$currencySymbol}{(row.limit_cents / 100).toFixed(0)}</span>
                {:else}
                  <span class="text-neutral-600 font-normal text-[10px]">(no limit)</span>
                {/if}
              </span>
            </div>
            {#if row.limit_cents > 0}
              <div class="h-1.5 bg-neutral-800 rounded-full overflow-hidden mt-1">
                <div
                  class="h-full rounded-full bg-gradient-to-r {barClass} transition-all duration-500"
                  style="width:{barPct}%"
                ></div>
              </div>
              <p class="text-[10px] {textClass} font-bold mt-0.5 text-right">{row.pct_used.toFixed(1)}%</p>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- ── Configured budget rows ─────────────────────────── -->
  <div>
    <h3 class="text-xs font-semibold text-neutral-500 uppercase tracking-wider mb-3">Configured Limits</h3>
    {#if $budgets.length === 0}
      <div class="text-center text-neutral-600 text-sm py-6 border border-dashed border-neutral-800 rounded-xl mb-4">
        No limits configured.
      </div>
    {:else}
      <div class="overflow-x-auto rounded-xl border border-neutral-800 mb-4">
        <table class="w-full text-sm border-collapse">
          <thead>
            <tr class="bg-neutral-950/60">
              <th class="text-left text-xs font-semibold text-neutral-500 uppercase tracking-wider px-4 py-2.5">Category</th>
              <th class="text-left text-xs font-semibold text-neutral-500 uppercase tracking-wider px-4 py-2.5">Month</th>
              <th class="text-left text-xs font-semibold text-neutral-500 uppercase tracking-wider px-4 py-2.5">Limit ({$currencySymbol})</th>
              <th class="px-4 py-2.5"></th>
            </tr>
          </thead>
          <tbody>
            {#each $budgets as b}
              {@const key = editKey(b.category, b.month)}
              <tr class="border-t border-neutral-800/70 hover:bg-neutral-800/30 transition-colors group">
                <td class="px-4 py-3 font-medium text-neutral-200">{b.category}</td>
                <td class="px-4 py-3 text-neutral-500 text-xs">{b.month}</td>
                <td class="px-4 py-3 tabular-nums">
                  {#if key in editMap}
                    <span class="inline-flex items-center gap-1.5">
                      <input
                        class="w-24 bg-neutral-800 border border-indigo-500 rounded-lg px-2 py-1 text-sm text-neutral-100
                               focus:outline-none focus:ring-1 focus:ring-indigo-500 transition-colors"
                        type="number" min="0" step="0.01"
                        bind:value={editMap[key]}
                        on:keydown={(e) => { if (e.key === 'Enter') saveEdit(b); if (e.key === 'Escape') { delete editMap[key]; editMap = editMap; } }}
                      />
                      <button
                        class="px-2 py-1 rounded-lg text-xs font-semibold bg-emerald-600 hover:bg-emerald-500 text-white transition-colors"
                        on:click={() => saveEdit(b)} disabled={saving[key]}
                      >✓</button>
                    </span>
                  {:else}
                    <button
                      class="px-2 py-1 rounded border border-neutral-700 text-xs text-sky-400 hover:border-indigo-500 transition-colors"
                      on:click={() => startEdit(b)}
                    >{$currencySymbol}{(b.limit_cents / 100).toFixed(2)}</button>
                  {/if}
                </td>
                <td class="px-4 py-3 text-right whitespace-nowrap">
                  {#if confirmDeleteKey === key}
                    <span class="inline-flex items-center gap-1.5">
                      <span class="text-xs text-neutral-400">Remove?</span>
                      <button
                        on:click={() => confirmBudgetDelete(b)}
                        class="px-2 py-0.5 rounded text-xs font-semibold bg-red-600 hover:bg-red-500 transition-colors"
                      >Yes</button>
                      <button
                        on:click={cancelBudgetDelete}
                        class="px-2 py-0.5 rounded text-xs font-semibold bg-neutral-700 hover:bg-neutral-600 transition-colors"
                      >No</button>
                    </span>
                  {:else}
                    <button
                      on:click={() => requestBudgetDelete(b)}
                      title="Remove"
                      class="opacity-0 group-hover:opacity-100 p-1 rounded-lg text-neutral-500
                             hover:text-red-400 hover:bg-red-950/40 transition-all duration-150"
                    >
                      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                        <polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14H6L5 6"/><path d="M10 11v6M14 11v6"/>
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

    <!-- Add new budget row -->
    <div class="bg-neutral-950/60 border border-neutral-800 rounded-xl p-5">
      <h4 class="text-sm font-semibold text-neutral-200 mb-4">Add / Update Limit</h4>
      {#if addError}
        <div class="bg-red-950/40 border border-red-900/60 text-red-400 rounded-lg px-3 py-2 text-xs mb-3">{addError}</div>
      {/if}
      <div class="flex flex-wrap gap-3 items-end">
        <div class="flex flex-col gap-1 flex-1 min-w-[130px]">
          <label for="bcat" class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wider">Category</label>
          <select
            id="bcat" bind:value={newForm.category}
            class="bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100
                   focus:outline-none focus:border-indigo-500 transition-colors"
          >
            <option value="">— select —</option>
            {#each $splits as s}
              <option value={s.category}>{s.category}</option>
            {/each}
          </select>
        </div>
        <div class="flex flex-col gap-1 min-w-[130px] max-w-[160px]">
          <label for="bmonth" class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wider">Month</label>
          <input
            id="bmonth" type="text" bind:value={newForm.month} placeholder="ALL or YYYY-MM"
            class="bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100
                   placeholder-neutral-600 focus:outline-none focus:border-indigo-500 focus:ring-1
                   focus:ring-indigo-500 transition-colors"
          />
        </div>
        <div class="flex flex-col gap-1 min-w-[110px] max-w-[140px]">
          <label for="blimit" class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wider">Limit ({$currencySymbol})</label>
          <input
            id="blimit" type="number" min="0" step="0.01"
            bind:value={newForm.limit_euros} placeholder="0.00"
            class="bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100
                   placeholder-neutral-600 focus:outline-none focus:border-indigo-500 focus:ring-1
                   focus:ring-indigo-500 transition-colors"
          />
        </div>
        <button
          on:click={handleAdd} disabled={addSaving}
          class="bg-gradient-to-r from-indigo-600 to-violet-600
                 hover:from-indigo-500 hover:to-violet-500
                 text-white text-sm font-semibold rounded-lg px-5 py-2
                 disabled:opacity-50 disabled:cursor-not-allowed
                 transition-all duration-150 shadow-md shadow-indigo-900/30 active:scale-[0.98]"
        >
          {addSaving ? '…' : 'Save'}
        </button>
      </div>
    </div>
  </div>
</div>
