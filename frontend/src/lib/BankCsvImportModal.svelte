<script>
  /**
   * BankCsvImportModal.svelte
   *
   * Bank statement CSV Importer dialog for Jizifin:
   * - Bank Selection (ING Bank Belgium active, KBC Bank upcoming)
   * - Household Payer Selection
   * - Client-side zero-knowledge statement parsing
   * - Overview table with manual actions:
   *   * Beneficiary-based category cascading
   *   * Optional tag assignment
   *   * Prevent storage checkbox
   *   * Group to Other (semi-hidden expense consolidation)
   */

  import { createEventDispatcher, onMount } from 'svelte';
  import { users, splits, tags, currencySymbol, selectedMonth, incomeCategories, expenses } from './stores.js';
  import { parseBankCsv, cascadeCategory, prepareTransactionsForImport } from './csvParser.js';
  import { createExpensesBatch, createIncome } from './api.js';
  import { findMatchingExpenses } from './duplicateDetector.js';
  import DuplicatePromptModal from './DuplicatePromptModal.svelte';

  const dispatch = createEventDispatcher();

  export let isOpen = false;

  // Active household members
  $: activeUsers = $users.filter((u) => u.is_active);
  $: activeTags = $tags.filter((t) => t.is_active !== false && t.is_active !== 0);

  // Form State
  let selectedBank = 'ING';
  let selectedPayer = '';

  // Watch activeUsers to set default payer
  $: if (activeUsers.length > 0 && !selectedPayer) {
    selectedPayer = activeUsers[0].name;
  }

  // File & Parsing State
  let fileInput;
  let isDragging = false;
  let fileName = '';
  let parsing = false;
  let parseError = null;

  // Review Stage State
  let transactions = [];
  let stats = null;
  let searchQuery = '';
  let filterOnlyUncategorized = false;
  let cascadeNotice = '';
  let cascadeNoticeTimer = null;

  // Import Execution State
  let importing = false;
  let importError = null;
  let importSuccessCount = null;

  // Duplicate Detection State
  let showDuplicateModal = false;
  let duplicateItems = [];
  let pendingImportPayload = null;

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
    pendingImportPayload = null;
  }

  function handleClose() {
    resetState();
    dispatch('close');
  }

  function handleKeydown(e) {
    if (e.key === 'Escape' && !importing) {
      handleClose();
    }
  }

  // Handle Drag & Drop
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
      if (selectedBank === 'KBC') {
        throw new Error('KBC Bank export format is not yet supported. The parser will be added in a future update.');
      }

      const text = await file.text();
      const result = parseBankCsv(text, selectedBank, selectedPayer);
      // Transactions start with category = '' requiring explicit selection
      transactions = result.transactions;
      stats = result.stats;
    } catch (err) {
      parseError = err.message || 'Failed to parse CSV file.';
      transactions = [];
      stats = null;
    } finally {
      parsing = false;
    }
  }

  // Manual Actions: Category Cascading
  function onCategoryChange(tx, newCategory) {
    tx.category = newCategory;
    const beneficiary = tx.beneficiary;
    if (!beneficiary) return;

    // Cascade within the same type (income or expense)
    const { updatedTransactions, count } = cascadeCategory(transactions, beneficiary, newCategory, tx.isCredit);
    transactions = updatedTransactions;

    if (count > 1) {
      if (cascadeNoticeTimer) clearTimeout(cascadeNoticeTimer);
      const typeLabel = tx.isCredit ? 'income transactions' : 'expenses';
      cascadeNotice = `Updated ${count} ${typeLabel} for "${beneficiary}" to "${newCategory}"`;
      cascadeNoticeTimer = setTimeout(() => {
        cascadeNotice = '';
      }, 3500);
    }
  }

  // Quick Action: Select / Deselect All
  function toggleAllIncluded(value) {
    transactions = transactions.map((t) => ({ ...t, include: value }));
  }

  // Quick Action: Auto-group micro expenses (< €5.00)
  function groupMicroExpenses() {
    let count = 0;
    transactions = transactions.map((t) => {
      if (t.include && !t.isCredit && t.amountCents < 500) {
        count++;
        return { ...t, groupToOther: true };
      }
      return t;
    });
    if (count > 0) {
      if (cascadeNoticeTimer) clearTimeout(cascadeNoticeTimer);
      cascadeNotice = `Grouped ${count} micro-expenses (< €5) to "Other"`;
      cascadeNoticeTimer = setTimeout(() => {
        cascadeNotice = '';
      }, 3500);
    }
  }

  // Computed Review Stats
  $: includedTransactions = transactions.filter((t) => t.include && t.amountCents > 0);
  $: includedExpenses = includedTransactions.filter((t) => !t.isCredit);
  $: includedIncomes = includedTransactions.filter((t) => t.isCredit);
  $: groupedToOtherTransactions = includedExpenses.filter((t) => t.groupToOther);

  $: totalIncludedExpenseCents = includedExpenses.reduce((sum, t) => sum + t.amountCents, 0);
  $: totalIncludedIncomeCents = includedIncomes.reduce((sum, t) => sum + t.amountCents, 0);
  $: totalGroupedOtherCents = groupedToOtherTransactions.reduce((sum, t) => sum + t.amountCents, 0);

  // Uncategorized check: all included items (expenses not grouped to other, and all income) must have a category
  $: uncategorizedTransactions = includedTransactions.filter(
    (t) => (!t.category || !t.category.trim()) && !t.groupToOther
  );
  $: uncategorizedCount = uncategorizedTransactions.length;
  $: hasUncategorized = uncategorizedCount > 0;

  // Map of tx.id -> matching existing expenses for duplicate indication
  $: duplicateMap = new Map();
  $: {
    const map = new Map();
    for (const tx of transactions) {
      if (tx.include && !tx.isCredit && tx.category && !tx.groupToOther) {
        const matches = findMatchingExpenses({ category: tx.category, cost_cents: tx.amountCents }, $expenses);
        if (matches.length > 0) {
          map.set(tx.id, matches);
        }
      }
    }
    duplicateMap = map;
  }

  // Filtered view in table
  $: filteredTransactions = transactions.filter((t) => {
    if (filterOnlyUncategorized) {
      const isUncat = t.include && t.amountCents > 0 && (!t.category || !t.category.trim()) && !t.groupToOther;
      if (!isUncat) return false;
    }
    if (!searchQuery.trim()) return true;
    const q = searchQuery.toLowerCase();
    return (
      (t.beneficiary && t.beneficiary.toLowerCase().includes(q)) ||
      (t.rawDescription && t.rawDescription.toLowerCase().includes(q)) ||
      (t.category && t.category.toLowerCase().includes(q)) ||
      t.displayDate.includes(q)
    );
  });

  // Execute Import
  async function handleImport() {
    importError = null;

    try {
      if (hasUncategorized) {
        throw new Error(
          `All to-import transactions must have a category set before importing (${uncategorizedCount} items missing category).`
        );
      }

      const { expenses: preparedExpenses, incomes: preparedIncomes } = prepareTransactionsForImport(
        transactions,
        selectedPayer
      );

      if (preparedExpenses.length === 0 && preparedIncomes.length === 0) {
        throw new Error('No expenses or income entries selected for import.');
      }

      // Duplicate detection: check if any to-import expense matches existing expenses
      const duplicateWarnings = [];
      for (const exp of preparedExpenses) {
        if (exp.name.startsWith('Other expenses (')) continue;
        const matches = findMatchingExpenses(exp, $expenses);
        if (matches.length > 0) {
          duplicateWarnings.push({ prospective: exp, matches });
        }
      }

      if (duplicateWarnings.length > 0) {
        pendingImportPayload = { expenses: preparedExpenses, incomes: preparedIncomes };
        duplicateItems = duplicateWarnings;
        showDuplicateModal = true;
        return;
      }

      await executeImport(preparedExpenses, preparedIncomes);
    } catch (err) {
      importError = err.message || 'Failed to import transactions.';
    }
  }

  async function executeImport(expensesToSave, incomesToSave) {
    importing = true;
    try {
      if (expensesToSave.length > 0) {
        await createExpensesBatch(expensesToSave, $selectedMonth);
      }
      if (incomesToSave.length > 0) {
        await createIncome(incomesToSave, $selectedMonth);
      }

      importSuccessCount = { expenses: expensesToSave.length, incomes: incomesToSave.length };
      setTimeout(() => {
        handleClose();
      }, 1500);
    } catch (err) {
      importError = err.message || 'Failed to import transactions.';
    } finally {
      importing = false;
    }
  }

  async function handleConfirmDuplicates() {
    showDuplicateModal = false;
    if (pendingImportPayload) {
      const { expenses: exps, incomes: incs } = pendingImportPayload;
      pendingImportPayload = null;
      duplicateItems = [];
      await executeImport(exps, incs);
    }
  }

  async function handleSkipDuplicates() {
    showDuplicateModal = false;
    if (pendingImportPayload) {
      const dupKeys = new Set(
        duplicateItems.map((d) => `${d.prospective.category}__${d.prospective.cost_cents}__${d.prospective.expense_date}`)
      );
      const remainingExpenses = pendingImportPayload.expenses.filter(
        (e) => !dupKeys.has(`${e.category}__${e.cost_cents}__${e.expense_date}`)
      );
      const incs = pendingImportPayload.incomes;

      // Also uncheck skipped duplicate transactions in the table
      transactions = transactions.map((t) => {
        if (t.include && !t.isCredit && dupKeys.has(`${t.category}__${t.amountCents}__${t.date}`)) {
          return { ...t, include: false };
        }
        return t;
      });

      pendingImportPayload = null;
      duplicateItems = [];

      if (remainingExpenses.length === 0 && incs.length === 0) {
        cascadeNotice = 'All duplicate items were skipped. No remaining items to import.';
        return;
      }

      await executeImport(remainingExpenses, incs);
    }
  }

  function handleCancelDuplicates() {
    showDuplicateModal = false;
    pendingImportPayload = null;
    duplicateItems = [];
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if isOpen}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-4 overflow-hidden"
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
                  {#each activeUsers as u}
                    <option value={u.name}>{u.name}</option>
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
                  {includedExpenses.length} <span class="text-xs font-medium text-neutral-500">({$currencySymbol} {(totalIncludedExpenseCents / 100).toFixed(2)})</span>
                </p>
              </div>
              <div class="p-3.5 rounded-xl border border-emerald-200 dark:border-emerald-900/60 bg-emerald-50/50 dark:bg-emerald-950/20">
                <span class="text-[11px] font-semibold text-emerald-600 dark:text-emerald-400 uppercase tracking-wider">Income to Store</span>
                <p class="text-lg font-bold text-emerald-900 dark:text-emerald-200 mt-0.5">
                  {includedIncomes.length} <span class="text-xs font-medium text-neutral-500">(+{$currencySymbol} {(totalIncludedIncomeCents / 100).toFixed(2)})</span>
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
                    {#each filteredTransactions as tx (tx.id)}
                      <tr class="hover:bg-neutral-50/70 dark:hover:bg-neutral-800/40 transition-colors {tx.include ? '' : 'opacity-40 bg-neutral-50/30'}">
                        <!-- Include Checkbox -->
                        <td class="p-2.5 text-center">
                          <input
                            type="checkbox"
                            bind:checked={tx.include}
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
                            bind:value={tx.beneficiary}
                            class="w-full px-2 py-1 rounded-md border border-transparent hover:border-neutral-300 dark:hover:border-neutral-700 focus:border-indigo-500 bg-transparent text-xs font-semibold text-neutral-900 dark:text-neutral-100 outline-none transition"
                          />
                          {#if duplicateMap.has(tx.id)}
                            <div class="mt-0.5">
                              <span
                                class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-bold bg-amber-100 text-amber-900 dark:bg-amber-950/80 dark:text-amber-300 cursor-help"
                                title="Matches existing: {duplicateMap.get(tx.id).map((m) => `${m.name} (${m.expense_date})`).join(', ')}"
                              >
                                <span>⚠️</span> Duplicate match
                              </span>
                            </div>
                          {/if}
                        </td>

                        <!-- Amount -->
                        <td class="p-2.5 text-right whitespace-nowrap font-mono font-semibold">
                          {#if tx.isCredit}
                            <span class="text-emerald-600 dark:text-emerald-400">
                              +{$currencySymbol}{(tx.amountCents / 100).toFixed(2)}
                            </span>
                          {:else}
                            <span class="text-neutral-900 dark:text-white">
                              {$currencySymbol}{(tx.amountCents / 100).toFixed(2)}
                            </span>
                          {/if}
                        </td>

                        <!-- Category with Auto-Cascade -->
                        <td class="p-2.5">
                          {#if tx.isCredit}
                            <select
                              value={tx.category}
                              on:change={(e) => onCategoryChange(tx, e.target.value)}
                              class="w-full px-2 py-1 rounded-lg border text-xs font-medium focus:ring-1 outline-none transition
                                {!tx.category && tx.include
                                  ? 'border-amber-400 dark:border-amber-500 bg-amber-50/70 dark:bg-amber-950/50 text-amber-900 dark:text-amber-100 focus:ring-amber-500'
                                  : 'border-emerald-300 dark:border-emerald-700 bg-emerald-50/50 dark:bg-emerald-950/40 text-emerald-900 dark:text-emerald-100 focus:ring-emerald-500'}"
                            >
                              <option value="">-- Choose Income Category --</option>
                              {#each $incomeCategories as c}
                                <option value={c.category}>{c.category}</option>
                              {/each}
                            </select>
                          {:else}
                            <select
                              value={tx.category}
                              on:change={(e) => onCategoryChange(tx, e.target.value)}
                              class="w-full px-2 py-1 rounded-lg border text-xs font-medium focus:ring-1 outline-none transition
                                {!tx.category && tx.include && !tx.groupToOther
                                  ? 'border-amber-400 dark:border-amber-500 bg-amber-50/70 dark:bg-amber-950/50 text-amber-900 dark:text-amber-100 focus:ring-amber-500'
                                  : 'border-neutral-300 dark:border-neutral-700 bg-neutral-50 dark:bg-neutral-800 text-neutral-900 dark:text-neutral-100 focus:ring-indigo-500'}"
                            >
                              <option value="">-- Choose Category --</option>
                              {#each $splits as split}
                                <option value={split.category}>{split.category}</option>
                              {/each}
                            </select>
                          {/if}
                        </td>

                        <!-- Tag -->
                        <td class="p-2.5">
                          <select
                            bind:value={tx.tagId}
                            class="w-full px-2 py-1 rounded-lg border border-neutral-300 dark:border-neutral-700 bg-neutral-50 dark:bg-neutral-800 text-xs text-neutral-900 dark:text-neutral-100 font-medium focus:ring-1 focus:ring-indigo-500 outline-none transition"
                          >
                            <option value={null}>None</option>
                            {#each activeTags as tag}
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
                              bind:checked={tx.groupToOther}
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
            {#if groupedToOtherTransactions.length > 0}
              <div class="px-4 py-2.5 rounded-xl bg-purple-50 dark:bg-purple-950/40 border border-purple-200 dark:border-purple-800/60 text-purple-900 dark:text-purple-200 text-xs flex items-center justify-between">
                <div class="flex items-center gap-2">
                  <span>📦</span>
                  <span>
                    <strong>{groupedToOtherTransactions.length} expenses</strong> will be bundled into 1 consolidated <em>"Other expenses"</em> ledger entry ({$currencySymbol}{(totalGroupedOtherCents / 100).toFixed(2)}).
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
            disabled={importing || (includedExpenses.length === 0 && includedIncomes.length === 0) || hasUncategorized}
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
                {#if includedExpenses.length > 0 && includedIncomes.length > 0}
                  {includedExpenses.length} Expenses & {includedIncomes.length} Incomes
                {:else if includedExpenses.length > 0}
                  {includedExpenses.length} Expenses ({$currencySymbol}{(totalIncludedExpenseCents / 100).toFixed(2)})
                {:else}
                  {includedIncomes.length} Incomes ({$currencySymbol}{(totalIncludedIncomeCents / 100).toFixed(2)})
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
