<script>
  import { recurringExpenses, splits, users, currencySymbol } from './stores.js';
  import { createRecurring, deleteRecurring } from './api.js';

  $: activeUsers = $users.filter((u) => u.is_active);

  let form = { name: '', cost_euros: '', who_paid: '', category: '', day_of_month: 1 };
  // Default who_paid to first active user once users load
  $: if (!form.who_paid && activeUsers.length > 0) form.who_paid = activeUsers[0].name;
  let saving = false;
  let error = '';

  // Per-row delete confirmation state
  let confirmDeleteId = null;
  let deletingId = null;

  async function handleSubmit() {
    error = '';
    const cost_cents = Math.round(parseFloat(form.cost_euros) * 100);
    if (!form.name || isNaN(cost_cents) || cost_cents <= 0 || !form.category) {
      error = 'Fill in all fields with valid values.';
      return;
    }
    saving = true;
    try {
      await createRecurring({
        name: form.name,
        cost_cents,
        who_paid: form.who_paid,
        category: form.category,
        day_of_month: Number(form.day_of_month),
      });
      form = { name: '', cost_euros: '', who_paid: activeUsers[0]?.name ?? '', category: '', day_of_month: 1 };
    } catch (e) {
      error = e.message;
    } finally {
      saving = false;
    }
  }

  function requestDelete(id) {
    confirmDeleteId = id;
  }

  function cancelDelete() {
    confirmDeleteId = null;
  }

  async function confirmDelete(id) {
    error = '';
    deletingId = id;
    try {
      await deleteRecurring(id);
      confirmDeleteId = null;
    } catch (e) {
      error = e.message;
    } finally {
      deletingId = null;
    }
  }

  function ordinal(n) {
    const s = ['th','st','nd','rd'];
    const v = n % 100;
    return n + (s[(v - 20) % 10] || s[v] || s[0]);
  }
</script>

<div>
  <h2 class="text-base font-semibold text-neutral-200 mb-1">Recurring Expenses</h2>
  <p class="text-xs text-neutral-500 mb-5">Templates are automatically inserted as expenses each month on their scheduled day.</p>

  <!-- ── Existing templates ─────────────────────────────── -->
  {#if $recurringExpenses.length === 0}
    <div class="text-center text-neutral-600 text-sm py-8 border border-dashed border-neutral-800 rounded-xl mb-5">
      No recurring expenses configured yet.
    </div>
  {:else}
    <div class="overflow-x-auto rounded-xl border border-neutral-800 mb-5">
      <table class="w-full text-sm border-collapse">
        <thead>
          <tr class="bg-neutral-950/60">
            <th class="text-left text-xs font-semibold text-neutral-500 uppercase tracking-wider px-4 py-2.5">Name</th>
            <th class="text-left text-xs font-semibold text-neutral-500 uppercase tracking-wider px-4 py-2.5">Amount</th>
            <th class="text-left text-xs font-semibold text-neutral-500 uppercase tracking-wider px-4 py-2.5">Paid by</th>
            <th class="text-left text-xs font-semibold text-neutral-500 uppercase tracking-wider px-4 py-2.5">Category</th>
            <th class="text-left text-xs font-semibold text-neutral-500 uppercase tracking-wider px-4 py-2.5">Day</th>
            <th class="px-4 py-2.5"></th>
          </tr>
        </thead>
        <tbody>
          {#each $recurringExpenses as rec (rec.id)}
            {@const payerColor = ($users.find(u => u.name === rec.who_paid)?.color ?? '#6366f1')}
            <tr class="border-t border-neutral-800/70 hover:bg-neutral-800/30 transition-colors group">
              <td class="px-4 py-3 font-medium text-neutral-100">{rec.name}</td>
              <td class="px-4 py-3 font-semibold tabular-nums text-sky-400">{$currencySymbol}{(rec.cost_cents / 100).toFixed(2)}</td>
              <td class="px-4 py-3">
                <span
                  class="inline-block px-2 py-0.5 rounded-full text-xs font-semibold"
                  style="background-color:{payerColor}22; color:{payerColor}"
                >{rec.who_paid}</span>
              </td>
              <td class="px-4 py-3 text-xs text-neutral-400">{rec.category}</td>
              <td class="px-4 py-3 text-neutral-400 text-xs">{ordinal(rec.day_of_month)}</td>
              <td class="px-4 py-3 text-right whitespace-nowrap">
                {#if confirmDeleteId === rec.id}
                  <span class="inline-flex items-center gap-1.5">
                    <span class="text-xs text-neutral-400">Remove?</span>
                    <button
                      on:click={() => confirmDelete(rec.id)}
                      disabled={deletingId === rec.id}
                      class="px-2 py-0.5 rounded text-xs font-semibold bg-red-600 hover:bg-red-500
                             disabled:opacity-40 transition-colors"
                    >{deletingId === rec.id ? '…' : 'Yes'}</button>
                    <button
                      on:click={cancelDelete}
                      class="px-2 py-0.5 rounded text-xs font-semibold bg-neutral-700 hover:bg-neutral-600 transition-colors"
                    >No</button>
                  </span>
                {:else}
                  <button
                    on:click={() => requestDelete(rec.id)}
                    title="Remove"
                    class="opacity-0 group-hover:opacity-100 p-1 rounded-lg text-neutral-500
                           hover:text-red-400 hover:bg-red-950/40 transition-all duration-150"
                  >
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
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

  <!-- ── Add new template ───────────────────────────────── -->
  <div class="bg-neutral-950/60 border border-neutral-800 rounded-xl p-5">
    <h3 class="text-sm font-semibold text-neutral-200 mb-4">Add Recurring Expense</h3>
    {#if error}
      <div class="bg-red-950/40 border border-red-900/60 text-red-400 rounded-lg px-3 py-2 text-xs mb-4">{error}</div>
    {/if}
    <form class="flex flex-col gap-4" on:submit|preventDefault={handleSubmit}>
      <div class="flex flex-wrap gap-3">
        <div class="flex flex-col gap-1 flex-1 min-w-[140px]">
          <label for="rec-name" class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wider">Name</label>
          <input
            id="rec-name" type="text" bind:value={form.name}
            placeholder="e.g. Spotify" maxlength="96"
            class="bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100
                   placeholder-neutral-600 focus:outline-none focus:border-indigo-500 focus:ring-1
                   focus:ring-indigo-500 transition-colors"
          />
        </div>
        <div class="flex flex-col gap-1 min-w-[120px] max-w-[140px]">
          <label for="rec-amount" class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wider">Amount ({$currencySymbol})</label>
          <input
            id="rec-amount" type="number" min="0.01" step="0.01"
            bind:value={form.cost_euros} placeholder="0.00"
            class="bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100
                   placeholder-neutral-600 focus:outline-none focus:border-indigo-500 focus:ring-1
                   focus:ring-indigo-500 transition-colors"
          />
        </div>
      </div>
      <div class="flex flex-wrap gap-3">
        <div class="flex flex-col gap-1 flex-1 min-w-[130px]">
          <label for="rec-payer" class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wider">Paid by</label>
          <select
            id="rec-payer" bind:value={form.who_paid}
            class="bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100
                   focus:outline-none focus:border-indigo-500 transition-colors"
          >
            {#each activeUsers as u (u.name)}
              <option value={u.name}>{u.name}</option>
            {/each}
          </select>
        </div>
        <div class="flex flex-col gap-1 flex-1 min-w-[130px]">
          <label for="rec-cat" class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wider">Category</label>
          <select
            id="rec-cat" bind:value={form.category}
            class="bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100
                   focus:outline-none focus:border-indigo-500 transition-colors"
          >
            <option value="">— select —</option>
            {#each $splits as s}
              <option value={s.category}>{s.category}</option>
            {/each}
          </select>
        </div>
        <div class="flex flex-col gap-1 max-w-[90px]">
          <label for="rec-day" class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wider">Day</label>
          <input
            id="rec-day" type="number" min="1" max="31"
            bind:value={form.day_of_month}
            class="bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100
                   focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 transition-colors"
          />
        </div>
      </div>
      <button
        type="submit" disabled={saving}
        class="self-start bg-gradient-to-r from-indigo-600 to-violet-600
               hover:from-indigo-500 hover:to-violet-500
               text-white text-sm font-semibold rounded-lg px-5 py-2
               disabled:opacity-50 disabled:cursor-not-allowed
               transition-all duration-150 shadow-md shadow-indigo-900/30 active:scale-[0.98]"
      >
        {saving ? 'Saving…' : '+ Add Recurring'}
      </button>
    </form>
  </div>
</div>
