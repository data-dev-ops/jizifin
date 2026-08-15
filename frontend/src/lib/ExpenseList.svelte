<script>
  /**
   * ExpenseList.svelte
   *
   * Renders the expenses store as a scrollable table with rich interactivity:
   * - Filters by `selectedMonth` store (YYYY-MM), text search, category, and tag.
   * - Quick inline tag assign/change popover for any listed expense.
   * - Full-featured Edit Expense modal (updating name, amount, date, who_paid,
   *   category, project, tag, and custom split overrides).
   * - Delete with click-through confirmation.
   * - Dates converted from YYYY-MM-DD (storage) → DD/MM/YYYY (display).
   */

  import { expenses, selectedMonth, projects, tags, settlements, users, splits, currencySymbol, jointCategories, jointAccountEnabled, showProjectsInExpense } from './stores.js';
  import { deleteExpense, updateExpense } from './api.js';

  $: activeUsers = $users.filter((u) => u.is_active);

  /** Set of plain category names assigned to the joint account */
  $: jointCategorySet = new Set(($jointCategories || []).map((c) => (typeof c === 'string' ? c : c.plain)));

  /** Lookup user color from the users store. */
  function userColor(name) {
    return $users.find((u) => u.name === name)?.color ?? '#6366f1';
  }

  /** True if a given expense_date month is locked */
  function isLocked(expDate) {
    const m = expDate ? expDate.slice(0, 7) : null;
    return m ? $settlements.some((s) => s.month === m) : false;
  }

  /** Build id→name lookup map from projects store */
  $: projectMap = Object.fromEntries($projects.map((p) => [p.id, p.name]));

  /** Build id→tag lookup map from tags store */
  $: tagMap = Object.fromEntries($tags.map((t) => [t.id, t]));

  /** YYYY-MM-DD → DD/MM/YYYY */
  function formatDate(iso) {
    if (!iso) return '—';
    const [y, m, d] = iso.split('-');
    return `${d}/${m}/${y}`;
  }

  /** Integer cents → currency amount */
  function formatAmount(cents) {
    return `${$currencySymbol}${(cents / 100).toFixed(2)}`;
  }

  // ── Filters & Search ────────────────────────────────────────────────────────
  let searchQuery = '';
  let selectedCategoryFilter = 'ALL';
  let selectedTagFilter = 'ALL';

  /** Expenses for current month */
  $: monthExpenses = $expenses.filter((e) =>
    e.expense_date && e.expense_date.startsWith($selectedMonth)
  );

  /** Filtered list based on search and category/tag selectors */
  $: filtered = monthExpenses.filter((e) => {
    if (selectedCategoryFilter !== 'ALL' && e.category !== selectedCategoryFilter) return false;
    if (selectedTagFilter !== 'ALL') {
      if (selectedTagFilter === 'UNTAGGED' && e.tag_id) return false;
      if (selectedTagFilter !== 'UNTAGGED' && String(e.tag_id) !== String(selectedTagFilter)) return false;
    }
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      const matchName = (e.name || '').toLowerCase().includes(q);
      const matchCategory = (e.category || '').toLowerCase().includes(q);
      const matchPayer = (e.who_paid || '').toLowerCase().includes(q);
      const matchTag = e.tag_id && tagMap[e.tag_id] && tagMap[e.tag_id].name.toLowerCase().includes(q);
      const matchProject = e.project_id && projectMap[e.project_id] && projectMap[e.project_id].toLowerCase().includes(q);
      if (!matchName && !matchCategory && !matchPayer && !matchTag && !matchProject) return false;
    }
    return true;
  });

  $: totalFilteredCents = filtered.reduce((acc, e) => acc + (e.cost_cents || 0), 0);

  // ── Quick Tag Popover State ────────────────────────────────────────────────
  let quickTagExpenseId = null;
  let tagUpdatingId = null;

  function toggleQuickTag(expenseId, e) {
    if (e) e.stopPropagation();
    quickTagExpenseId = quickTagExpenseId === expenseId ? null : expenseId;
  }

  async function setExpenseTag(expense, targetTagId) {
    tagUpdatingId = expense.id;
    try {
      await updateExpense(expense.id, { tag_id: targetTagId }, $selectedMonth);
      quickTagExpenseId = null;
    } catch (err) {
      alert(err.message || 'Failed to update tag.');
    } finally {
      tagUpdatingId = null;
    }
  }

  // ── Delete State ───────────────────────────────────────────────────────────
  let confirmDeleteId = null;
  let deletingId = null;
  let deleteError = null;

  function requestDelete(id) {
    confirmDeleteId = id;
    deleteError = null;
  }

  function cancelDelete() {
    confirmDeleteId = null;
    deleteError = null;
  }

  async function confirmDelete(id) {
    deletingId = id;
    deleteError = null;
    try {
      await deleteExpense(id, $selectedMonth);
      confirmDeleteId = null;
    } catch (err) {
      deleteError = err.message ?? 'Delete failed.';
    } finally {
      deletingId = null;
    }
  }

  // ── Edit Expense Modal State ───────────────────────────────────────────────
  let editingExpense = null;
  let editName = '';
  let editCostEuros = '';
  let editDate = '';
  let editCategory = '';
  let editWhoPaid = '';
  let editPaidByJoint = false;
  let editProjectId = null;
  let editTagId = null;
  let editCustomSplit = false;
  let editOverridePcts = {};
  let editUseSlider = true;
  let editSliderVal = 50;
  let editSaving = false;
  let editError = null;

  function openEditModal(expense) {
    editingExpense = expense;
    editName = expense.name || '';
    editCostEuros = ((expense.cost_cents || 0) / 100).toFixed(2);
    editDate = expense.expense_date || '';
    editCategory = expense.category || '';
    editPaidByJoint = !!expense.is_joint;
    editWhoPaid = expense.who_paid || (activeUsers[0]?.name ?? '');
    editProjectId = expense.project_id ?? null;
    editTagId = expense.tag_id ?? null;
    editError = null;

    // Split overrides
    editOverridePcts = {};
    if (expense.overrides && expense.overrides.length > 0) {
      editCustomSplit = true;
      expense.overrides.forEach((o) => {
        editOverridePcts[o.user_name] = o.pct;
      });
      if (activeUsers.length === 2 && editOverridePcts[activeUsers[0].name] !== undefined) {
        editSliderVal = Math.round(editOverridePcts[activeUsers[0].name]);
        editUseSlider = true;
      } else {
        editUseSlider = false;
      }
    } else {
      editCustomSplit = false;
      if (activeUsers.length === 2) {
        editSliderVal = 50;
        editOverridePcts[activeUsers[0].name] = 50;
        editOverridePcts[activeUsers[1].name] = 50;
      } else if (activeUsers.length > 0) {
        const base = Math.floor(100 / activeUsers.length);
        activeUsers.forEach((u) => { editOverridePcts[u.name] = base; });
      }
    }
  }

  function closeEditModal() {
    editingExpense = null;
    editError = null;
  }

  function handleEditSliderInput(e) {
    const val = Math.round(parseFloat(e.target.value));
    editSliderVal = val;
    if (activeUsers.length === 2) {
      editOverridePcts[activeUsers[0].name] = val;
      editOverridePcts[activeUsers[1].name] = 100 - val;
    }
  }

  $: editOverrideSum = Object.values(editOverridePcts).reduce((s, v) => s + (parseInt(v, 10) || 0), 0);
  $: editOverrideOk = editOverrideSum === 100;

  async function handleSaveEdit() {
    editError = null;
    if (!editName.trim()) {
      editError = 'Description is required.';
      return;
    }
    const parsedCost = parseFloat(editCostEuros);
    if (isNaN(parsedCost) || parsedCost <= 0) {
      editError = 'Please enter a valid positive amount.';
      return;
    }
    const costCents = Math.round(parsedCost * 100);

    if (!editDate || !/^\d{4}-\d{2}-\d{2}$/.test(editDate)) {
      editError = 'Valid date required (YYYY-MM-DD).';
      return;
    }
    if (!editPaidByJoint && !editWhoPaid) {
      editError = 'Please select who paid.';
      return;
    }
    if (!editCategory) {
      editError = 'Please select a category.';
      return;
    }

    editSaving = true;
    try {
      const payload = {
        name: editName.trim(),
        cost_cents: costCents,
        expense_date: editDate,
        who_paid: editPaidByJoint ? (activeUsers[0]?.name ?? 'John') : editWhoPaid,
        category: editCategory,
        project_id: editProjectId ? Number(editProjectId) : null,
        tag_id: editTagId ? Number(editTagId) : null,
        is_joint: editPaidByJoint,
      };

      if (!editPaidByJoint && editCustomSplit && editOverrideOk) {
        payload.overrides = Object.entries(editOverridePcts).map(([user_name, pct]) => ({
          user_name,
          pct: Math.round(parseFloat(pct)),
        }));
      } else if (!editCustomSplit) {
        payload.overrides = [];
      }

      await updateExpense(editingExpense.id, payload, $selectedMonth);
      closeEditModal();
    } catch (err) {
      editError = err.message || 'Failed to update expense.';
    } finally {
      editSaving = false;
    }
  }
</script>

<svelte:window on:click={() => (quickTagExpenseId = null)} />

<!-- ── Filter & Search Toolbar ────────────────────────────────────────────── -->
<div class="mb-4 space-y-3">
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
    <!-- Search Bar -->
    <div class="relative flex-1 min-w-[180px]">
      <span class="absolute left-3 top-1/2 -translate-y-1/2 text-neutral-500">
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
          <path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
      </span>
      <input
        id="expense-search-input"
        type="text"
        placeholder="Filter by name, payer, or tag…"
        bind:value={searchQuery}
        class="w-full bg-neutral-950/80 border border-neutral-800 rounded-xl pl-9 pr-3 py-1.5 text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-indigo-500 transition-colors"
      />
      {#if searchQuery}
        <button
          on:click={() => (searchQuery = '')}
          class="absolute right-2.5 top-1/2 -translate-y-1/2 text-neutral-500 hover:text-neutral-300 text-xs font-bold"
        >✕</button>
      {/if}
    </div>

    <!-- Category Selector Filter -->
    <select
      id="expense-category-filter"
      bind:value={selectedCategoryFilter}
      class="bg-neutral-950/80 border border-neutral-800 rounded-xl px-2.5 py-1.5 text-xs text-neutral-300 focus:outline-none focus:border-indigo-500 transition-colors"
    >
      <option value="ALL">All Categories</option>
      {#each $splits as s}
        <option value={s.category}>{s.category}</option>
      {/each}
    </select>

    <!-- Tag Selector Filter -->
    {#if $tags.length > 0}
      <select
        id="expense-tag-filter"
        bind:value={selectedTagFilter}
        class="bg-neutral-950/80 border border-neutral-800 rounded-xl px-2.5 py-1.5 text-xs text-neutral-300 focus:outline-none focus:border-indigo-500 transition-colors"
      >
        <option value="ALL">All Tags</option>
        <option value="UNTAGGED">No Tag</option>
        {#each $tags as t}
          <option value={t.id}>● {t.name}</option>
        {/each}
      </select>
    {/if}
  </div>

  <!-- Summary Stats Bar -->
  <div class="flex items-center justify-between text-xs text-neutral-400 px-1">
    <span>
      Showing <strong class="text-neutral-200">{filtered.length}</strong> of {monthExpenses.length} entries
    </span>
    <span class="font-semibold text-neutral-200">
      Total: <span class="text-indigo-400 font-bold">{formatAmount(totalFilteredCents)}</span>
    </span>
  </div>
</div>

{#if filtered.length === 0}
  <div class="flex flex-col items-center justify-center py-16 text-center gap-3">
    <div class="w-14 h-14 rounded-2xl bg-neutral-800/80 flex items-center justify-center mb-1">
      <svg class="w-7 h-7 text-neutral-600" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
        <path stroke-linecap="round" stroke-linejoin="round"
          d="M9 14l2 2 4-4M7.5 3.75A1.5 1.5 0 006 5.25v13.5A1.5 1.5 0 007.5 20.25h9A1.5 1.5 0 0018 18.75V5.25A1.5 1.5 0 0016.5 3.75H7.5z" />
      </svg>
    </div>
    <p class="text-neutral-400 text-sm font-medium">
      {monthExpenses.length === 0 ? 'No expenses for this month.' : 'No matching expenses found.'}
    </p>
    <p class="text-neutral-600 text-xs max-w-xs">
      {#if monthExpenses.length === 0}
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
        <tr class="border-b border-neutral-800">
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
          <tr class="border-b border-neutral-800/60 hover:bg-neutral-800/40 transition-colors group">
            <!-- Date -->
            <td class="py-3 pr-4 pl-1 text-neutral-500 tabular-nums whitespace-nowrap">
              {formatDate(expense.expense_date)}
            </td>

            <!-- Description & Badges -->
            <td class="py-3 pr-4 text-neutral-200 max-w-[200px]" title={expense.name}>
              <span class="block truncate font-medium text-neutral-100">{expense.name}</span>
              
              <div class="flex items-center gap-1.5 flex-wrap mt-1">
                {#if expense.project_id && projectMap[expense.project_id]}
                  <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px]
                               bg-indigo-950/60 text-indigo-400 border border-indigo-900/60 font-medium">
                    ▰ {projectMap[expense.project_id]}
                  </span>
                {/if}

                <!-- Tag Badge & Quick Tagging Trigger -->
                <div class="relative inline-block" on:click|stopPropagation>
                  {#if expense.tag_id && tagMap[expense.tag_id]}
                    {@const tag = tagMap[expense.tag_id]}
                    <button
                      type="button"
                      id="tag-badge-{expense.id}"
                      on:click={(e) => toggleQuickTag(expense.id, e)}
                      class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] border font-medium hover:brightness-125 transition-all"
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
                      class="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded text-[10px] text-neutral-500 hover:text-neutral-300 hover:bg-neutral-800 border border-transparent hover:border-neutral-700 transition-all font-medium"
                      title="Add tag to this expense"
                    >
                      + Tag
                    </button>
                  {/if}

                  <!-- Quick Tag Dropdown Popover -->
                  {#if quickTagExpenseId === expense.id}
                    <div
                      class="absolute left-0 top-full mt-1.5 z-40 w-48 bg-neutral-900 border border-neutral-700 rounded-xl shadow-2xl p-2 space-y-1 text-left animate-fadeIn"
                    >
                      <p class="text-[10px] font-semibold text-neutral-400 uppercase tracking-wider px-2 py-1">Assign Tag</p>
                      
                      {#if $tags.length === 0}
                        <p class="text-xs text-neutral-500 px-2 py-1.5">No tags configured yet.</p>
                      {:else}
                        <div class="max-h-40 overflow-y-auto space-y-0.5">
                          {#each $tags as t}
                            <button
                              type="button"
                              on:click={() => setExpenseTag(expense, t.id)}
                              disabled={tagUpdatingId === expense.id}
                              class="w-full flex items-center gap-2 px-2 py-1.5 rounded-lg text-xs hover:bg-neutral-800 transition-colors text-left {expense.tag_id === t.id ? 'bg-neutral-800/80 font-semibold' : ''}"
                            >
                              <span class="w-2.5 h-2.5 rounded-full flex-none" style="background-color: {t.color}"></span>
                              <span class="truncate flex-1 text-neutral-200">{t.name}</span>
                              {#if expense.tag_id === t.id}
                                <span class="text-indigo-400 text-xs">✓</span>
                              {/if}
                            </button>
                          {/each}
                        </div>
                      {/if}

                      {#if expense.tag_id}
                        <div class="border-t border-neutral-800 pt-1 mt-1">
                          <button
                            type="button"
                            on:click={() => setExpenseTag(expense, null)}
                            disabled={tagUpdatingId === expense.id}
                            class="w-full text-left px-2 py-1 rounded-lg text-[11px] text-red-400 hover:bg-red-950/40 transition-colors"
                          >
                            ✕ Remove Tag
                          </button>
                        </div>
                      {/if}
                    </div>
                  {/if}
                </div>

                {#if expense.is_joint}
                  <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px]
                               bg-indigo-950/80 text-indigo-300 border border-indigo-700/60 font-semibold"
                        title="Paid by Joint Account">
                    🏦 Joint
                  </span>
                  {#if !jointCategorySet.has(expense.category)}
                    <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px]
                                 bg-amber-950/80 text-amber-300 border border-amber-700/60 font-semibold"
                          title="Category is not in the list of coupled joint account categories">
                      ⚠️ Not Coupled
                    </span>
                  {/if}
                {/if}

                {#if expense.overrides && expense.overrides.length > 0}
                  <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px]
                               bg-amber-950/60 text-amber-400 border border-amber-800/60"
                        title="Custom split: {expense.overrides.map(o => o.user_name + ' ' + o.pct + '%').join(' / ')}">
                    ✶ custom split
                  </span>
                {/if}
              </div>
            </td>

            <!-- Category -->
            <td class="py-3 pr-4">
              <span class="inline-flex items-center px-2 py-0.5 rounded-md bg-neutral-800 text-xs text-neutral-300 border border-neutral-700 font-medium">
                {expense.category}
              </span>
            </td>

            <!-- Paid by -->
            <td class="py-3 pr-4">
              <span class="text-xs font-semibold tabular-nums" style="color: {userColor(expense.who_paid)}">
                {expense.who_paid}
              </span>
              {#if expense.is_joint}
                <span class="block text-[10px] text-indigo-400 font-medium">(Joint)</span>
              {/if}
            </td>

            <!-- Amount -->
            <td class="py-3 pr-4 text-right font-semibold text-neutral-100 tabular-nums">
              {formatAmount(expense.cost_cents)}
            </td>

            <!-- Actions -->
            <td class="py-3 pr-1 text-right whitespace-nowrap">
              {#if isLocked(expense.expense_date)}
                <span class="text-[10px] text-amber-600 px-1.5 py-0.5 rounded border border-amber-900/50 bg-amber-950/30">🔒 Locked</span>
              {:else if confirmDeleteId === expense.id}
                <!-- Confirmation prompt -->
                <span class="inline-flex items-center gap-1.5">
                  <span class="text-xs text-neutral-400">Delete?</span>
                  <button
                    id="confirm-delete-{expense.id}"
                    on:click={() => confirmDelete(expense.id)}
                    disabled={deletingId === expense.id}
                    class="px-2 py-0.5 rounded text-xs font-semibold bg-red-600 hover:bg-red-500
                           disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
                  >
                    {deletingId === expense.id ? '…' : 'Yes'}
                  </button>
                  <button
                    id="cancel-delete-{expense.id}"
                    on:click={cancelDelete}
                    class="px-2 py-0.5 rounded text-xs font-semibold bg-neutral-700 hover:bg-neutral-600
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
                    on:click={() => openEditModal(expense)}
                    title="Edit expense"
                    class="p-1 rounded-lg text-neutral-500 hover:text-indigo-300 hover:bg-indigo-950/40 transition-all duration-150"
                  >
                    <!-- Pencil Icon -->
                    <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L10.582 16.07a4.5 4.5 0 01-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 011.13-1.897l8.932-8.931zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0115.75 21H5.25A2.25 2.25 0 013 18.75V8.25A2.25 2.25 0 015.25 6H10" />
                    </svg>
                  </button>

                  <!-- Trash Icon Button -->
                  <button
                    id="delete-expense-{expense.id}"
                    on:click={() => requestDelete(expense.id)}
                    title="Delete expense"
                    class="p-1 rounded-lg text-neutral-500 hover:text-red-400 hover:bg-red-950/40 transition-all duration-150"
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

  {#if deleteError}
    <p class="text-xs text-red-400 mt-3 text-right">{deleteError}</p>
  {/if}
{/if}

<!-- ── Full-Featured Edit Expense Modal ────────────────────────────────────── -->
{#if editingExpense}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 backdrop-blur-sm animate-fadeIn">
    <div class="bg-neutral-900 border border-neutral-800 rounded-2xl p-5 sm:p-6 max-w-lg w-full shadow-2xl max-h-[90vh] overflow-y-auto space-y-4 text-left">
      <div class="flex items-center justify-between border-b border-neutral-800 pb-3">
        <h3 class="text-base font-bold text-white flex items-center gap-2">
          <span>Edit Expense</span>
          <span class="text-xs font-normal text-neutral-400">({formatDate(editingExpense.expense_date)})</span>
        </h3>
        <button
          on:click={closeEditModal}
          class="text-neutral-500 hover:text-neutral-200 text-lg font-bold p-1 leading-none"
        >✕</button>
      </div>

      {#if editError}
        <div class="p-3 bg-red-950/60 border border-red-800 rounded-xl text-red-300 text-xs">
          {editError}
        </div>
      {/if}

      <form on:submit|preventDefault={handleSaveEdit} class="space-y-3.5">
        <!-- 1. Description -->
        <div>
          <label for="edit-expense-name" class="block text-xs font-medium text-neutral-400 mb-1">Description</label>
          <input
            id="edit-expense-name"
            type="text"
            bind:value={editName}
            class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100 focus:outline-none focus:border-indigo-500"
          />
        </div>

        <!-- 2. Amount & Date in Grid -->
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <div>
            <label for="edit-expense-cost" class="block text-xs font-medium text-neutral-400 mb-1">Amount ({$currencySymbol})</label>
            <input
              id="edit-expense-cost"
              type="number"
              min="0.01"
              step="0.01"
              bind:value={editCostEuros}
              class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100 focus:outline-none focus:border-indigo-500"
            />
          </div>
          <div>
            <label for="edit-expense-date" class="block text-xs font-medium text-neutral-400 mb-1">Date</label>
            <input
              id="edit-expense-date"
              type="date"
              bind:value={editDate}
              class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100 focus:outline-none focus:border-indigo-500 [color-scheme:dark]"
            />
          </div>
        </div>

        <!-- 3. Category -->
        <div>
          <label for="edit-expense-category" class="block text-xs font-medium text-neutral-400 mb-1">Category</label>
          <select
            id="edit-expense-category"
            bind:value={editCategory}
            class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100 focus:outline-none focus:border-indigo-500"
          >
            {#each $splits as s}
              <option value={s.category}>{s.category}</option>
            {/each}
          </select>
        </div>

        <!-- 4. Project & Tag in Grid -->
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
          {#if $projects.length > 0}
            <div>
              <label for="edit-expense-project" class="block text-xs font-medium text-neutral-400 mb-1">Project</label>
              <select
                id="edit-expense-project"
                bind:value={editProjectId}
                class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100 focus:outline-none focus:border-indigo-500"
              >
                <option value={null}>— No Project —</option>
                {#each $projects as p}
                  <option value={p.id}>{p.name}{p.is_joint ? ' 🏦' : ''}</option>
                {/each}
              </select>
            </div>
          {/if}

          <div>
            <label for="edit-expense-tag" class="block text-xs font-medium text-neutral-400 mb-1">Tag</label>
            <select
              id="edit-expense-tag"
              bind:value={editTagId}
              class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100 focus:outline-none focus:border-indigo-500"
            >
              <option value={null}>— No Tag —</option>
              {#each $tags as t}
                <option value={t.id}>● {t.name}</option>
              {/each}
            </select>
          </div>
        </div>

        <!-- 5. Who Paid -->
        <div>
          <p class="block text-xs font-medium text-neutral-400 mb-2">Paid by</p>
          <div class="flex flex-wrap gap-2.5">
            {#each activeUsers as u}
              <label class="flex items-center gap-2 text-xs text-neutral-200 cursor-pointer bg-neutral-800/80 px-3 py-1.5 rounded-lg border transition-all {editWhoPaid === u.name && !editPaidByJoint ? 'border-indigo-500 bg-indigo-950/40 text-white' : 'border-neutral-700 hover:border-neutral-600'}">
                <input
                  type="radio"
                  name="edit-who-paid"
                  value={u.name}
                  checked={editWhoPaid === u.name && !editPaidByJoint}
                  on:change={() => {
                    editWhoPaid = u.name;
                    editPaidByJoint = false;
                  }}
                  class="sr-only"
                />
                <span class="w-2.5 h-2.5 rounded-full" style="background-color: {u.color}"></span>
                <span>{u.name}</span>
              </label>
            {/each}

            {#if $jointAccountEnabled}
              <label class="flex items-center gap-2 text-xs text-indigo-300 cursor-pointer bg-indigo-950/60 px-3 py-1.5 rounded-lg border transition-all {editPaidByJoint ? 'border-indigo-400 bg-indigo-900/60 font-semibold' : 'border-indigo-800/60 hover:border-indigo-600'}">
                <input
                  type="radio"
                  name="edit-who-paid"
                  value="joint"
                  checked={editPaidByJoint}
                  on:change={() => {
                    editPaidByJoint = true;
                  }}
                  class="sr-only"
                />
                <span>🏦 Joint Account</span>
              </label>
            {/if}
          </div>
        </div>

        <!-- 6. Custom Split Allocations -->
        {#if !editPaidByJoint}
          <div class="border-t border-neutral-800 pt-3">
            <label class="flex items-center gap-2 cursor-pointer select-none">
              <input
                id="edit-custom-split-toggle"
                type="checkbox"
                bind:checked={editCustomSplit}
                class="rounded bg-neutral-800 border-neutral-700 text-indigo-600 focus:ring-0"
              />
              <span class="text-xs font-semibold text-neutral-300">Custom Split Allocation</span>
            </label>

            {#if editCustomSplit}
              <div class="mt-2.5 p-3 bg-neutral-950/80 border border-neutral-800 rounded-xl space-y-2">
                {#if activeUsers.length === 2 && editUseSlider}
                  <div class="flex items-center justify-between gap-3 py-1">
                    <span class="text-xs font-semibold" style="color: {activeUsers[0].color}">
                      {activeUsers[0].name}: {editSliderVal}%
                    </span>
                    <input
                      type="range" min="0" max="100" step="1"
                      value={editSliderVal}
                      on:input={handleEditSliderInput}
                      class="flex-1 h-2 bg-neutral-800 rounded-lg cursor-pointer accent-indigo-500"
                    />
                    <span class="text-xs font-semibold" style="color: {activeUsers[1].color}">
                      {activeUsers[1].name}: {100 - editSliderVal}%
                    </span>
                  </div>
                  <div class="flex justify-between items-center text-[10px] text-neutral-500 pt-1">
                    <button type="button" on:click={() => (editUseSlider = false)} class="underline hover:text-neutral-300">
                      Manual % Inputs
                    </button>
                    <button
                      type="button"
                      on:click={() => {
                        editSliderVal = 50;
                        editOverridePcts[activeUsers[0].name] = 50;
                        editOverridePcts[activeUsers[1].name] = 50;
                      }}
                      class="hover:text-neutral-300"
                    >
                      Reset 50/50
                    </button>
                  </div>
                {:else}
                  <div class="space-y-1.5">
                    {#each activeUsers as u}
                      <div class="flex items-center gap-2">
                        <span class="text-xs font-semibold w-16 truncate" style="color: {u.color}">{u.name}</span>
                        <input
                          type="number" min="0" max="100" step="1"
                          bind:value={editOverridePcts[u.name]}
                          class="w-16 bg-neutral-800 border border-neutral-700 rounded px-2 py-1 text-xs text-neutral-100"
                        />
                        <span class="text-xs text-neutral-500">%</span>
                      </div>
                    {/each}
                    <div class="flex items-center justify-between pt-1">
                      <span class="text-[10px] {editOverrideOk ? 'text-emerald-400' : 'text-amber-400'} font-semibold">
                        Sum: {editOverrideSum}% (must equal 100%)
                      </span>
                      {#if activeUsers.length === 2}
                        <button type="button" on:click={() => (editUseSlider = true)} class="text-[10px] underline text-neutral-500 hover:text-neutral-300">
                          Use Slider
                        </button>
                      {/if}
                    </div>
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        {/if}

        <div class="flex items-center justify-end gap-2.5 pt-3 border-t border-neutral-800">
          <button
            type="button"
            on:click={closeEditModal}
            class="px-4 py-2 rounded-xl text-xs font-semibold text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800 transition-colors"
          >
            Cancel
          </button>
          <button
            id="save-expense-edit-btn"
            type="submit"
            disabled={editSaving}
            class="px-5 py-2 rounded-xl text-xs font-semibold bg-indigo-600 hover:bg-indigo-500 text-white disabled:opacity-50 transition-colors shadow-md shadow-indigo-600/30"
          >
            {editSaving ? 'Saving…' : 'Save Changes'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}
