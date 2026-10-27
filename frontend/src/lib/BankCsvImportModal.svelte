<script>
  /**
   * BankCsvImportModal.svelte — bank statement import. Parsing, the category cascade,
   * micro-debit grouping, totals and entry drafts run in jizifin-wasm; duplicates are
   * checked by the server before the batch is written.
   */
  import { createEventDispatcher } from 'svelte';
  import { activeMembers, expenseCategories, incomeCategories, tags } from './stores.js';
  import { parseBankCsv, summarize, cascadeFrom, microIndices, draftsFor } from './csv.js';
  import { entryApi } from './api.js';
  import { money } from './format.js';
  import DuplicatePromptModal from './DuplicatePromptModal.svelte';

  const dispatch = createEventDispatcher();
  export let isOpen = false;

  /** Debits below this are offered for bundling. */
  const MICRO_CENTS = 500;
  const BUNDLE_CATEGORY = 'OTHER';

  let selectedBank = 'ING';
  let selectedPayer = null;
  $: if (!selectedPayer && $activeMembers.length) selectedPayer = $activeMembers[0].id;

  let fileInput;
  let isDragging = false;
  let fileName = '';
  let parsing = false;
  let parseError = null;

  let transactions = [];
  let stats = null;
  let summary = null;
  let searchQuery = '';
  let filterOnlyUncategorized = false;
  let cascadeNotice = '';
  let cascadeNoticeTimer = null;

  let importing = false;
  let importError = null;
  let importSuccessCount = null;

  let showDuplicateModal = false;
  let duplicateItems = [];
  let pendingDrafts = null;
  let duplicateIndices = [];
  let duplicateRows = new Set();

  $: summarize(transactions).then((s) => (summary = s)).catch(() => null);
  $: hasUncategorized = (summary?.uncategorized ?? 0) > 0;
  $: uncategorizedCount = summary?.uncategorized ?? 0;
  $: bundleCategory = $expenseCategories.find((c) => c.name === BUNDLE_CATEGORY)?.id ?? null;

  function resetState() {
    fileName = '';
    parsing = false;
    parseError = null;
    transactions = [];
    stats = null;
    searchQuery = '';
    filterOnlyUncategorized = false;
    cascadeNotice = '';
    importing = false;
    importError = null;
    importSuccessCount = null;
    showDuplicateModal = false;
    duplicateItems = [];
    pendingDrafts = null;
    duplicateRows = new Set();
  }

  function handleClose() {
    resetState();
    dispatch('close');
  }

  function handleKeydown(e) {
    if (e.key === 'Escape' && isOpen && !importing) handleClose();
  }

  function handleDragOver(e) {
    e.preventDefault();
    isDragging = true;
  }

  function handleDragLeave() {
    isDragging = false;
  }

  function handleDrop(e) {
    e.preventDefault();
    isDragging = false;
    const file = e.dataTransfer?.files?.[0];
    if (file) processFile(file);
  }

  function handleFileInputChange(e) {
    const file = e.target.files?.[0];
    if (file) processFile(file);
  }

  async function processFile(file) {
    if (!file.name.toLowerCase().endsWith('.csv')) {
      parseError = 'Please upload a valid .csv file.';
      return;
    }
    parseError = null;
    parsing = true;
    fileName = file.name;
    try {
      const result = await parseBankCsv(await file.text(), selectedBank);
      transactions = result.transactions.map((t) => ({ ...t, member: selectedPayer, category: null, tag: null }));
      stats = result.stats;
    } catch (err) {
      parseError = err.message ?? String(err);
      transactions = [];
      stats = null;
    } finally {
      parsing = false;
    }
  }

  function notice(text) {
    clearTimeout(cascadeNoticeTimer);
    cascadeNotice = text;
    cascadeNoticeTimer = setTimeout(() => (cascadeNotice = ''), 3500);
  }

  async function onCategoryChange(index, category) {
    const targets = await cascadeFrom(transactions, index);
    targets.forEach((i) => { transactions[i].category = category; });
    transactions = transactions;
    if (targets.length > 1) notice(`Updated ${targets.length} rows for "${transactions[index].beneficiary}"`);
  }

  function toggleAllIncluded(value) {
    transactions = transactions.map((t) => ({ ...t, include: value }));
  }

  async function groupMicroExpenses() {
    const targets = await microIndices(transactions, MICRO_CENTS);
    targets.forEach((i) => { transactions[i].groupToOther = true; });
    transactions = transactions;
    if (targets.length) notice(`Grouped ${targets.length} micro-expenses (< ${$money(MICRO_CENTS)}) to "${BUNDLE_CATEGORY}"`);
  }

  $: needle = searchQuery.trim().toLowerCase();
  $: filteredTransactions = transactions
    .map((t, index) => ({ t, index }))
    .filter(({ t }) => !filterOnlyUncategorized || (t.include && t.category == null && !t.groupToOther))
    .filter(({ t }) => !needle || [t.beneficiary, t.rawDescription, t.displayDate].some((v) => (v || '').toLowerCase().includes(needle)));

  const entryOf = (d) => ({ kind: d.kind, date: d.date, amount: d.amount, member: d.member, category: d.category, tag: d.tag, name: d.label });

  async function handleImport() {
    importError = null;
    try {
      const drafts = await draftsFor(transactions, bundleCategory);
      if (drafts.length === 0) throw new Error('No expenses or income entries selected for import.');
      const probes = drafts.map((d) => ({ category: d.category, amount: d.amount, date: d.date }));
      const found = (await entryApi.duplicates(probes)).filter((f) => drafts[f.index].kind === 'expense');
      if (found.length) {
        pendingDrafts = drafts;
        duplicateIndices = found.map((f) => f.index);
        duplicateItems = found.map((f) => ({ prospective: { ...drafts[f.index], name: drafts[f.index].label }, matches: f.matches }));
        showDuplicateModal = true;
        return;
      }
      await executeImport(drafts);
    } catch (err) {
      importError = err.message ?? String(err);
    }
  }

  async function executeImport(drafts) {
    importing = true;
    try {
      await entryApi.batch(drafts.map(entryOf));
      importSuccessCount = { expenses: drafts.filter((d) => d.kind === 'expense').length, incomes: drafts.filter((d) => d.kind === 'income').length };
      setTimeout(handleClose, 1500);
    } catch (err) {
      importError = err.message;
    } finally {
      importing = false;
    }
  }

  async function handleConfirmDuplicates() {
    showDuplicateModal = false;
    const drafts = pendingDrafts;
    pendingDrafts = null;
    duplicateItems = [];
    if (drafts) await executeImport(drafts);
  }

  async function handleSkipDuplicates() {
    showDuplicateModal = false;
    const skip = new Set(duplicateIndices);
    const remaining = (pendingDrafts ?? []).filter((_, i) => !skip.has(i));
    pendingDrafts = null;
    duplicateItems = [];
    if (remaining.length) await executeImport(remaining);
    else importError = 'All selected expenses were duplicates; nothing imported.';
  }

  function handleCancelDuplicates() {
    showDuplicateModal = false;
    pendingDrafts = null;
    duplicateItems = [];
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if isOpen}
  <div
    class="fixed inset-0 z-50 !m-0 w-full h-full min-h-screen flex items-center justify-center p-3 sm:p-4 overflow-hidden"
    role="dialog"
    aria-modal="true"
    aria-labelledby="csv-modal-title"
  >
    <!-- Backdrop button for click-outside dismissal -->
    <button
      type="button"
      class="fixed inset-0 bg-neutral-900/60 dark:bg-black/75 backdrop-blur-sm transition-opacity duration-200 cursor-default w-full h-full border-0 p-0 m-0"
      on:click={handleClose}
      aria-label="Close dialog backdrop"
      tabindex="-1"
    ></button>

    <div
      class="relative z-10 bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 rounded-2xl shadow-2xl w-full max-w-5xl max-h-[90vh] flex flex-col overflow-hidden animate-in fade-in zoom-in-95 duration-150"
    >
      <!-- Modal Header -->
      <header class="px-6 py-4 border-b border-neutral-200 dark:border-neutral-800 flex items-center justify-between bg-neutral-50/50 dark:bg-neutral-900/50">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-indigo-100 dark:bg-indigo-950/60 text-indigo-600 dark:text-indigo-400 flex items-center justify-center text-xl shadow-sm">
            🏦
          </div>
          <div>
            <h2 id="csv-modal-title" class="text-base font-bold text-neutral-900 dark:text-white">
              Bank Statement CSV Importer
            </h2>
            <p class="text-xs text-neutral-500 dark:text-neutral-400">
              Parse bank exports, auto-categorize beneficiaries, and import into your encrypted ledger
            </p>
          </div>
        </div>
        <button
          class="p-2 rounded-xl text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 hover:bg-neutral-100 dark:hover:bg-neutral-800 transition-colors"
          on:click={handleClose}
          aria-label="Close dialog"
        >
          ✕
        </button>
      </header>

      <!-- Modal Body -->
      <div class="flex-1 overflow-y-auto p-6 space-y-6">
        <!-- Stage 1: Upload & Bank Selection (Shown when no transactions loaded) -->
        {#if transactions.length === 0}
          <div class="max-w-2xl mx-auto space-y-6">
            <!-- Bank & Payer Configuration -->
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <!-- Bank Selection -->
              <div>
                <label for="bank-select" class="block text-xs font-semibold text-neutral-700 dark:text-neutral-300 mb-1.5">
                  Select Origin Bank
                </label>
                <select
                  id="bank-select"
                  bind:value={selectedBank}
                  class="w-full px-3.5 py-2.5 rounded-xl border border-neutral-300 dark:border-neutral-700 bg-white dark:bg-neutral-800 text-neutral-900 dark:text-neutral-100 text-sm font-medium focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none transition"
                >
                  <option value="ING">ING Bank (Belgium)</option>
                  <option value="KBC">KBC Bank (Belgium) — Coming Soon</option>
                </select>
                {#if selectedBank === 'KBC'}
                  <p class="text-xs text-amber-600 dark:text-amber-400 mt-1 flex items-center gap-1">
                    <span>⚠️</span> KBC parser is scheduled for a future update. Please select ING for this file.
                  </p>
                {/if}
              </div>

              <!-- Payer Selection -->
              <div>
                <label for="payer-select" class="block text-xs font-semibold text-neutral-700 dark:text-neutral-300 mb-1.5">
                  Who Paid (Account Holder)
                </label>
                <select
                  id="payer-select"
                  bind:value={selectedPayer}
                  class="w-full px-3.5 py-2.5 rounded-xl border border-neutral-300 dark:border-neutral-700 bg-white dark:bg-neutral-800 text-neutral-900 dark:text-neutral-100 text-sm font-medium focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none transition"
                >
                  {#each $activeMembers as u (u.id)}
                    <option value={u.id}>{u.name}</option>
                  {/each}
                </select>
              </div>
            </div>

            <!-- Drag & Drop Zone -->
            <div
              class="border-2 border-dashed rounded-2xl p-8 sm:p-12 text-center transition-all cursor-pointer {isDragging
                ? 'border-indigo-500 bg-indigo-50/50 dark:bg-indigo-950/20'
                : 'border-neutral-300 dark:border-neutral-700 hover:border-neutral-400 dark:hover:border-neutral-600 bg-neutral-50/50 dark:bg-neutral-800/30'}"
              on:dragover={handleDragOver}
              on:dragleave={handleDragLeave}
              on:drop={handleDrop}
              on:click={() => fileInput?.click()}
              role="button"
              tabindex="0"
              on:keydown={(e) => e.key === 'Enter' && fileInput?.click()}
            >
              <input
                id="csv-file-input"
                type="file"
                accept=".csv"
                class="hidden"
                bind:this={fileInput}
                on:change={handleFileInputChange}
              />
              <div class="w-14 h-14 rounded-2xl bg-indigo-100 dark:bg-indigo-900/40 text-indigo-600 dark:text-indigo-400 mx-auto flex items-center justify-center text-2xl mb-3 shadow-inner">
                📥
              </div>
              <p class="text-sm font-bold text-neutral-800 dark:text-neutral-200">
                Click to browse or drag and drop your bank CSV file
              </p>
              <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-1">
                Zero-knowledge: your bank statement is parsed locally in your browser and never leaves unencrypted.
              </p>
            </div>

            {#if parsing}
              <div class="flex items-center justify-center gap-3 py-4 text-sm text-neutral-600 dark:text-neutral-300">
                <div class="animate-spin h-5 w-5 border-2 border-indigo-600 border-t-transparent rounded-full" />
                Parsing statement transactions...
              </div>
            {/if}

            {#if parseError}
              <div class="p-4 rounded-xl bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-900/60 text-rose-700 dark:text-rose-300 text-sm flex items-center gap-2">
                <span>⚠️</span>
                <span>{parseError}</span>
              </div>
            {/if}
          </div>

        <!-- Stage 2: Parsed Overview & Review Table -->
        {:else}
          <div class="space-y-4">
            <!-- Summary Metric Cards & Toolbar -->
            <div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
              <div class="p-3.5 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-neutral-50 dark:bg-neutral-800/40">
                <span class="text-[11px] font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">Total Rows</span>
                <p class="text-lg font-bold text-neutral-900 dark:text-white mt-0.5">{stats?.total ?? transactions.length}</p>
              </div>
              <div class="p-3.5 rounded-xl border border-indigo-200 dark:border-indigo-900/60 bg-indigo-50/50 dark:bg-indigo-950/20">
                <span class="text-[11px] font-semibold text-indigo-600 dark:text-indigo-400 uppercase tracking-wider">Expenses to Store</span>
                <p class="text-lg font-bold text-indigo-900 dark:text-indigo-200 mt-0.5">
                  {summary?.expenses ?? 0} <span class="text-xs font-medium text-neutral-500">({$money(summary?.expenseCents ?? 0)})</span>
                </p>
              </div>
              <div class="p-3.5 rounded-xl border border-emerald-200 dark:border-emerald-900/60 bg-emerald-50/50 dark:bg-emerald-950/20">
                <span class="text-[11px] font-semibold text-emerald-600 dark:text-emerald-400 uppercase tracking-wider">Income to Store</span>
                <p class="text-lg font-bold text-emerald-900 dark:text-emerald-200 mt-0.5">
                  {summary?.incomes ?? 0} <span class="text-xs font-medium text-neutral-500">(+{$money(summary?.incomeCents ?? 0)})</span>
                </p>
              </div>
              <div class="p-3.5 rounded-xl border {hasUncategorized ? 'border-amber-300 dark:border-amber-800/80 bg-amber-50/70 dark:bg-amber-950/30' : 'border-neutral-200 dark:border-neutral-800 bg-neutral-50 dark:bg-neutral-800/40'}">
                <span class="text-[11px] font-semibold {hasUncategorized ? 'text-amber-700 dark:text-amber-400' : 'text-neutral-500 dark:text-neutral-400'} uppercase tracking-wider">
                  {hasUncategorized ? 'Missing Category' : 'Categorized'}
                </span>
                <p class="text-lg font-bold {hasUncategorized ? 'text-amber-900 dark:text-amber-200' : 'text-emerald-600 dark:text-emerald-400'} mt-0.5">
                  {hasUncategorized ? `${uncategorizedCount} items` : 'All Set ✓'}
                </p>
              </div>
            </div>

            <!-- Toast / Notification Banner -->
            {#if cascadeNotice}
              <div class="px-4 py-2.5 rounded-xl bg-emerald-50 dark:bg-emerald-950/50 border border-emerald-200 dark:border-emerald-800/60 text-emerald-800 dark:text-emerald-200 text-xs font-semibold flex items-center gap-2 animate-in fade-in duration-200">
                <span>✨</span>
                <span>{cascadeNotice}</span>
              </div>
            {/if}

            <!-- Uncategorized Warning Banner -->
            {#if hasUncategorized}
              <div class="px-4 py-2.5 rounded-xl bg-amber-50 dark:bg-amber-950/50 border border-amber-300 dark:border-amber-800/60 text-amber-900 dark:text-amber-200 text-xs font-semibold flex items-center justify-between gap-3 animate-in fade-in duration-200">
                <div class="flex items-center gap-2">
                  <span>⚠️</span>
                  <span>
                    <strong>{uncategorizedCount} included item{uncategorizedCount === 1 ? '' : 's'}</strong> missing a category. All to-import transactions must have an explicit category set.
                  </span>
                </div>
                <button
                  type="button"
                  on:click={() => (filterOnlyUncategorized = !filterOnlyUncategorized)}
                  class="px-2.5 py-1 rounded-lg bg-amber-200/80 dark:bg-amber-900/60 hover:bg-amber-300 dark:hover:bg-amber-800 text-amber-950 dark:text-amber-100 font-bold transition text-[11px] whitespace-nowrap"
                >
                  {filterOnlyUncategorized ? 'Show All' : 'Show Uncategorized Only'}
                </button>
              </div>
            {/if}

            <!-- Toolbar & Controls -->
            <div class="flex flex-wrap items-center justify-between gap-3 bg-neutral-100/60 dark:bg-neutral-800/40 p-2.5 rounded-xl">
              <div class="flex items-center gap-2">
                <input
                  type="text"
                  bind:value={searchQuery}
                  placeholder="Search merchant or category..."
                  class="px-3 py-1.5 rounded-lg border border-neutral-300 dark:border-neutral-700 bg-white dark:bg-neutral-800 text-xs text-neutral-900 dark:text-neutral-100 placeholder-neutral-400 outline-none focus:ring-1 focus:ring-indigo-500 w-52 sm:w-64"
                />
                <button
                  type="button"
                  on:click={() => toggleAllIncluded(true)}
                  class="px-2.5 py-1.5 rounded-lg text-xs font-semibold text-neutral-700 dark:text-neutral-300 bg-white dark:bg-neutral-800 border border-neutral-300 dark:border-neutral-700 hover:bg-neutral-50 dark:hover:bg-neutral-700 transition"
                >
                  Select All
                </button>
                <button
                  type="button"
                  on:click={() => toggleAllIncluded(false)}
                  class="px-2.5 py-1.5 rounded-lg text-xs font-semibold text-neutral-700 dark:text-neutral-300 bg-white dark:bg-neutral-800 border border-neutral-300 dark:border-neutral-700 hover:bg-neutral-50 dark:hover:bg-neutral-700 transition"
                >
                  Deselect All
                </button>
              </div>

              <div class="flex items-center gap-2">
                <button
                  type="button"
                  on:click={groupMicroExpenses}
                  class="px-2.5 py-1.5 rounded-lg text-xs font-semibold text-purple-700 dark:text-purple-300 bg-purple-50 dark:bg-purple-950/50 border border-purple-200 dark:border-purple-800 hover:bg-purple-100 transition flex items-center gap-1.5"
                  title="Automatically check 'Group to Other' for expenses under €5.00"
                >
                  <span>📦</span> Group Micro-Expenses (&lt; €5)
                </button>
                <button
                  type="button"
                  on:click={resetState}
                  class="px-2.5 py-1.5 rounded-lg text-xs font-semibold text-neutral-500 hover:text-neutral-800 dark:hover:text-neutral-200 transition"
                >
                  Upload Another File
                </button>
              </div>
            </div>

            <!-- Table -->
            <div class="border border-neutral-200 dark:border-neutral-800 rounded-xl overflow-hidden shadow-sm">
              <div class="max-h-[46vh] overflow-y-auto">
                <table class="w-full text-left text-xs text-neutral-700 dark:text-neutral-300">
                  <thead class="sticky top-0 z-10 bg-neutral-100 dark:bg-neutral-800 border-b border-neutral-200 dark:border-neutral-700 text-[11px] font-bold uppercase tracking-wider text-neutral-500 dark:text-neutral-400">
                    <tr>
                      <th class="p-2.5 text-center w-12">Store</th>
                      <th class="p-2.5 w-16 text-center">Type</th>
                      <th class="p-2.5 w-24">Date</th>
                      <th class="p-2.5 min-w-[170px]">Receiving Party / Beneficiary</th>
                      <th class="p-2.5 text-right w-24">Amount</th>
                      <th class="p-2.5 min-w-[160px]">Category (Auto-Cascade)</th>
                      <th class="p-2.5 min-w-[120px]">Tag (Opt)</th>
                      <th class="p-2.5 text-center w-28" title="Group into consolidated 'Other' expense entry">
                        Group Other
                      </th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-neutral-200 dark:divide-neutral-800 bg-white dark:bg-neutral-900">
                    {#each filteredTransactions as row (row.t.rowIndex)}
                      {@const tx = row.t}
                      <tr class="hover:bg-neutral-50/70 dark:hover:bg-neutral-800/40 transition-colors {tx.include ? '' : 'opacity-40 bg-neutral-50/30'}">
                        <!-- Include Checkbox -->
                        <td class="p-2.5 text-center">
                          <input
                            type="checkbox"
                            bind:checked={transactions[row.index].include}
                            class="w-4 h-4 rounded text-indigo-600 focus:ring-indigo-500 dark:focus:ring-offset-neutral-900 cursor-pointer"
                          />
                        </td>

                        <!-- Type Badge -->
                        <td class="p-2.5 text-center whitespace-nowrap">
                          {#if tx.isCredit}
                            <span class="px-1.5 py-0.5 rounded text-[10px] font-bold bg-emerald-100 text-emerald-800 dark:bg-emerald-950/60 dark:text-emerald-300">
                              Income
                            </span>
                          {:else}
                            <span class="px-1.5 py-0.5 rounded text-[10px] font-bold bg-indigo-100 text-indigo-800 dark:bg-indigo-950/60 dark:text-indigo-300">
                              Expense
                            </span>
                          {/if}
                        </td>

                        <!-- Date -->
                        <td class="p-2.5 whitespace-nowrap font-mono text-[11px] text-neutral-600 dark:text-neutral-400">
                          {tx.displayDate || tx.date}
                        </td>

                        <!-- Beneficiary -->
                        <td class="p-2.5">
                          <input
                            type="text"
                            bind:value={transactions[row.index].beneficiary}
                            class="w-full px-2 py-1 rounded-md border border-transparent hover:border-neutral-300 dark:hover:border-neutral-700 focus:border-indigo-500 bg-transparent text-xs font-semibold text-neutral-900 dark:text-neutral-100 outline-none transition"
                          />
                        </td>

                        <!-- Amount -->
                        <td class="p-2.5 text-right whitespace-nowrap font-mono font-semibold">
                          {#if tx.isCredit}
                            <span class="text-emerald-600 dark:text-emerald-400">
                              +{$money(tx.amountCents)}
                            </span>
                          {:else}
                            <span class="text-neutral-900 dark:text-white">
                              {$money(tx.amountCents)}
                            </span>
                          {/if}
                        </td>

                        <!-- Category with Auto-Cascade -->
                        <td class="p-2.5">
                          {#if tx.isCredit}
                            <select
                              value={tx.category}
                              on:change={(e) => onCategoryChange(row.index, e.target.value ? Number(e.target.value) : null)}
                              class="w-full px-2 py-1 rounded-lg border text-xs font-medium focus:ring-1 outline-none transition
                                {!tx.category && tx.include
                                  ? 'border-amber-400 dark:border-amber-500 bg-amber-50/70 dark:bg-amber-950/50 text-amber-900 dark:text-amber-100 focus:ring-amber-500'
                                  : 'border-emerald-300 dark:border-emerald-700 bg-emerald-50/50 dark:bg-emerald-950/40 text-emerald-900 dark:text-emerald-100 focus:ring-emerald-500'}"
                            >
                              <option value={null}>-- Choose Income Category --</option>
                              {#each $incomeCategories as c (c.id)}
                                <option value={c.id}>{c.name}</option>
                              {/each}
                            </select>
                          {:else}
                            <select
                              value={tx.category}
                              on:change={(e) => onCategoryChange(row.index, e.target.value ? Number(e.target.value) : null)}
                              class="w-full px-2 py-1 rounded-lg border text-xs font-medium focus:ring-1 outline-none transition
                                {!tx.category && tx.include && !tx.groupToOther
                                  ? 'border-amber-400 dark:border-amber-500 bg-amber-50/70 dark:bg-amber-950/50 text-amber-900 dark:text-amber-100 focus:ring-amber-500'
                                  : 'border-neutral-300 dark:border-neutral-700 bg-neutral-50 dark:bg-neutral-800 text-neutral-900 dark:text-neutral-100 focus:ring-indigo-500'}"
                            >
                              <option value={null}>-- Choose Category --</option>
                              {#each $expenseCategories as c (c.id)}
                                <option value={c.id}>{c.name}</option>
                              {/each}
                            </select>
                          {/if}
                        </td>

                        <!-- Tag -->
                        <td class="p-2.5">
                          <select
                            bind:value={transactions[row.index].tag}
                            class="w-full px-2 py-1 rounded-lg border border-neutral-300 dark:border-neutral-700 bg-neutral-50 dark:bg-neutral-800 text-xs text-neutral-900 dark:text-neutral-100 font-medium focus:ring-1 focus:ring-indigo-500 outline-none transition"
                          >
                            <option value={null}>None</option>
                            {#each $tags as tag (tag.id)}
                              <option value={tag.id}>{tag.name}</option>
                            {/each}
                          </select>
                        </td>

                        <!-- Group to Other Checkbox (Expenses only) -->
                        <td class="p-2.5 text-center">
                          {#if tx.isCredit}
                            <span class="text-neutral-400 text-xs" title="Income entries are not grouped into Other">—</span>
                          {:else}
                            <input
                              type="checkbox"
                              bind:checked={transactions[row.index].groupToOther}
                              disabled={!tx.include}
                              class="w-4 h-4 rounded text-purple-600 focus:ring-purple-500 dark:focus:ring-offset-neutral-900 cursor-pointer disabled:opacity-30"
                            />
                          {/if}
                        </td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            </div>

            <!-- Grouped To Other Note -->
            {#if summary?.grouped > 0}
              <div class="px-4 py-2.5 rounded-xl bg-purple-50 dark:bg-purple-950/40 border border-purple-200 dark:border-purple-800/60 text-purple-900 dark:text-purple-200 text-xs flex items-center justify-between">
                <div class="flex items-center gap-2">
                  <span>📦</span>
                  <span>
                    <strong>{summary.grouped} expenses</strong> will be bundled into 1 consolidated <em>"Other expenses"</em> ledger entry ({$money(summary.groupedCents)}).
                  </span>
                </div>
                <span class="font-bold text-purple-700 dark:text-purple-300 text-[11px] uppercase tracking-wider">Semi-Hidden Grouping</span>
              </div>
            {/if}

            {#if importError}
              <div class="p-3.5 rounded-xl bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-900/60 text-rose-700 dark:text-rose-300 text-xs flex items-center gap-2">
                <span>⚠️</span>
                <span>{importError}</span>
              </div>
            {/if}

            {#if importSuccessCount !== null}
              <div class="p-3.5 rounded-xl bg-emerald-50 dark:bg-emerald-950/50 border border-emerald-200 dark:border-emerald-800 text-emerald-800 dark:text-emerald-200 text-xs font-bold flex items-center gap-2">
                <span>✅</span>
                <span>
                  Successfully imported
                  {#if importSuccessCount.expenses > 0 && importSuccessCount.incomes > 0}
                    {importSuccessCount.expenses} expenses and {importSuccessCount.incomes} income entries!
                  {:else if importSuccessCount.expenses > 0}
                    {importSuccessCount.expenses} expenses!
                  {:else}
                    {importSuccessCount.incomes} income entries!
                  {/if}
                </span>
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Modal Footer -->
      <footer class="px-6 py-4 border-t border-neutral-200 dark:border-neutral-800 flex items-center justify-between bg-neutral-50/50 dark:bg-neutral-900/50">
        <button
          type="button"
          on:click={handleClose}
          disabled={importing}
          class="px-4 py-2 rounded-xl text-xs font-semibold text-neutral-700 dark:text-neutral-300 hover:bg-neutral-200 dark:hover:bg-neutral-800 transition"
        >
          Cancel
        </button>

        {#if transactions.length > 0}
          <button
            type="button"
            on:click={handleImport}
            disabled={importing || !(summary?.expenses || summary?.incomes) || hasUncategorized}
            class="px-5 py-2.5 rounded-xl text-xs font-bold text-white bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 disabled:pointer-events-none transition shadow-lg shadow-indigo-600/20 flex items-center gap-2"
          >
            {#if importing}
              <div class="animate-spin h-3.5 w-3.5 border-2 border-white border-t-transparent rounded-full" />
              <span>Encrypting & Importing...</span>
            {:else if hasUncategorized}
              <span>Assign Categories to Import ({uncategorizedCount} missing)</span>
            {:else}
              <span>
                Import
                {#if summary?.expenses > 0 && summary?.incomes > 0}
                  {summary.expenses} Expenses & {summary.incomes} Incomes
                {:else if summary?.expenses > 0}
                  {summary.expenses} Expenses ({$money(summary.expenseCents)})
                {:else}
                  {summary?.incomes ?? 0} Incomes ({$money(summary?.incomeCents ?? 0)})
                {/if}
              </span>
            {/if}
          </button>
        {/if}
      </footer>
    </div>
  </div>

  <DuplicatePromptModal
    isOpen={showDuplicateModal}
    items={duplicateItems}
    isBatch={true}
    on:confirm={handleConfirmDuplicates}
    on:skipDuplicates={handleSkipDuplicates}
    on:cancel={handleCancelDuplicates}
  />
{/if}
