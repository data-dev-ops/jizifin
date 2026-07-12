<script>
  /**
   * IncomeTab.svelte
   *
   * Full-featured income management panel:
   *   - Monthly ledger of income entries with delete
   *   - Log Income form (name, amount, who, category, date)
   *   - Inline income category manager (add / remove)
   *
   * All encryption/decryption is handled transparently by api.js.
   * This component works exclusively with plaintext values from the stores.
   */

  import { onMount } from 'svelte';
  import { selectedMonth, users, incomeEntries, incomeCategories, currencySymbol } from './stores.js';
  import {
    fetchIncome,
    deleteIncome,
    createIncome,
    fetchIncomeCategories,
    createIncomeCategory,
    deleteIncomeCategory,
  } from './api.js';

  // ── Form state ──────────────────────────────────────────────────────────────
  let formName        = '';
  let formAmountEur   = '';
  let formWho         = '';
  let formCategory    = '';
  let formDate        = new Date().toISOString().slice(0, 10);
  let submitting      = false;
  let formError       = '';
  let formSuccess     = false;

  // ── Category manager state ──────────────────────────────────────────────────
  let newCatName     = '';
  let addingCat      = false;
  let catError       = '';
  let deletingCat    = '';   // category name currently being deleted

  // ── Ledger state ───────────────────────────────────────────────────────────
  let deletingId     = null;  // id being deleted
  let confirmId      = null;  // id pending confirmation

  // ── Derived ────────────────────────────────────────────────────────────────
  $: activeUsers = $users.filter((u) => u.is_active);
  $: if (activeUsers.length && !formWho) formWho = activeUsers[0]?.name ?? '';
  $: if ($incomeCategories.length && !formCategory) formCategory = $incomeCategories[0]?.category ?? '';

  // ── Formatting helpers ─────────────────────────────────────────────────────
  function fmt(cents) {
    return `${$currencySymbol}${(cents / 100).toFixed(2)}`;
  }

  function fmtDate(iso) {
    return new Date(iso + 'T00:00:00').toLocaleDateString(undefined, {
      day: 'numeric', month: 'short', year: 'numeric',
    });
  }

  function userColor(name) {
    return $users.find((u) => u.name === name)?.color ?? '#6366f1';
  }

  function userInitial(name) {
    return name ? name[0].toUpperCase() : '?';
  }

  /** Sequential palette for income category badges */
  const CAT_COLOURS = [
    '#6366f1','#8b5cf6','#ec4899','#f59e0b','#10b981','#3b82f6','#ef4444',
  ];
  function catColour(cat) {
    const idx = $incomeCategories.findIndex((c) => c.category === cat);
    return CAT_COLOURS[idx % CAT_COLOURS.length] ?? '#6366f1';
  }

  // ── Month change ────────────────────────────────────────────────────────────
  $: fetchIncome($selectedMonth);

  // ── Submit income entry ─────────────────────────────────────────────────────
  async function submit() {
    formError   = '';
    formSuccess = false;
    const amountCents = Math.round(parseFloat(formAmountEur) * 100);
    if (!formName.trim())                   { formError = 'Name is required.';          return; }
    if (!amountCents || amountCents <= 0)   { formError = 'Enter a positive amount.';   return; }
    if (!formWho)                           { formError = 'Select a person.';            return; }
    if (!formCategory)                      { formError = 'Select a category.';          return; }
    if (!formDate)                          { formError = 'Select a date.';              return; }
    submitting = true;
    try {
      await createIncome(
        [{ name: formName.trim(), amount_cents: amountCents, who: formWho, category: formCategory, income_date: formDate }],
        $selectedMonth,
      );
      await fetchIncome($selectedMonth);
      formName      = '';
      formAmountEur = '';
      formSuccess   = true;
      setTimeout(() => (formSuccess = false), 3000);
    } catch (err) {
      formError = err.message;
    } finally {
      submitting = false;
    }
  }

  // ── Delete income entry ─────────────────────────────────────────────────────
  async function confirmDelete(id) {
    if (confirmId !== id) { confirmId = id; return; }
    confirmId  = null;
    deletingId = id;
    try {
      await deleteIncome(id);
    } catch (err) {
      console.error(err);
    } finally {
      deletingId = null;
    }
  }

  // ── Category manager ────────────────────────────────────────────────────────
  async function addCategory() {
    catError = '';
    const name = newCatName.trim();
    if (!name) { catError = 'Category name required.'; return; }
    addingCat = true;
    try {
      await createIncomeCategory(name);
      newCatName = '';
      if (!formCategory) formCategory = name;
    } catch (err) {
      catError = err.message;
    } finally {
      addingCat = false;
    }
  }

  async function removeCategory(name) {
    deletingCat = name;
    try {
      await deleteIncomeCategory(name);
      if (formCategory === name) formCategory = $incomeCategories[0]?.category ?? '';
    } catch (err) {
      catError = err.message;
    } finally {
      deletingCat = '';
    }
  }

  function handleCatKeydown(e) {
    if (e.key === 'Enter') addCategory();
  }
</script>

<!-- ── Page header ──────────────────────────────────────────────────────────── -->
<div class="mb-6">
  <h2 class="text-lg font-bold text-neutral-100">Income</h2>
  <p class="text-xs text-neutral-500 mt-0.5">Track household income by person and category.</p>
</div>

<!-- ── Top row: Form + Ledger ──────────────────────────────────────────────── -->
<div class="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">

  <!-- Log Income form -->
  <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-5">
    <p class="text-xs font-semibold text-neutral-400 uppercase tracking-wider mb-4">Log Income</p>

    <div class="space-y-3">
      <!-- Name -->
      <div>
        <label for="income-name" class="block text-xs text-neutral-500 mb-1">Description</label>
        <input
          id="income-name"
          type="text"
          bind:value={formName}
          placeholder="e.g. January salary"
          class="w-full bg-neutral-800 border border-neutral-700 rounded-xl px-3 py-2 text-sm text-neutral-200
                 placeholder-neutral-600 focus:outline-none focus:ring-1 focus:ring-indigo-500 transition-colors"
        />
      </div>

      <!-- Amount + Who (2-col) -->
      <div class="grid grid-cols-2 gap-3">
        <div>
          <label for="income-amount" class="block text-xs text-neutral-500 mb-1">Amount ({$currencySymbol})</label>
          <input
            id="income-amount"
            type="number" min="0.01" step="0.01"
            bind:value={formAmountEur}
            placeholder="0.00"
            class="w-full bg-neutral-800 border border-neutral-700 rounded-xl px-3 py-2 text-sm text-neutral-200
                   tabular-nums focus:outline-none focus:ring-1 focus:ring-indigo-500 transition-colors"
          />
        </div>
        <div>
          <label for="income-who" class="block text-xs text-neutral-500 mb-1">Person</label>
          <select
            id="income-who"
            bind:value={formWho}
            class="w-full bg-neutral-800 border border-neutral-700 rounded-xl px-3 py-2 text-sm text-neutral-200
                   focus:outline-none focus:ring-1 focus:ring-indigo-500 transition-colors"
          >
            {#each activeUsers as u}
              <option value={u.name}>{u.name}</option>
            {/each}
          </select>
        </div>
      </div>

      <!-- Category + Date (2-col) -->
      <div class="grid grid-cols-2 gap-3">
        <div>
          <label for="income-category" class="block text-xs text-neutral-500 mb-1">Category</label>
          <select
            id="income-category"
            bind:value={formCategory}
            class="w-full bg-neutral-800 border border-neutral-700 rounded-xl px-3 py-2 text-sm text-neutral-200
                   focus:outline-none focus:ring-1 focus:ring-indigo-500 transition-colors"
          >
            {#each $incomeCategories as c}
              <option value={c.category}>{c.category}</option>
            {/each}
            {#if $incomeCategories.length === 0}
              <option disabled value="">— add categories below —</option>
            {/if}
          </select>
        </div>
        <div>
          <label for="income-date" class="block text-xs text-neutral-500 mb-1">Date</label>
          <input
            id="income-date"
            type="date"
            bind:value={formDate}
            class="w-full bg-neutral-800 border border-neutral-700 rounded-xl px-3 py-2 text-sm text-neutral-200
                   focus:outline-none focus:ring-1 focus:ring-indigo-500 transition-colors"
          />
        </div>
      </div>

      {#if formError}
        <p class="text-xs text-red-400 bg-red-950/40 border border-red-900/50 rounded-lg px-3 py-2">{formError}</p>
      {/if}
      {#if formSuccess}
        <p class="text-xs text-emerald-400 bg-emerald-950/40 border border-emerald-900/50 rounded-lg px-3 py-2">
          ✓ Entry saved.
        </p>
      {/if}

      <button
        id="income-submit"
        on:click={submit}
        disabled={submitting || $incomeCategories.length === 0}
        class="w-full py-2.5 rounded-xl text-sm font-semibold bg-indigo-600 hover:bg-indigo-500
               disabled:opacity-40 disabled:cursor-not-allowed transition-colors active:scale-[0.98]"
      >
        {submitting ? 'Saving…' : 'Save Income Entry'}
      </button>
    </div>
  </div>

  <!-- Monthly Ledger -->
  <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-5">
    <p class="text-xs font-semibold text-neutral-400 uppercase tracking-wider mb-4">
      Ledger — {$selectedMonth ?? 'Current Month'}
    </p>

    {#if $incomeEntries.length === 0}
      <div class="flex flex-col items-center justify-center py-10 text-center">
        <p class="text-neutral-500 text-sm">No income recorded this month.</p>
        <p class="text-neutral-700 text-xs mt-1">Use the form to log an entry.</p>
      </div>
    {:else}
      <div class="space-y-2">
        {#each $incomeEntries as entry (entry.id)}
          {@const color = userColor(entry.who)}
          {@const badgeColor = catColour(entry.category)}
          <div
            class="flex items-center gap-3 rounded-xl bg-neutral-800/50 border border-neutral-800 px-3 py-2.5
                   hover:border-neutral-700 transition-colors group"
          >
            <div
              class="w-7 h-7 rounded-full flex items-center justify-center text-[10px] font-bold shrink-0"
              style="background-color: {color}"
            >
              {userInitial(entry.who)}
            </div>

            <div class="flex-1 min-w-0">
              <p class="text-sm font-semibold text-neutral-200 truncate">{entry.name}</p>
              <div class="flex items-center gap-2 mt-0.5 flex-wrap">
                <span
                  class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-semibold"
                  style="background-color: {badgeColor}22; color: {badgeColor}"
                >
                  {entry.category}
                </span>
                <span class="text-[11px] text-neutral-600">{fmtDate(entry.income_date)}</span>
              </div>
            </div>

            <p class="text-sm font-bold tabular-nums shrink-0" style="color: {color}">
              {fmt(entry.amount_cents)}
            </p>

            <button
              id="delete-income-{entry.id}"
              on:click={() => confirmDelete(entry.id)}
              disabled={deletingId === entry.id}
              title={confirmId === entry.id ? 'Click again to confirm' : 'Delete entry'}
              class="shrink-0 w-7 h-7 rounded-lg flex items-center justify-center text-xs
                     transition-all opacity-0 group-hover:opacity-100
                     {confirmId === entry.id
                       ? 'bg-red-600/80 text-white opacity-100'
                       : 'bg-neutral-700 text-neutral-400 hover:bg-red-900/60 hover:text-red-400'}
                     disabled:opacity-30"
            >
              {deletingId === entry.id ? '…' : confirmId === entry.id ? '!' : '×'}
            </button>
          </div>
        {/each}
      </div>

      <div class="mt-4 pt-3 border-t border-neutral-800 flex items-center justify-between">
        <span class="text-xs text-neutral-500">Total this month</span>
        <span class="text-sm font-bold tabular-nums text-indigo-400">
          {fmt($incomeEntries.reduce((s, e) => s + e.amount_cents, 0))}
        </span>
      </div>
    {/if}
  </div>
</div>

<!-- ── Category Manager ─────────────────────────────────────────────────────── -->
<div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-5">
  <p class="text-xs font-semibold text-neutral-400 uppercase tracking-wider mb-4">Income Categories</p>

  <div class="flex gap-2 mb-4">
    <input
      id="income-cat-input"
      type="text"
      bind:value={newCatName}
      on:keydown={handleCatKeydown}
      placeholder="New category name…"
      class="flex-1 bg-neutral-800 border border-neutral-700 rounded-xl px-3 py-2 text-sm text-neutral-200
             placeholder-neutral-600 focus:outline-none focus:ring-1 focus:ring-indigo-500 transition-colors"
    />
    <button
      id="income-cat-add"
      on:click={addCategory}
      disabled={addingCat}
      class="px-4 py-2 rounded-xl text-sm font-semibold bg-indigo-600 hover:bg-indigo-500
             disabled:opacity-40 disabled:cursor-not-allowed transition-colors active:scale-[0.98]"
    >
      {addingCat ? '…' : 'Add'}
    </button>
  </div>

  {#if catError}
    <p class="text-xs text-red-400 mb-3">{catError}</p>
  {/if}

  {#if $incomeCategories.length === 0}
    <p class="text-sm text-neutral-600">No categories yet. Add one above.</p>
  {:else}
    <div class="flex flex-wrap gap-2">
      {#each $incomeCategories as c (c.category)}
        {@const col = catColour(c.category)}
        <div
          class="flex items-center gap-1.5 pl-2.5 pr-1.5 py-1 rounded-lg border text-xs font-semibold"
          style="background-color: {col}18; border-color: {col}40; color: {col}"
        >
          {c.category}
          <button
            id="income-cat-delete-{c.category}"
            on:click={() => removeCategory(c.category)}
            disabled={deletingCat === c.category}
            title="Remove category"
            class="w-4 h-4 rounded flex items-center justify-center
                   hover:bg-red-600/30 hover:text-red-400 transition-colors disabled:opacity-30"
          >
            {deletingCat === c.category ? '…' : '×'}
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>
