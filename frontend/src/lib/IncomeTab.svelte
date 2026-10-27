<script>
  /**
   * IncomeTab.svelte — employment streams, month-specific salary overrides and
   * one-off income. Effective income, ratios and totals come from the Auditor.
   */

  import { audit, entryApi, jobApi, incomeOverrideApi } from './api.js';
  import { money, fmtDate, fmtPct, toCents, centsInput, today, monthBounds } from './format.js';
  import { selectedMonth, ledgerVersion, activeMembers, memberById, incomeCategories, categoryName, jobs, currencySymbol, privacyShield } from './stores.js';

  const FREQUENCIES = ['monthly', 'weekly', 'biweekly', 'annual'];
  const LEAVE = /leave|sick|sabbatical|parental/i;

  let income = {};
  let ratios = {};
  let householdTotal = 0;
  let overrides = [];
  let ledger = [];
  let loadError = '';

  $: load($selectedMonth, $ledgerVersion);

  async function load(month) {
    loadError = '';
    try {
      const [inc, rat, cash, ovr, page] = await Promise.all([
        audit.income(month), audit.salaryRatios(month), audit.cashFlow(month), incomeOverrideApi.list(month),
        entryApi.page({ kind: 'income', ...monthBounds(month) }),
      ]);
      [income, ratios, householdTotal, overrides, ledger] = [inc.value, rat.value.pct, cash.value.income, ovr, page.entries];
    } catch (err) {
      loadError = err.message;
    }
  }

  const memberColor = (id) => $memberById.get(id)?.color ?? '#6366f1';
  const memberLabel = (id) => $memberById.get(id)?.name ?? `#${id}`;
  const initial = (id) => memberLabel(id).charAt(0).toUpperCase();

  function jobStatus(job) {
    const now = today();
    if (job.valid_from > now) return ['Upcoming', 'bg-sky-50 dark:bg-sky-950/80 text-sky-700 dark:text-sky-300 border-sky-200 dark:border-sky-800'];
    if (job.valid_to && job.valid_to < monthBounds($selectedMonth).from) return ['Ended', 'bg-neutral-100 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-400 border-neutral-200 dark:border-neutral-700'];
    if (job.notes && LEAVE.test(job.notes)) return ['On Leave', 'bg-amber-50 dark:bg-amber-950/80 text-amber-700 dark:text-amber-300 border-amber-200 dark:border-amber-800'];
    return ['Active', 'bg-emerald-50 dark:bg-emerald-950/80 text-emerald-700 dark:text-emerald-300 border-emerald-200 dark:border-emerald-800'];
  }

  // ── Salary override ────────────────────────────────────────────────────────
  let overrideFor = null;
  let overrideAmount = '';
  let overrideNote = '';
  let overrideError = '';

  function openOverride(member) {
    const existing = overrides.find((o) => o.member === member);
    overrideFor = member;
    overrideAmount = centsInput(income[member]?.base ?? 0);
    overrideNote = existing?.note ?? '';
    overrideError = '';
  }

  async function saveOverride() {
    const amount = toCents(overrideAmount);
    if (amount == null || amount < 0) {
      overrideError = 'Please enter a valid salary amount (0 or greater).';
      return;
    }
    try {
      await incomeOverrideApi.upsert({ member: overrideFor, month: $selectedMonth, amount, note: overrideNote });
      overrideFor = null;
    } catch (err) {
      overrideError = err.message;
    }
  }

  async function resetOverride() {
    try {
      await incomeOverrideApi.remove(overrideFor, $selectedMonth);
      overrideFor = null;
    } catch (err) {
      overrideError = err.message;
    }
  }

  // ── Jobs ───────────────────────────────────────────────────────────────────
  let jobModal = null; // 'add' | 'edit' | 'adjust'
  let jobForm = {};
  let jobError = '';
  let confirmJobDeleteId = null;

  function openJob(mode, job = null) {
    jobError = '';
    jobModal = mode;
    const start = monthBounds($selectedMonth).from;
    if (mode === 'add') jobForm = { member: $activeMembers[0]?.id, name: '', amount: '', frequency: 'monthly', valid_from: start, valid_to: '', notes: '' };
    if (mode === 'edit') jobForm = { ...job, amount: centsInput(job.amount), valid_to: job.valid_to ?? '', notes: job.notes ?? '' };
    if (mode === 'adjust') jobForm = { id: job.id, name: job.name, effective: start, amount: centsInput(job.amount), notes: 'Promotion / Rate Adjustment' };
  }

  function jobPayload() {
    return {
      member: jobForm.member, name: jobForm.name, amount: toCents(jobForm.amount), frequency: jobForm.frequency,
      valid_from: jobForm.valid_from, valid_to: jobForm.valid_to || null, notes: jobForm.notes,
    };
  }

  async function saveJob() {
    jobError = '';
    try {
      if (jobModal === 'adjust') await jobApi.adjust(jobForm.id, { effective: jobForm.effective, amount: toCents(jobForm.amount), notes: jobForm.notes });
      else if (jobModal === 'edit') await jobApi.update(jobForm.id, jobPayload());
      else await jobApi.create(jobPayload());
      jobModal = null;
    } catch (err) {
      jobError = err.message;
    }
  }

  async function deleteJob(id) {
    if (confirmJobDeleteId !== id) {
      confirmJobDeleteId = id;
      return;
    }
    confirmJobDeleteId = null;
    await jobApi.remove(id).catch((err) => { loadError = err.message; });
  }

  // ── One-off income ─────────────────────────────────────────────────────────
  let form = { name: '', amount: '', member: null, category: null, date: today() };
  let formError = '';
  let formSuccess = false;
  let confirmId = null;

  $: if (!form.member && $activeMembers.length) form.member = $activeMembers[0].id;
  $: if (!form.category && $incomeCategories.length) form.category = $incomeCategories[0].id;

  async function submitIncome() {
    formError = '';
    try {
      await entryApi.create({ kind: 'income', name: form.name, amount: toCents(form.amount), member: form.member, category: form.category, date: form.date });
      form = { ...form, name: '', amount: '' };
      formSuccess = true;
      setTimeout(() => (formSuccess = false), 3000);
    } catch (err) {
      formError = err.message;
    }
  }

  async function deleteEntry(id) {
    if (confirmId !== id) {
      confirmId = id;
      return;
    }
    confirmId = null;
    await entryApi.remove(id).catch((err) => { loadError = err.message; });
  }
</script>

<header class="page-header">
  <div>
    <h1 class="page-title flex items-center gap-2.5"><span>💰</span> Income & Employment Streams</h1>
    <p class="page-subtitle">Salaries, rate changes and one-off income; effective income and split ratios are computed by the Auditor.</p>
  </div>
  <button id="btn-add-job-top" on:click={() => openJob('add')} class="btn-primary self-start sm:self-auto">+ Add Job / Income Stream</button>
</header>

{#if loadError}<p class="text-xs text-rose-600 dark:text-red-400 mb-4">{loadError}</p>{/if}

<!-- ── 1. Monthly summary (Auditor) ────────────────────────────────────────── -->
<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
  {#each Object.values(income) as u (u.member)}
    {@const c = memberColor(u.member)}
    <div class="card p-4 sm:p-5 relative overflow-hidden" style="border-color: {c}50">
      <div class="flex items-center justify-between gap-2 mb-3">
        <div class="flex items-center gap-2">
          <div class="w-6 h-6 rounded-full flex items-center justify-center text-[10px] font-bold text-white shrink-0" style="background-color: {c}">{initial(u.member)}</div>
          <span class="text-xs font-semibold text-neutral-800 dark:text-neutral-200">{memberLabel(u.member)}</span>
        </div>
        <span class="text-[10px] font-bold px-2 py-0.5 rounded-full" style="background-color: {c}15; color: {c}; border: 1px solid {c}40">{fmtPct(ratios[u.member])} ratio</span>
      </div>
      <p class="text-2xl font-bold tabular-nums" style="color: {c}"><span class:privacy-masked={$privacyShield}>{$money(u.total)}</span></p>
      <div class="mt-3 pt-3 border-t border-neutral-200 dark:border-neutral-800/80 flex items-center justify-between text-[11px] text-neutral-500 dark:text-neutral-400 flex-wrap gap-1">
        <span>Base: <span class="font-medium text-neutral-700 dark:text-neutral-300 tabular-nums"><span class:privacy-masked={$privacyShield}>{$money(u.base)}</span></span>
          {#if u.has_override}<span class="ml-1 px-1.5 rounded text-[10px] font-semibold bg-amber-50 dark:bg-amber-950/80 text-amber-700 dark:text-amber-300 border border-amber-200 dark:border-amber-800">⚡ Adjusted</span>{/if}
        </span>
        {#if u.one_off > 0}<span>Bonuses: <span class="font-medium text-emerald-600 dark:text-emerald-400 tabular-nums"><span class:privacy-masked={$privacyShield}>+{$money(u.one_off)}</span></span></span>{/if}
      </div>
      <div class="mt-2.5 pt-2 border-t border-dashed border-neutral-200 dark:border-neutral-800/60 flex items-center justify-between">
        <button id="btn-adjust-salary-{u.member}" type="button" on:click={() => openOverride(u.member)} class="text-[11px] font-medium text-indigo-600 dark:text-indigo-400 hover:text-indigo-700 dark:hover:text-indigo-300 cursor-pointer">
          ✏️ {u.has_override ? 'Edit Salary Override' : 'Adjust Monthly Salary'}
        </button>
        {#if u.has_override}
          <span class="text-[10px] text-neutral-400 dark:text-neutral-500 font-mono">Contract: <span class:privacy-masked={$privacyShield}>{$money(u.contract)}</span></span>
        {/if}
      </div>
    </div>
  {/each}
  <div class="card p-4 sm:p-5 border-indigo-200 dark:border-indigo-900/40 bg-indigo-50/50 dark:bg-gradient-to-br dark:from-neutral-900 dark:to-indigo-950/40">
    <div class="flex items-center justify-between gap-2 mb-3">
      <span class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">Household Total</span>
    </div>
    <p class="text-2xl font-bold tabular-nums text-neutral-900 dark:text-white"><span class:privacy-masked={$privacyShield}>{$money(householdTotal)}</span></p>
    <p class="text-[11px] text-neutral-500 mt-3 pt-3 border-t border-neutral-200 dark:border-neutral-800">Contracts or overrides, plus one-off income</p>
  </div>
</div>

<!-- ── 2. Jobs ─────────────────────────────────────────────────────────────── -->
<div class="card p-5 sm:p-6 mb-8">
  <div class="mb-6">
    <h2 class="text-base font-semibold text-neutral-900 dark:text-white">💼 Jobs & Employment Streams</h2>
    <p class="text-xs text-neutral-500 mt-0.5">Contracts with pay periods and start/end dates.</p>
  </div>
  {#if $jobs.length === 0}
    <div class="text-center py-12 px-4 border border-dashed border-neutral-200 dark:border-neutral-800 rounded-2xl bg-neutral-50/50 dark:bg-neutral-950/30">
      <p class="text-sm font-semibold text-neutral-700 dark:text-neutral-300">No employment streams configured yet</p>
      <button id="btn-add-first-job" on:click={() => openJob('add')} class="btn-primary mt-4">+ Add First Employment Stream</button>
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
      {#each $jobs as job (job.id)}
        {@const [statusLabel, statusClass] = jobStatus(job)}
        <div class="card-sub p-4 flex flex-col justify-between">
          <div>
            <div class="flex items-start justify-between gap-2 mb-2">
              <div class="flex items-center gap-2 min-w-0">
                <div class="w-5 h-5 rounded-full flex items-center justify-center text-[9px] font-bold text-white shrink-0" style="background-color: {memberColor(job.member)}">{initial(job.member)}</div>
                <span class="text-xs font-medium text-neutral-500 dark:text-neutral-400 truncate">{memberLabel(job.member)}</span>
              </div>
              <span class="inline-flex items-center px-2 py-0.5 rounded text-[10px] font-semibold border {statusClass}">{statusLabel}</span>
            </div>
            <h4 class="text-sm font-semibold text-neutral-900 dark:text-neutral-100 truncate mb-2">{job.name}</h4>
            <div class="flex items-baseline gap-1.5 mb-3">
              <span class="text-lg font-bold tabular-nums text-neutral-900 dark:text-white">{$money(job.amount)}</span>
              <span class="text-xs text-neutral-500 capitalize">/ {job.frequency}</span>
            </div>
            <div class="space-y-1 text-[11px] text-neutral-500 dark:text-neutral-400 border-t border-neutral-200 dark:border-neutral-800/80 pt-2.5 mb-4">
              <div class="flex justify-between"><span>Timeline:</span><span class="font-mono text-[10px] text-neutral-700 dark:text-neutral-300">{fmtDate(job.valid_from)} → {job.valid_to ? fmtDate(job.valid_to) : 'Ongoing'}</span></div>
              {#if job.notes}<div class="flex justify-between"><span>Note:</span><span class="truncate max-w-[170px] text-neutral-700 dark:text-neutral-300" title={job.notes}>{job.notes}</span></div>{/if}
            </div>
          </div>
          <div class="flex items-center justify-between gap-2 border-t border-neutral-200 dark:border-neutral-800/60 pt-3">
            <button on:click={() => openJob('adjust', job)} class="text-[11px] font-semibold text-indigo-600 dark:text-indigo-400 cursor-pointer">Adjust Rate / Raise →</button>
            <div class="flex items-center gap-1">
              <button on:click={() => openJob('edit', job)} title="Edit job" class="p-1.5 rounded-lg text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 text-xs">✏️</button>
              <button on:click={() => deleteJob(job.id)} title="Delete job" class="p-1.5 rounded-lg text-xs {confirmJobDeleteId === job.id ? 'bg-rose-600 text-white' : 'text-neutral-400 hover:text-rose-600'}">{confirmJobDeleteId === job.id ? 'Confirm' : '🗑️'}</button>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- ── 3. One-off income ───────────────────────────────────────────────────── -->
<div class="grid grid-cols-1 lg:grid-cols-5 gap-6 mb-8">
  <form class="lg:col-span-2 card space-y-3" on:submit|preventDefault={submitIncome}>
    <h3 class="text-sm font-semibold text-neutral-800 dark:text-neutral-200">Log One-Off Income & Bonuses</h3>
    <div>
      <label for="income-name" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1">Description</label>
      <input id="income-name" type="text" bind:value={form.name} placeholder="e.g. Q1 Performance Bonus" class="input-field" />
    </div>
    <div class="grid grid-cols-2 gap-3">
      <div>
        <label for="income-amount" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1">Amount ({$currencySymbol})</label>
        <input id="income-amount" type="text" inputmode="decimal" bind:value={form.amount} placeholder="0.00" class="input-field tabular-nums" />
      </div>
      <div>
        <label for="income-who" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1">Person</label>
        <select id="income-who" bind:value={form.member} class="select-field">
          {#each $activeMembers as m (m.id)}<option value={m.id}>{m.name}</option>{/each}
        </select>
      </div>
    </div>
    <div class="grid grid-cols-2 gap-3">
      <div>
        <label for="income-category" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1">Category</label>
        <select id="income-category" bind:value={form.category} class="select-field">
          {#each $incomeCategories as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
        </select>
      </div>
      <div>
        <label for="income-date" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1">Date</label>
        <input id="income-date" type="date" bind:value={form.date} class="input-field" />
      </div>
    </div>
    {#if formError}<p class="text-xs text-rose-600 dark:text-red-400">{formError}</p>{/if}
    {#if formSuccess}<p class="text-xs text-emerald-600 dark:text-emerald-400">✓ Income logged.</p>{/if}
    <button id="btn-submit-income" type="submit" class="btn-primary w-full">Log Income</button>
  </form>

  <div class="lg:col-span-3 card">
    <h3 class="text-sm font-semibold text-neutral-800 dark:text-neutral-200 mb-4">Income Ledger · {$selectedMonth}</h3>
    {#if ledger.length === 0}
      <p class="text-xs text-neutral-500 py-8 text-center">No one-off income logged for this month.</p>
    {:else}
      <div class="space-y-2">
        {#each ledger as e (e.id)}
          <div class="card-sub p-3 flex items-center justify-between gap-3">
            <div class="min-w-0">
              <p class="text-xs font-semibold text-neutral-900 dark:text-white truncate">{e.name}</p>
              <p class="text-[11px] text-neutral-500">{fmtDate(e.date)} · <span style="color: {memberColor(e.member)}">{memberLabel(e.member)}</span> · {$categoryName(e.category)}</p>
            </div>
            <div class="flex items-center gap-2 flex-none">
              <span class="text-sm font-bold tabular-nums text-emerald-600 dark:text-emerald-400"><span class:privacy-masked={$privacyShield}>+{$money(e.amount)}</span></span>
              <button on:click={() => deleteEntry(e.id)} class="px-2 py-0.5 rounded text-[10px] {confirmId === e.id ? 'bg-rose-600 text-white' : 'text-neutral-400 hover:text-rose-600'}">{confirmId === e.id ? 'Confirm' : '🗑️'}</button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<!-- ── Salary override modal ───────────────────────────────────────────────── -->
{#if overrideFor != null}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm" role="dialog" aria-modal="true">
    <form class="card max-w-md w-full shadow-2xl space-y-4" on:submit|preventDefault={saveOverride}>
      <h3 class="text-base font-bold text-neutral-900 dark:text-white">{memberLabel(overrideFor)} · {$selectedMonth} salary</h3>
      <p class="text-xs text-neutral-500">Replaces the contract salary ({$money(income[overrideFor]?.contract ?? 0)}) for this month only.</p>
      <input id="salary-override-amount" type="text" inputmode="decimal" bind:value={overrideAmount} class="input-field tabular-nums" />
      <input id="salary-override-note" type="text" bind:value={overrideNote} placeholder="Reason (e.g. unpaid leave)" class="input-field" />
      {#if overrideError}<p class="text-xs text-rose-600 dark:text-red-400">{overrideError}</p>{/if}
      <div class="flex gap-2">
        {#if income[overrideFor]?.has_override}<button type="button" on:click={resetOverride} class="btn-secondary">Reset to contract</button>{/if}
        <button type="button" on:click={() => (overrideFor = null)} class="btn-secondary flex-1">Cancel</button>
        <button type="submit" class="btn-primary flex-1">Save</button>
      </div>
    </form>
  </div>
{/if}

<!-- ── Job modal ───────────────────────────────────────────────────────────── -->
{#if jobModal}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm" role="dialog" aria-modal="true">
    <form class="card max-w-md w-full shadow-2xl space-y-3" on:submit|preventDefault={saveJob}>
      <h3 class="text-base font-bold text-neutral-900 dark:text-white">
        {jobModal === 'adjust' ? `Adjust ${jobForm.name}` : jobModal === 'edit' ? 'Edit Employment Stream' : 'Add Employment Stream'}
      </h3>
      {#if jobModal === 'adjust'}
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">Effective from <input id="adjust-effective" type="date" bind:value={jobForm.effective} class="input-field mt-1" /></label>
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">New amount ({$currencySymbol}) <input id="adjust-amount" type="text" inputmode="decimal" bind:value={jobForm.amount} class="input-field mt-1" /></label>
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">Note <input id="adjust-notes" type="text" bind:value={jobForm.notes} class="input-field mt-1" /></label>
        <p class="text-[11px] text-neutral-500">The current stream ends the day before; a new one starts with the new rate.</p>
      {:else}
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">Job / employer <input id="job-name" type="text" bind:value={jobForm.name} class="input-field mt-1" /></label>
        <div class="grid grid-cols-2 gap-3">
          <label class="block text-xs text-neutral-600 dark:text-neutral-400">Member
            <select id="job-member" bind:value={jobForm.member} class="select-field mt-1">
              {#each $activeMembers as m (m.id)}<option value={m.id}>{m.name}</option>{/each}
            </select>
          </label>
          <label class="block text-xs text-neutral-600 dark:text-neutral-400">Frequency
            <select id="job-frequency" bind:value={jobForm.frequency} class="select-field mt-1">
              {#each FREQUENCIES as f}<option value={f}>{f}</option>{/each}
            </select>
          </label>
        </div>
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">Amount per period ({$currencySymbol}) <input id="job-amount" type="text" inputmode="decimal" bind:value={jobForm.amount} class="input-field mt-1" /></label>
        <div class="grid grid-cols-2 gap-3">
          <label class="block text-xs text-neutral-600 dark:text-neutral-400">Start <input id="job-start" type="date" bind:value={jobForm.valid_from} class="input-field mt-1" /></label>
          <label class="block text-xs text-neutral-600 dark:text-neutral-400">End <input id="job-end" type="date" bind:value={jobForm.valid_to} class="input-field mt-1" /></label>
        </div>
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">Notes <input id="job-notes" type="text" bind:value={jobForm.notes} class="input-field mt-1" /></label>
      {/if}
      {#if jobError}<p class="text-xs text-rose-600 dark:text-red-400">{jobError}</p>{/if}
      <div class="flex gap-2 pt-2">
        <button type="button" on:click={() => (jobModal = null)} class="btn-secondary flex-1">Cancel</button>
        <button id="btn-save-job" type="submit" class="btn-primary flex-1">Save</button>
      </div>
    </form>
  </div>
{/if}
