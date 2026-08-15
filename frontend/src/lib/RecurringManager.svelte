<script>
  import { recurringExpenses, splits, users, currencySymbol } from './stores.js';
  import { createRecurring, deleteRecurring } from './api.js';

  $: activeUsers = $users.filter((u) => u.is_active);

  let form = { name: '', cost_euros: '', who_paid: '', category: '', day_of_month: 1, is_joint: false };
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
        is_joint: form.is_joint,
      });
      form = { name: '', cost_euros: '', who_paid: activeUsers[0]?.name ?? '', category: '', day_of_month: 1, is_joint: false };
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
                {#if rec.is_joint}
                  <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-semibold bg-indigo-950/80 text-indigo-300 border border-indigo-800/60">
                    🏦 Joint Account
                  </span>
                {:else}
                  <span
                    class="inline-block px-2 py-0.5 rounded-full text-xs font-semibold"
                    style="background-color:{payerColor}22; color:{payerColor}"
                  >{rec.who_paid}</span>
                {/if}
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
  <div class="card-sub p-5">
    <h3 class="text-sm font-semibold text-neutral-200 mb-4">Add Recurring Expense</h3>
    {#if error}
      <div class="bg-red-950/40 border border-red-800 text-red-400 rounded-xl px-3.5 py-2 text-xs mb-4">{error}</div>
    {/if}
    <form on:submit|preventDefault={handleSubmit} class="space-y-4">
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div>
          <label for="rec-name" class="block text-xs font-medium text-neutral-400 mb-1.5">Expense Name</label>
          <input
            id="rec-name"
            class="input-field"
            placeholder="e.g. Netflix, Rent, Internet"
            bind:value={form.name}
          />
        </div>
        <div>
          <label for="rec-amount" class="block text-xs font-medium text-neutral-400 mb-1.5">Amount ({$currencySymbol})</label>
          <input
            id="rec-amount"
            class="input-field tabular-nums"
            type="number"
            min="0.01"
            step="0.01"
            placeholder="0.00"
            bind:value={form.cost_euros}
          />
        </div>
      </div>

      <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
        <div>
          <label for="rec-payer" class="block text-xs font-medium text-neutral-400 mb-1.5">Paid by</label>
          <select
            id="rec-payer"
            class="select-field"
            bind:value={form.who_paid}
            disabled={form.is_joint}
          >
            {#each activeUsers as u}
              <option value={u.name}>{u.name}</option>
            {/each}
          </select>
        </div>
        <div>
          <label for="rec-cat" class="block text-xs font-medium text-neutral-400 mb-1.5">Category</label>
          <select
            id="rec-cat"
            class="select-field"
            bind:value={form.category}
          >
            <option value="" disabled>Select category…</option>
            {#each $splits as s}
              <option value={s.category}>{s.category}</option>
            {/each}
          </select>
        </div>
        <div>
          <label for="rec-day" class="block text-xs font-medium text-neutral-400 mb-1.5">Day of Month</label>
          <input
            id="rec-day"
            type="number"
            min="1"
            max="31"
            class="input-field"
            bind:value={form.day_of_month}
          />
        </div>
      </div>

      <div class="flex flex-col gap-1.5">
        <span class="text-xs font-medium text-neutral-400">Payment Source</span>
        <div class="inline-flex rounded-xl bg-neutral-950 p-1 border border-neutral-800 h-[42px] items-center self-start">
          <button
            type="button"
            id="rec-source-personal"
            on:click={() => form.is_joint = false}
            class="px-4 py-1.5 rounded-lg text-xs font-semibold transition-colors cursor-pointer {!form.is_joint ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-400 hover:text-neutral-200'}"
          >
            Personal
          </button>
          <button
            type="button"
            id="rec-source-joint"
            on:click={() => form.is_joint = true}
            class="px-4 py-1.5 rounded-lg text-xs font-semibold transition-colors cursor-pointer {form.is_joint ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-400 hover:text-neutral-200'}"
          >
            🏦 Joint
          </button>
        </div>
      </div>

      <button
        type="submit"
        disabled={saving}
        class="btn-primary"
      >
        {saving ? 'Saving…' : '+ Add Recurring'}
      </button>
    </form>
  </div>
</div>
