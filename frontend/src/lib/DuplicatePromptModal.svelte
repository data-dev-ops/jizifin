<script>
  /**
   * DuplicatePromptModal.svelte
   *
   * Accessible confirmation dialog displayed when prospective expenses match
   * existing ledger entries by category and amount.
   */

  import { createEventDispatcher } from 'svelte';
  import { currencySymbol } from './stores.js';

  export let isOpen = false;
  /**
   * items: Array of {
   *   prospective: { name, cost_cents, category, expense_date, who_paid },
   *   matches: Array<{ id, name, cost_cents, category, expense_date, who_paid }>
   * }
   */
  export let items = [];
  export let isBatch = false;

  const dispatch = createEventDispatcher();

  function handleConfirm() {
    dispatch('confirm');
  }

  function handleCancel() {
    dispatch('cancel');
  }

  function handleSkipDuplicates() {
    dispatch('skipDuplicates');
  }

  function handleKeydown(e) {
    if (e.key === 'Escape' && isOpen) {
      handleCancel();
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if isOpen && items.length > 0}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-4 overflow-hidden"
    role="dialog"
    aria-modal="true"
    aria-labelledby="duplicate-modal-title"
  >
    <!-- Backdrop -->
    <button
      type="button"
      class="fixed inset-0 bg-neutral-900/60 dark:bg-black/75 backdrop-blur-sm transition-opacity duration-200 cursor-default w-full h-full border-0 p-0 m-0"
      on:click={handleCancel}
      aria-label="Close dialog backdrop"
      tabindex="-1"
    />

    <!-- Dialog Content -->
    <div class="relative z-10 bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 rounded-2xl shadow-2xl w-full max-w-lg overflow-hidden animate-in fade-in zoom-in-95 duration-150">
      <div class="p-5 sm:p-6 space-y-4">
        <div class="flex items-start gap-3">
          <div class="w-10 h-10 rounded-xl bg-amber-100 dark:bg-amber-950/60 text-amber-600 dark:text-amber-400 flex items-center justify-center text-xl shrink-0">
            ⚠️
          </div>
          <div>
            <h3 id="duplicate-modal-title" class="text-sm sm:text-base font-bold text-neutral-900 dark:text-white">
              Potential Duplicate Expense Detected
            </h3>
            <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-1">
              {#if items.length === 1}
                This expense might already have been entered:
              {:else}
                The following {items.length} expenses might already have been entered:
              {/if}
            </p>
          </div>
        </div>

        <!-- Matching Entries List -->
        <div class="space-y-3 max-h-60 overflow-y-auto pr-1">
          {#each items as item}
            <div class="p-3 rounded-xl bg-neutral-50 dark:bg-neutral-800/60 border border-neutral-200 dark:border-neutral-700/80 space-y-2 text-xs">
              {#if isBatch}
                <div class="flex items-center justify-between pb-1 border-b border-neutral-200 dark:border-neutral-700 font-medium">
                  <span class="text-neutral-900 dark:text-white font-semibold">
                    New: {item.prospective.name}
                  </span>
                  <span class="font-mono text-neutral-700 dark:text-neutral-300">
                    {$currencySymbol}{(item.prospective.cost_cents / 100).toFixed(2)} ({item.prospective.category})
                  </span>
                </div>
              {/if}

              <div class="text-[11px] text-neutral-500 dark:text-neutral-400 uppercase tracking-wider font-bold">
                Matching existing entry{item.matches.length > 1 ? 's' : ''}:
              </div>

              {#each item.matches as match}
                <div class="flex items-start justify-between gap-2 pl-2.5 border-l-2 border-amber-400 dark:border-amber-500">
                  <div>
                    <div class="font-semibold text-neutral-800 dark:text-neutral-200">{match.name}</div>
                    <div class="text-neutral-500 dark:text-neutral-400 text-[11px]">
                      {match.expense_date} • Category: <strong class="text-neutral-700 dark:text-neutral-300">{match.category}</strong> • Paid by {match.who_paid}
                    </div>
                  </div>
                  <div class="font-mono font-bold text-neutral-900 dark:text-white whitespace-nowrap">
                    {$currencySymbol}{(match.cost_cents / 100).toFixed(2)}
                  </div>
                </div>
              {/each}
            </div>
          {/each}
        </div>

        <p class="text-xs font-semibold text-neutral-800 dark:text-neutral-200">
          Do you still want to add {items.length > 1 ? 'them' : 'it'}?
        </p>
      </div>

      <!-- Footer Buttons -->
      <footer class="px-5 py-3.5 bg-neutral-50/70 dark:bg-neutral-900/70 border-t border-neutral-200 dark:border-neutral-800 flex flex-wrap items-center justify-end gap-2">
        <button
          type="button"
          on:click={handleCancel}
          class="px-3.5 py-2 rounded-xl text-xs font-semibold text-neutral-700 dark:text-neutral-300 hover:bg-neutral-200 dark:hover:bg-neutral-800 transition"
        >
          No, Cancel
        </button>

        {#if isBatch}
          <button
            type="button"
            on:click={handleSkipDuplicates}
            class="px-3.5 py-2 rounded-xl text-xs font-bold text-amber-700 dark:text-amber-300 bg-amber-100 dark:bg-amber-950/60 hover:bg-amber-200 dark:hover:bg-amber-900/60 transition"
          >
            No, Skip Duplicates
          </button>
        {/if}

        <button
          type="button"
          on:click={handleConfirm}
          class="px-4 py-2 rounded-xl text-xs font-bold text-white bg-indigo-600 hover:bg-indigo-500 transition shadow-md shadow-indigo-600/20"
        >
          Yes, Add Anyway
        </button>
      </footer>
    </div>
  </div>
{/if}
