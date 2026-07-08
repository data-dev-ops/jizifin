<script>
  /**
   * ExpenseList.svelte
   *
   * Renders the expenses store as a scrollable table.
   * Dates are converted from YYYY-MM-DD (storage) → DD/MM/YYYY (display)
   * at the presentation tier per AGENTS.md.
   *
   * Filters by `selectedMonth` store (YYYY-MM).
   * Delete requires click-through confirmation.
   */

  import { expenses, selectedMonth, projects, settlements, users, currencySymbol } from './stores.js';
  import { deleteExpense } from './api.js';

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

  // Per-row delete state: null = idle, id = pending confirmation
  let confirmDeleteId = null;
  let deletingId = null;
  let deleteError = null;

  /** Expenses filtered by the selected month (YYYY-MM prefix match on expense_date). */
  $: filtered = $expenses.filter((e) =>
    e.expense_date && e.expense_date.startsWith($selectedMonth)
  );

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
</script>

{#if filtered.length === 0}
  <div class="flex flex-col items-center justify-center py-16 text-center gap-3">
    <div class="w-14 h-14 rounded-2xl bg-neutral-800/80 flex items-center justify-center mb-1">
      <!-- Receipt icon -->
      <svg class="w-7 h-7 text-neutral-600" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
        <path stroke-linecap="round" stroke-linejoin="round"
          d="M9 14l2 2 4-4M7.5 3.75A1.5 1.5 0 006 5.25v13.5A1.5 1.5 0 007.5 20.25h9A1.5 1.5 0 0018 18.75V5.25A1.5 1.5 0 0016.5 3.75H7.5z" />
      </svg>
    </div>
    <p class="text-neutral-400 text-sm font-medium">No expenses for this month.</p>
    <p class="text-neutral-600 text-xs max-w-xs">
      Expenses you log will appear here. Use the form on the left to add your first entry.
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
          <th class="text-right text-xs font-medium text-neutral-500 pb-3 pr-1"></th>
        </tr>
      </thead>
      <tbody>
        {#each filtered as expense (expense.id)}
          <tr class="border-b border-neutral-800/60 hover:bg-neutral-800/40 transition-colors group">
            <td class="py-3 pr-4 pl-1 text-neutral-500 tabular-nums whitespace-nowrap">
              {formatDate(expense.expense_date)}
            </td>
            <td class="py-3 pr-4 text-neutral-200 max-w-[160px]" title={expense.name}>
              <span class="block truncate">{expense.name}</span>
              {#if expense.project_id && projectMap[expense.project_id]}
                <span class="inline-flex items-center mt-0.5 px-1.5 py-0.5 rounded text-[10px]
                             bg-indigo-950/60 text-indigo-400 border border-indigo-900/60">
                  ▰ {projectMap[expense.project_id]}
                </span>
              {/if}
              {#if expense.overrides && expense.overrides.length > 0}
                <span class="inline-flex items-center mt-0.5 px-1.5 py-0.5 rounded text-[10px]
                             bg-amber-950/60 text-amber-400 border border-amber-800/60"
                      title="Custom split: {expense.overrides.map(o => o.user_name + ' ' + o.pct + '%').join(' / ')}">
                  ✦ custom split
                </span>
              {/if}
            </td>
            <td class="py-3 pr-4">
              <span class="inline-flex items-center px-2 py-0.5 rounded-md bg-neutral-800 text-xs text-neutral-400 border border-neutral-700">
                {expense.category}
              </span>
            </td>
            <td class="py-3 pr-4">
              <span class="text-xs font-semibold tabular-nums" style="color: {userColor(expense.who_paid)}">
                {expense.who_paid}
              </span>
            </td>
            <td class="py-3 pr-4 text-right font-semibold text-neutral-100 tabular-nums">
              {formatAmount(expense.cost_cents)}
            </td>
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
                <!-- Trash icon, visible on row hover -->
                <button
                  id="delete-expense-{expense.id}"
                  on:click={() => requestDelete(expense.id)}
                  title="Delete expense"
                  class="opacity-0 group-hover:opacity-100 p-1 rounded-lg text-neutral-500
                         hover:text-red-400 hover:bg-red-950/40 transition-all duration-150"
                >
                  <!-- Heroicon: trash -->
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor"
                       class="w-4 h-4">
                    <path fill-rule="evenodd"
                      d="M8.75 1A2.75 2.75 0 0 0 6 3.75v.443c-.795.077-1.584.176-2.365.298a.75.75 0 1 0 .23 1.482l.149-.022.841 10.518A2.75 2.75 0 0 0 7.596 19h4.807a2.75 2.75 0 0 0 2.742-2.53l.841-10.52.149.023a.75.75 0 0 0 .23-1.482A41.03 41.03 0 0 0 14 4.193V3.75A2.75 2.75 0 0 0 11.25 1h-2.5ZM10 4c.84 0 1.673.025 2.5.075V3.75c0-.69-.56-1.25-1.25-1.25h-2.5c-.69 0-1.25.56-1.25 1.25v.325C8.327 4.025 9.16 4 10 4ZM8.58 7.72a.75.75 0 0 0-1.5.06l.3 7.5a.75.75 0 1 0 1.5-.06l-.3-7.5Zm4.34.06a.75.75 0 1 0-1.5-.06l-.3 7.5a.75.75 0 1 0 1.5.06l.3-7.5Z"
                      clip-rule="evenodd" />
                  </svg>
                </button>
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

  <p class="text-xs text-neutral-600 mt-4 text-right">
    {filtered.length} {filtered.length === 1 ? 'entry' : 'entries'} in {$selectedMonth}
  </p>
{/if}
