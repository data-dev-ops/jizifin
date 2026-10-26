<script>
  /**
   * ExpenseList.svelte — one month of expenses, paged by the server (keyset
   * cursor) and filtered by category and tag server-side. Search is a plaintext
   * substring match over the loaded rows; totals come from `/views/month`.
   */

  import { entryApi, views } from './api.js';
  import { entryInput } from './entryInput.js';
  import EntryEditModal from './EntryEditModal.svelte';
  import { money, fmtDate, monthBounds } from './format.js';
  import {
    selectedMonth, ledgerVersion, seals, expenseCategories, tags, tagById, projectById,
    accountById, categoryById, memberById, memberName, categoryName, privacyShield, expenseRowDensity,
  } from './stores.js';

  let searchQuery = '';
  let selectedCategoryFilter = 'ALL';
  let selectedTagFilter = 'ALL';

  let rows = [];
  let nextCursor = null;
  let monthTotal = null;
  let loadError = null;
  let loadingMore = false;

  $: filter = {
    kind: 'expense',
    ...monthBounds($selectedMonth),
    category: selectedCategoryFilter === 'ALL' ? null : selectedCategoryFilter,
    tag: selectedTagFilter === 'ALL' ? null : selectedTagFilter,
  };
  $: reload(filter, $ledgerVersion);

  async function reload(f) {
    loadError = null;
    try {
      const [page, month] = await Promise.all([entryApi.page(f), views.month($selectedMonth)]);
      rows = page.entries;
      nextCursor = page.next_cursor;
      monthTotal = f.category ? month.by_category.find((b) => b.key === Number(f.category)) ?? { total: 0, count: 0 } : month;
    } catch (err) {
      loadError = err.message;
    }
  }

  async function loadMore() {
    loadingMore = true;
    try {
      const page = await entryApi.page({ ...filter, cursor: nextCursor });
      rows = [...rows, ...page.entries];
      nextCursor = page.next_cursor;
    } catch (err) {
      loadError = err.message;
    } finally {
      loadingMore = false;
    }
  }

  $: needle = searchQuery.trim().toLowerCase();
  $: filtered = needle ? rows.filter(matches) : rows;

  function matches(e) {
    const labels = [e.name, $categoryName(e.category), $memberName(e.member), $tagById.get(e.tag)?.name, $projectById.get(e.project)?.name];
    return labels.some((l) => (l || '').toLowerCase().includes(needle));
  }

  $: sealedMonths = new Set($seals.map((s) => s.month));
  const isLocked = (date) => sealedMonths.has(date.slice(0, 7));

  function isCoupled(e) {
    return $accountById.get(e.account)?.categories.some((c) => c.category === e.category) ?? true;
  }

  function sharesLabel(e) {
    return e.overrides.map((o) => `${$memberName(o.member)} ${o.pct}%`).join(' / ');
  }

  // ── Quick tag ──────────────────────────────────────────────────────────────
  let quickTagExpenseId = null;
  let tagUpdatingId = null;

  function toggleQuickTag(id, e) {
    e?.stopPropagation();
    quickTagExpenseId = quickTagExpenseId === id ? null : id;
  }

  function selectableTags(e) {
    return $tags.filter((t) => t.id === e.tag || ((!t.valid_from || t.valid_from <= e.date) && (!t.valid_to || t.valid_to >= e.date)));
  }

  async function setExpenseTag(expense, tag) {
    tagUpdatingId = expense.id;
    try {
      await entryApi.update(expense.id, entryInput(expense, { tag }));
      quickTagExpenseId = null;
    } catch (err) {
      loadError = err.message;
    } finally {
      tagUpdatingId = null;
    }
  }

  // ── Delete ─────────────────────────────────────────────────────────────────
  let confirmDeleteId = null;
  let deletingId = null;
  let deleteError = null;

  async function confirmDelete(id) {
    deletingId = id;
    deleteError = null;
    try {
      await entryApi.remove(id);
      confirmDeleteId = null;
    } catch (err) {
      deleteError = err.message;
    } finally {
      deletingId = null;
    }
  }

  let editing = null;
</script>

<svelte:window on:click={() => (quickTagExpenseId = null)} />

<!-- ── Filter & Search Toolbar ────────────────────────────────────────────── -->
<div class="mb-4 space-y-3">
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
    <!-- Search Bar -->
    <div class="relative flex-1 min-w-[180px]">
      <span class="absolute left-3 top-1/2 -translate-y-1/2 text-neutral-400 dark:text-neutral-500">
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
          <path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
      </span>
      <input
        id="expense-search-input"
        type="text"
        placeholder="Search loaded rows by name, payer, tag…"
        bind:value={searchQuery}
        class="input-field pl-9 pr-8 text-xs py-1.5"
      />
      {#if searchQuery}
        <button
          on:click={() => (searchQuery = '')}
          class="absolute right-2.5 top-1/2 -translate-y-1/2 text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-300 text-xs font-bold"
        >✕</button>
      {/if}
    </div>

    <!-- Category Selector Filter -->
    <select
      id="expense-category-filter"
      bind:value={selectedCategoryFilter}
      class="select-field text-xs py-1.5 w-auto"
    >
      <option value="ALL">All Categories</option>
      {#each $expenseCategories as c (c.id)}
        <option value={c.id}>{c.name}</option>
      {/each}
    </select>

    <!-- Tag Selector Filter -->
    {#if $tags.length > 0}
      <select
        id="expense-tag-filter"
        bind:value={selectedTagFilter}
        class="select-field text-xs py-1.5 w-auto"
      >
        <option value="ALL">All Tags</option>
        {#each $tags as t}
          <option value={t.id}>● {t.name}</option>
        {/each}
      </select>
    {/if}
  </div>

  <!-- Summary Stats Bar -->
  <div class="flex items-center justify-between text-xs text-neutral-500 dark:text-neutral-400 px-1">
    <span>
      Showing <strong class="text-neutral-900 dark:text-neutral-200">{filtered.length}</strong> of {monthTotal?.count ?? rows.length} entries
    </span>
    <span class="font-semibold text-neutral-800 dark:text-neutral-200">
      Total: <span class="text-indigo-600 dark:text-indigo-400 font-bold">{$money(monthTotal?.total ?? 0)}</span>
    </span>
  </div>
</div>

{#if loadError}
  <p class="text-xs text-rose-600 dark:text-red-400 mb-3">{loadError}</p>
{/if}

{#if filtered.length === 0}
  <div class="flex flex-col items-center justify-center py-16 text-center gap-3 empty-state-box">
    <div class="w-14 h-14 rounded-2xl bg-neutral-100 dark:bg-neutral-800/80 flex items-center justify-center mb-1">
      <svg class="w-7 h-7 text-neutral-400 dark:text-neutral-600" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
        <path stroke-linecap="round" stroke-linejoin="round"
          d="M9 14l2 2 4-4M7.5 3.75A1.5 1.5 0 006 5.25v13.5A1.5 1.5 0 007.5 20.25h9A1.5 1.5 0 0018 18.75V5.25A1.5 1.5 0 0016.5 3.75H7.5z" />
      </svg>
    </div>
    <p class="text-neutral-700 dark:text-neutral-400 text-sm font-medium">
      {rows.length === 0 ? 'No expenses for this month.' : 'No matching expenses found.'}
    </p>
    <p class="text-neutral-500 text-xs max-w-xs">
      {#if rows.length === 0}
        Expenses you log will appear here. Use the form on the left to add your first entry.
      {:else}
        Try clearing your search or category filter to see all {$selectedMonth} expenses.
      {/if}
    </p>
  </div>

{:else}
  <div class="overflow-x-auto -mx-1">
    <table class="w-full text-sm border-collapse" id="expense-table">
      <thead>
        <tr class="border-b border-neutral-200 dark:border-neutral-800">
          <th class="text-left text-xs font-medium text-neutral-500 pb-3 pr-4 pl-1">Date</th>
          <th class="text-left text-xs font-medium text-neutral-500 pb-3 pr-4">Description</th>
          <th class="text-left text-xs font-medium text-neutral-500 pb-3 pr-4">Category</th>
          <th class="text-left text-xs font-medium text-neutral-500 pb-3 pr-4">Paid by</th>
          <th class="text-right text-xs font-medium text-neutral-500 pb-3 pr-4">Amount</th>
          <th class="text-right text-xs font-medium text-neutral-500 pb-3 pr-1">Actions</th>
        </tr>
      </thead>
      <tbody>
        {#each filtered as expense (expense.id)}
          <tr class="border-b border-neutral-200/60 dark:border-neutral-800/60 hover:bg-neutral-100/60 dark:hover:bg-neutral-800/40 transition-colors group">
            <!-- Date -->
            <td class="py-3 pr-4 pl-1 text-neutral-500 tabular-nums whitespace-nowrap">
              {fmtDate(expense.date)}
            </td>

            <!-- Description & Badges -->
            <td class="py-3 pr-4 text-neutral-800 dark:text-neutral-200 max-w-[200px]" title={expense.name}>
              <span class="block truncate font-medium text-neutral-900 dark:text-neutral-100">{expense.name}</span>
              
              <div class="flex items-center gap-1.5 flex-wrap mt-1">
                {#if $projectById.get(expense.project)}
                  <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px]
                               bg-indigo-50 dark:bg-indigo-950/60 text-indigo-700 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-900/60 font-medium">
                    ▰ {$projectById.get(expense.project).name}
                  </span>
                {/if}

                <!-- Tag Badge & Quick Tagging Trigger -->
                <!-- svelte-ignore a11y-click-events-have-key-events -->
                <!-- svelte-ignore a11y-no-static-element-interactions -->
                <div class="relative inline-block" on:click|stopPropagation>
                  {#if $tagById.get(expense.tag)}
                    {@const tag = $tagById.get(expense.tag)}
                    <button
                      type="button"
                      id="tag-badge-{expense.id}"
                      on:click={(e) => toggleQuickTag(expense.id, e)}
                      class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] border font-medium hover:brightness-110 transition-all"
                      style="background-color: {tag.color}18; color: {tag.color}; border-color: {tag.color}40;"
                      title="Click to edit or remove tag"
                    >
                      <span>● {tag.name}</span>
                    </button>
                  {:else}
                    <button
                      type="button"
                      id="add-tag-btn-{expense.id}"
                      on:click={(e) => toggleQuickTag(expense.id, e)}
                      class="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded text-[10px] text-neutral-500 hover:text-neutral-800 dark:hover:text-neutral-300 hover:bg-neutral-200/60 dark:hover:bg-neutral-800 border border-transparent hover:border-neutral-300 dark:hover:border-neutral-700 transition-all font-medium"
                      title="Add tag to this expense"
                    >
                      + Tag
                    </button>
                  {/if}

                  <!-- Quick Tag Dropdown Popover -->
                  {#if quickTagExpenseId === expense.id}
                    <div
                      class="absolute left-0 top-full mt-1.5 z-40 w-48 bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-700 rounded-xl shadow-2xl p-2 space-y-1 text-left animate-fadeIn"
                    >
                      <p class="text-[10px] font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider px-2 py-1">Assign Tag</p>
                      
                      {#if selectableTags(expense).length === 0}
                        <p class="text-xs text-neutral-500 px-2 py-1.5">No active tags available.</p>
                      {:else}
                        <div class="max-h-40 overflow-y-auto space-y-0.5">
                          {#each selectableTags(expense) as t (t.id)}
                            <button
                              type="button"
                              on:click={() => setExpenseTag(expense, t.id)}
                              disabled={tagUpdatingId === expense.id}
                              class="w-full flex items-center gap-2 px-2 py-1.5 rounded-lg text-xs hover:bg-neutral-100 dark:hover:bg-neutral-800 transition-colors text-left {expense.tag === t.id ? 'bg-neutral-100 dark:bg-neutral-800/80 font-semibold' : ''}"
                            >
                              <span class="w-2.5 h-2.5 rounded-full flex-none" style="background-color: {t.color}"></span>
                              <span class="truncate flex-1 text-neutral-800 dark:text-neutral-200">
                                {t.name}
                              </span>
                              {#if expense.tag === t.id}
                                <span class="text-indigo-600 dark:text-indigo-400 text-xs">✓</span>
                              {/if}
                            </button>
                          {/each}
                        </div>
                      {/if}

                      {#if expense.tag}
                        <div class="border-t border-neutral-200 dark:border-neutral-800 pt-1 mt-1">
                          <button
                            type="button"
                            on:click={() => setExpenseTag(expense, null)}
                            disabled={tagUpdatingId === expense.id}
                            class="w-full text-left px-2 py-1 rounded-lg text-[11px] text-rose-600 dark:text-red-400 hover:bg-rose-50 dark:hover:bg-red-950/40 transition-colors"
                          >
                            ✕ Remove Tag
                          </button>
                        </div>
                      {/if}
                    </div>
                  {/if}
                </div>

                {#if $expenseRowDensity !== 'minimal'}
                  {#if expense.account != null}
                    <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px]
                                 bg-indigo-50 dark:bg-indigo-950/80 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-700/60 font-semibold"
                          title="Paid by Joint Account">
                      🏦 Joint
                    </span>
                    {#if !isCoupled(expense)}
                      <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px]
                                   bg-amber-50 dark:bg-amber-950/80 text-amber-800 dark:text-amber-300 border border-amber-200 dark:border-amber-700/60 font-semibold"
                            title="Category is not in the list of coupled joint account categories">
                        ⚠️ Not Coupled
                      </span>
                    {/if}
                  {/if}

                  {#if expense.overrides && expense.overrides.length > 0}
                    <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px]
                                 bg-amber-50 dark:bg-amber-950/60 text-amber-700 dark:text-amber-400 border border-amber-200 dark:border-amber-800/60"
                          title="Custom split: {sharesLabel(expense)}">
                      ✶ custom split
                    </span>
                  {/if}
                {/if}
              </div>
            </td>

            <!-- Category -->
            <td class="py-3 pr-4">
              <span class="badge-neutral">
                {$categoryName(expense.category)}
              </span>
            </td>

            <!-- Paid by -->
            <td class="py-3 pr-4">
              <span class="text-xs font-semibold tabular-nums" style="color: {$memberById.get(expense.member)?.color ?? '#6366f1'}">
                {$memberName(expense.member)}
              </span>
              {#if expense.account != null && $expenseRowDensity !== 'minimal'}
                <span class="block text-[10px] text-indigo-600 dark:text-indigo-400 font-medium">
                  🏦 {$accountById.get(expense.account)?.name ?? 'Joint'}
                </span>
              {/if}
            </td>

            <!-- Amount -->
            <td class="py-3 pr-4 text-right font-semibold text-neutral-900 dark:text-neutral-100 tabular-nums">
              <span class:privacy-masked={$privacyShield}>{$money(expense.amount)}</span>
            </td>

            <!-- Actions -->
            <td class="py-3 pr-1 text-right whitespace-nowrap">
              {#if isLocked(expense.date)}
                <span class="text-[10px] text-amber-600 dark:text-amber-400 px-1.5 py-0.5 rounded border border-amber-300 dark:border-amber-900/50 bg-amber-50 dark:bg-amber-950/30">🔒 Locked</span>
              {:else if confirmDeleteId === expense.id}
                <!-- Confirmation prompt -->
                <span class="inline-flex items-center gap-1.5">
                  <span class="text-xs text-neutral-500 dark:text-neutral-400">Delete?</span>
                  <button
                    id="confirm-delete-{expense.id}"
                    on:click={() => confirmDelete(expense.id)}
                    disabled={deletingId === expense.id}
                    class="px-2 py-0.5 rounded text-xs font-semibold bg-rose-600 hover:bg-rose-500 text-white
                           disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
                  >
                    {deletingId === expense.id ? '…' : 'Yes'}
                  </button>
                  <button
                    id="cancel-delete-{expense.id}"
                    on:click={() => (confirmDeleteId = null)}
                    class="px-2 py-0.5 rounded text-xs font-semibold bg-neutral-200 dark:bg-neutral-700 hover:bg-neutral-300 dark:hover:bg-neutral-600 text-neutral-800 dark:text-neutral-200
                           transition-colors"
                  >
                    No
                  </button>
                </span>
              {:else}
                <div class="inline-flex items-center gap-1">
                  <!-- Edit Icon Button -->
                  <button
                    id="edit-expense-{expense.id}"
                    on:click={() => (editing = expense)}
                    title="Edit expense"
                    class="p-1 rounded-lg text-neutral-400 hover:text-indigo-600 dark:hover:text-indigo-300 hover:bg-indigo-50 dark:hover:bg-indigo-950/40 transition-all duration-150"
                  >
                    <!-- Pencil Icon -->
                    <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L10.582 16.07a4.5 4.5 0 01-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 011.13-1.897l8.932-8.931zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0115.75 21H5.25A2.25 2.25 0 013 18.75V8.25A2.25 2.25 0 015.25 6H10" />
                    </svg>
                  </button>

                  <!-- Trash Icon Button -->
                  <button
                    id="delete-expense-{expense.id}"
                    on:click={() => { confirmDeleteId = expense.id; deleteError = null; }}
                    title="Delete expense"
                    class="p-1 rounded-lg text-neutral-400 hover:text-rose-600 dark:hover:text-red-400 hover:bg-rose-50 dark:hover:bg-red-950/40 transition-all duration-150"
                  >
                    <!-- Trash Icon -->
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4">
                      <path fill-rule="evenodd"
                        d="M8.75 1A2.75 2.75 0 0 0 6 3.75v.443c-.795.077-1.584.176-2.365.298a.75.75 0 1 0 .23 1.482l.149-.022.841 10.518A2.75 2.75 0 0 0 7.596 19h4.807a2.75 2.75 0 0 0 2.742-2.53l.841-10.52.149.023a.75.75 0 0 0 .23-1.482A41.03 41.03 0 0 0 14 4.193V3.75A2.75 2.75 0 0 0 11.25 1h-2.5ZM10 4c.84 0 1.673.025 2.5.075V3.75c0-.69-.56-1.25-1.25-1.25h-2.5c-.69 0-1.25.56-1.25 1.25v.325C8.327 4.025 9.16 4 10 4ZM8.58 7.72a.75.75 0 0 0-1.5.06l.3 7.5a.75.75 0 1 0 1.5-.06l-.3-7.5Zm4.34.06a.75.75 0 1 0-1.5-.06l-.3 7.5a.75.75 0 1 0 1.5.06l.3-7.5Z"
                        clip-rule="evenodd" />
                    </svg>
                  </button>
                </div>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

  {#if nextCursor}
    <div class="flex justify-center mt-4">
      <button id="expense-load-more" type="button" on:click={loadMore} disabled={loadingMore} class="btn-secondary text-xs">
        {loadingMore ? 'Loading…' : 'Load more'}
      </button>
    </div>
  {/if}

  {#if deleteError}
    <p class="text-xs text-rose-600 dark:text-red-400 mt-3 text-right">{deleteError}</p>
  {/if}
{/if}

<EntryEditModal entry={editing} on:close={() => (editing = null)} />
