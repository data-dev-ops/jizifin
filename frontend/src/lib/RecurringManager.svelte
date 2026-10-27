<script>
  /**
   * RecurringManager.svelte — recurring templates. The month projection (occurrence
   * dates and totals) comes from `/views/recurring`; the backend scheduler books
   * each occurrence once.
   */

  import { recurringApi, views } from './api.js';
  import { money, fmtDate, fmtMonth, toCents, centsInput, today, shiftMonth } from './format.js';
  import { selectedMonth, ledgerVersion, recurring, activeMembers, memberById, expenseCategories, incomeCategories, categoryName, accounts, jointAccountEnabled, currencySymbol } from './stores.js';

  const RECURRENCES = [
    ['monthly', '🗓️ Monthly'], ['weekly', '⚡ Weekly'], ['biweekly', '⚡ Biweekly'],
    ['four_weekly', '🔄 4-Weekly'], ['quarterly', '📅 Quarterly'], ['annual', '🌟 Annual'],
  ];
  const LABEL = Object.fromEntries(RECURRENCES);

  let viewMonth = $selectedMonth;
  let projection = { total: 0, active: 0, due: [], by_category: [], by_member: [] };
  let error = '';

  $: load(viewMonth, $ledgerVersion);

  async function load(month) {
    try {
      projection = await views.recurring(month);
      error = '';
    } catch (err) {
      error = err.message;
    }
  }

  $: dueOf = new Map(projection.due.map((d) => [d.recurring, d]));

  // ── Form (create / edit) ───────────────────────────────────────────────────
  const blank = () => ({
    id: null, kind: 'expense', name: '', amount: '', member: $activeMembers[0]?.id ?? null, category: null,
    account: null, recurrence: 'monthly', day_of_month: 1, valid_from: today(), valid_to: '', is_active: true,
  });
  let form = blank();
  let formError = '';
  let confirmDeleteId = null;

  $: categoryOptions = form.kind === 'income' ? $incomeCategories : $expenseCategories;

  function edit(t) {
    form = { ...t, amount: centsInput(t.amount), valid_to: t.valid_to ?? '' };
    formError = '';
  }

  function payload(t) {
    return {
      kind: t.kind, name: t.name, amount: toCents(t.amount), member: t.member, category: t.category, account: t.account,
      recurrence: t.recurrence, day_of_month: Number(t.day_of_month), valid_from: t.valid_from, valid_to: t.valid_to || null, is_active: t.is_active,
    };
  }

  async function save() {
    formError = '';
    try {
      if (form.id) await recurringApi.update(form.id, payload(form));
      else await recurringApi.create(payload(form));
      form = blank();
    } catch (err) {
      formError = err.message;
    }
  }

  async function toggleActive(t) {
    await recurringApi.update(t.id, payload({ ...t, amount: centsInput(t.amount), is_active: !t.is_active })).catch((err) => { error = err.message; });
  }

  async function remove(id) {
    try {
      await recurringApi.remove(id);
      confirmDeleteId = null;
    } catch (err) {
      error = err.message;
    }
  }
</script>

<div class="space-y-6">
  <header class="page-header">
    <div>
      <h1 class="page-title flex items-center gap-2.5">
        <span>🔄</span> Recurring Commitments
        <span class="text-xs font-semibold px-2.5 py-0.5 rounded-full bg-neutral-100 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300">{projection.active} Active</span>
      </h1>
      <p class="page-subtitle">Subscriptions, fixed bills and income on a schedule.</p>
    </div>
    <div class="inline-flex items-center gap-2 bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 rounded-xl px-3 py-1.5 self-start sm:self-auto">
      <span class="text-xs text-neutral-500 dark:text-neutral-400">Viewing for:</span>
      <button type="button" on:click={() => (viewMonth = shiftMonth(viewMonth, -1))} class="w-6 h-6 rounded text-neutral-500" aria-label="Previous month">‹</button>
      <span class="text-xs font-bold text-indigo-600 dark:text-indigo-400 tabular-nums">{fmtMonth(viewMonth, true)}</span>
      <button type="button" on:click={() => (viewMonth = shiftMonth(viewMonth, 1))} class="w-6 h-6 rounded text-neutral-500" aria-label="Next month">›</button>
    </div>
  </header>

  {#if error}<p class="text-xs text-rose-600 dark:text-red-400">{error}</p>{/if}

  <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
    <div class="card p-4">
      <span class="text-xs font-medium text-neutral-500 dark:text-neutral-400">Total in {fmtMonth(viewMonth, true)}</span>
      <p class="text-2xl font-extrabold text-neutral-900 dark:text-neutral-100 tabular-nums mt-2">{$money(projection.total)}</p>
    </div>
    <div class="card p-4">
      <span class="text-xs font-medium text-neutral-500 dark:text-neutral-400">Active Schedules</span>
      <p class="text-2xl font-extrabold text-neutral-900 dark:text-neutral-100 tabular-nums mt-2">{projection.active} <span class="text-xs font-medium text-neutral-500">/ {$recurring.length} templates</span></p>
    </div>
    <div class="card p-4">
      <span class="text-xs font-medium text-neutral-500 dark:text-neutral-400">Categories Impacted</span>
      <p class="text-2xl font-extrabold text-neutral-900 dark:text-neutral-100 tabular-nums mt-2">{projection.by_category.length}</p>
    </div>
  </div>

  {#if projection.by_category.length > 0}
    <div class="flex flex-wrap gap-2">
      {#each projection.by_category as b (b.key)}
        <span class="px-3 py-1.5 rounded-xl text-xs bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800">
          <span class="font-semibold text-neutral-800 dark:text-neutral-200">{$categoryName(b.key)}</span>
          <span class="tabular-nums text-neutral-600 dark:text-neutral-400 ml-1">{$money(b.total)} ({b.count}×)</span>
        </span>
      {/each}
    </div>
  {/if}

  <div class="grid grid-cols-1 xl:grid-cols-5 gap-6">
    <form class="xl:col-span-2 card space-y-3" on:submit|preventDefault={save} id="recurring-form">
      <h3 class="text-sm font-bold text-neutral-800 dark:text-neutral-200">{form.id ? 'Edit template' : 'New recurring item'}</h3>
      <div class="inline-flex rounded-xl bg-neutral-100 dark:bg-neutral-950 p-1 border border-neutral-200 dark:border-neutral-800">
        {#each ['expense', 'income'] as k}
          <button type="button" on:click={() => { form.kind = k; form.category = null; }} class="px-3 py-1.5 rounded-lg text-xs font-semibold capitalize {form.kind === k ? 'bg-indigo-600 text-white' : 'text-neutral-600 dark:text-neutral-400'}">{k}</button>
        {/each}
      </div>
      <input id="recurring-name" name="recurring-name" type="text" bind:value={form.name} placeholder="e.g. Netflix, Rent" aria-label="Recurring item name" class="input-field" />
      <div class="grid grid-cols-2 gap-3">
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">Amount ({$currencySymbol}) <input id="recurring-amount" type="text" inputmode="decimal" bind:value={form.amount} class="input-field mt-1 tabular-nums" /></label>
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">{form.kind === 'income' ? 'Earned by' : 'Paid by'}
          <select id="recurring-member" bind:value={form.member} class="select-field mt-1">
            {#each $activeMembers as m (m.id)}<option value={m.id}>{m.name}</option>{/each}
          </select>
        </label>
      </div>
      <label class="block text-xs text-neutral-600 dark:text-neutral-400">Category
        <select id="recurring-category" bind:value={form.category} class="select-field mt-1">
          <option value={null}>— Select —</option>
          {#each categoryOptions as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
        </select>
      </label>
      <div class="grid grid-cols-2 gap-3">
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">Frequency
          <select id="recurring-frequency" bind:value={form.recurrence} class="select-field mt-1">
            {#each RECURRENCES as [value, label]}<option {value}>{label}</option>{/each}
          </select>
        </label>
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">Day of month <input id="recurring-day" type="number" min="1" max="31" bind:value={form.day_of_month} class="input-field mt-1" /></label>
      </div>
      <div class="grid grid-cols-2 gap-3">
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">Start <input id="recurring-start" type="date" bind:value={form.valid_from} class="input-field mt-1" /></label>
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">End <input id="recurring-end" type="date" bind:value={form.valid_to} class="input-field mt-1" /></label>
      </div>
      {#if $jointAccountEnabled && $accounts.length > 0 && form.kind === 'expense'}
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">Charged to
          <select id="recurring-account" bind:value={form.account} class="select-field mt-1">
            <option value={null}>Personal</option>
            {#each $accounts as a (a.id)}<option value={a.id}>🏦 {a.name}</option>{/each}
          </select>
        </label>
      {/if}
      {#if formError}<p class="text-xs text-rose-600 dark:text-red-400">{formError}</p>{/if}
      <div class="flex gap-2">
        {#if form.id}<button type="button" on:click={() => (form = blank())} class="btn-secondary">Cancel</button>{/if}
        <button id="recurring-save" type="submit" class="btn-primary flex-1">{form.id ? 'Save' : '+ Add recurring item'}</button>
      </div>
    </form>

    <div class="xl:col-span-3 space-y-2">
      {#each $recurring as t (t.id)}
        {@const due = dueOf.get(t.id)}
        <div class="card p-3.5 flex flex-wrap items-center gap-3 {t.is_active ? '' : 'opacity-60'}">
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2 flex-wrap">
              <span class="text-sm font-semibold text-neutral-900 dark:text-white truncate">{t.name}</span>
              <span class="badge-indigo text-[10px]">{LABEL[t.recurrence]}</span>
              {#if t.kind === 'income'}<span class="badge-emerald text-[10px]">Income</span>{/if}
              {#if t.account != null}<span class="badge-indigo text-[10px]">🏦 Joint</span>{/if}
            </div>
            <p class="text-[11px] text-neutral-500 mt-0.5">
              {$memberById.get(t.member)?.name} · {$categoryName(t.category)} · from {fmtDate(t.valid_from)}{t.valid_to ? ` to ${fmtDate(t.valid_to)}` : ''}
            </p>
            {#if due}<p class="text-[11px] text-indigo-600 dark:text-indigo-400 mt-0.5">Due {due.dates.map(fmtDate).join(', ')} · {$money(due.total)}</p>{/if}
          </div>
          <span class="text-sm font-bold tabular-nums text-neutral-900 dark:text-white">{$money(t.amount)}</span>
          <div class="flex gap-2 text-xs">
            <button type="button" on:click={() => toggleActive(t)} class="text-neutral-600 dark:text-neutral-300">{t.is_active ? 'Pause' : 'Resume'}</button>
            <button type="button" on:click={() => edit(t)} class="text-indigo-600 dark:text-indigo-400">Edit</button>
            {#if confirmDeleteId === t.id}
              <button type="button" on:click={() => remove(t.id)} class="px-2 py-0.5 rounded bg-rose-600 text-white">Confirm</button>
              <button type="button" on:click={() => (confirmDeleteId = null)} class="text-neutral-500">✕</button>
            {:else}
              <button type="button" on:click={() => (confirmDeleteId = t.id)} class="text-rose-600 dark:text-red-400">Delete</button>
            {/if}
          </div>
        </div>
      {:else}
        <div class="card empty-state-box py-10"><p class="text-xs text-neutral-500">No recurring items yet.</p></div>
      {/each}
    </div>
  </div>
</div>
