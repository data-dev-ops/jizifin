<script>
  /**
   * ProjectsTab.svelte — savings / milestone projects. Progress and forecasts come
   * from `/views/projects`; the participant balance sheet from the Auditor.
   */

  import { audit, projectApi, views } from './api.js';
  import { money, fmtDate, fmtPct, toCents, centsInput } from './format.js';
  import { ledgerVersion, projects, activeMembers, memberById, accounts, currencySymbol, showProjectsInExpense, projectDisplayFilter, privacyShield } from './stores.js';

  let progress = new Map();
  let error = '';

  $: load($ledgerVersion);

  async function load() {
    try {
      progress = new Map((await views.projects()).map((p) => [p.project, p]));
      error = '';
    } catch (err) {
      error = err.message;
    }
  }

  function visible(p) {
    const status = progress.get(p.id)?.completion.status;
    const started = (progress.get(p.id)?.count ?? 0) > 0;
    if ($projectDisplayFilter === 'active') return status !== 'completed';
    if ($projectDisplayFilter === 'in_progress') return started && status !== 'completed';
    return true;
  }

  $: shown = $projects.filter(visible);
  const memberLabel = (id) => $memberById.get(Number(id))?.name ?? `#${id}`;
  const memberColor = (id) => $memberById.get(Number(id))?.color ?? '#6366f1';

  function forecast(c) {
    if (!c) return '';
    if (c.status === 'completed') return '✓ Completed';
    if (c.status === 'indefinite') return 'No payments yet';
    return `~${c.months} mo left (${fmtDate(c.date)})`;
  }

  // ── Form (create / edit) ───────────────────────────────────────────────────
  const blank = () => ({ id: null, name: '', color: '#6366f1', target: '', target_date: '', members: [] });
  let form = blank();
  let formError = '';

  function edit(p) {
    form = { ...p, target: centsInput(p.target), target_date: p.target_date ?? '', members: [...p.members] };
    formError = '';
  }

  const toggleMember = (id) => (form.members = form.members.includes(id) ? form.members.filter((m) => m !== id) : [...form.members, id]);

  function fundFromAccount(account) {
    form.members = account.members.map((m) => m.member);
  }

  async function save() {
    formError = '';
    const payload = { name: form.name, color: form.color, target: toCents(form.target) ?? 0, target_date: form.target_date || null, members: form.members };
    try {
      if (form.id) await projectApi.update(form.id, payload);
      else await projectApi.create(payload);
      form = blank();
    } catch (err) {
      formError = err.message;
    }
  }

  let confirmDeleteId = null;

  async function remove(id) {
    try {
      await projectApi.remove(id);
      confirmDeleteId = null;
    } catch (err) {
      error = err.message;
    }
  }

  // ── Balance sheet (Auditor) ────────────────────────────────────────────────
  let sheets = {};

  async function toggleSheet(id) {
    if (sheets[id]) {
      sheets = { ...sheets, [id]: null };
      return;
    }
    try {
      sheets = { ...sheets, [id]: (await audit.projectSettlement(id)).value };
    } catch (err) {
      error = err.message;
    }
  }
</script>

<div class="space-y-6">
  <header class="page-header">
    <div>
      <h1 class="page-title flex items-center gap-2.5"><span>🚀</span> Projects & Savings Goals</h1>
      <p class="page-subtitle">Milestone budgets with progress, completion forecasts and participant balance sheets.</p>
    </div>
    <div class="flex items-center gap-2 text-xs">
      <select bind:value={$projectDisplayFilter} class="select-field text-xs py-1.5 w-auto">
        <option value="active">Active</option>
        <option value="in_progress">In progress</option>
        <option value="all">All</option>
      </select>
      <label class="flex items-center gap-1.5 text-neutral-600 dark:text-neutral-400"><input type="checkbox" bind:checked={$showProjectsInExpense} class="accent-indigo-600" /> Show in expense form</label>
    </div>
  </header>

  {#if error}<p class="text-xs text-rose-600 dark:text-red-400">{error}</p>{/if}

  <form class="card p-5 space-y-3" on:submit|preventDefault={save} id="project-form">
    <h3 class="text-sm font-bold text-neutral-800 dark:text-neutral-200">{form.id ? 'Edit project' : 'New project'}</h3>
    <div class="grid grid-cols-1 sm:grid-cols-4 gap-3">
      <label class="block text-xs text-neutral-600 dark:text-neutral-400 sm:col-span-2">Name <input id="project-name" type="text" bind:value={form.name} class="input-field mt-1" /></label>
      <label class="block text-xs text-neutral-600 dark:text-neutral-400">Target ({$currencySymbol}) <input id="project-target" type="text" inputmode="decimal" bind:value={form.target} class="input-field mt-1 tabular-nums" /></label>
      <label class="block text-xs text-neutral-600 dark:text-neutral-400">Target date <input id="project-date" type="date" bind:value={form.target_date} class="input-field mt-1" /></label>
    </div>
    <div class="flex flex-wrap items-center gap-2">
      <span class="text-xs text-neutral-600 dark:text-neutral-400">Members:</span>
      {#each $activeMembers as m (m.id)}
        <button type="button" on:click={() => toggleMember(m.id)} class="px-2.5 py-1 rounded-lg text-xs font-semibold border {form.members.includes(m.id) ? 'text-white' : 'border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400'}" style={form.members.includes(m.id) ? `background-color:${m.color};border-color:${m.color}` : ''}>{m.name}</button>
      {/each}
      {#each $accounts as a (a.id)}
        <button type="button" on:click={() => fundFromAccount(a)} class="btn-ghost text-xs">🏦 {a.name} members</button>
      {/each}
      <label class="flex items-center gap-1.5 text-xs text-neutral-600 dark:text-neutral-400 ml-auto">Colour <input type="color" bind:value={form.color} class="w-8 h-6 rounded" /></label>
    </div>
    {#if formError}<p class="text-xs text-rose-600 dark:text-red-400">{formError}</p>{/if}
    <div class="flex gap-2">
      {#if form.id}<button type="button" on:click={() => (form = blank())} class="btn-secondary">Cancel</button>{/if}
      <button id="project-save" type="submit" class="btn-primary">{form.id ? 'Save project' : '+ Create project'}</button>
    </div>
  </form>

  <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
    {#each shown as p (p.id)}
      {@const pr = progress.get(p.id)}
      {@const done = pr?.completion.status === 'completed'}
      <div class="card p-5 space-y-3" style="border-color: {p.color}40">
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0">
            <h3 class="text-sm font-bold text-neutral-900 dark:text-white truncate">{p.name}</h3>
            <p class="text-[11px] text-neutral-500">{p.target_date ? `Target ${fmtDate(p.target_date)}` : 'No target date'} · {p.members.map(memberLabel).join(', ') || 'Whole household'}</p>
          </div>
          <span class="text-xs font-bold tabular-nums {done ? 'text-emerald-600 dark:text-emerald-400' : 'text-indigo-600 dark:text-indigo-400'}">{fmtPct(pr?.progress_pct)}</span>
        </div>
        <div class="w-full h-2 bg-neutral-200 dark:bg-neutral-800 rounded-full overflow-hidden">
          <div class="h-full rounded-full" style="width: {pr?.progress_pct ?? 0}%; background-color: {done ? '#10b981' : p.color}"></div>
        </div>
        <div class="grid grid-cols-3 gap-2 text-xs">
          <div><p class="text-neutral-500">Spent</p><p class="font-bold tabular-nums text-neutral-900 dark:text-white"><span class:privacy-masked={$privacyShield}>{$money(pr?.spent ?? 0)}</span></p></div>
          <div><p class="text-neutral-500">Target</p><p class="font-bold tabular-nums text-neutral-900 dark:text-white">{$money(p.target)}</p></div>
          <div><p class="text-neutral-500">Avg / month</p><p class="font-bold tabular-nums text-neutral-900 dark:text-white">{$money(pr?.monthly_avg ?? 0)}</p></div>
        </div>
        <p class="text-[11px] text-neutral-500">{forecast(pr?.completion)}{pr?.last_payment ? ` · last payment ${fmtDate(pr.last_payment)}` : ''}</p>

        <div class="flex items-center justify-between border-t border-neutral-200 dark:border-neutral-800 pt-3 text-xs">
          <button id="toggle-settlement-{p.id}" type="button" on:click={() => toggleSheet(p.id)} class="text-indigo-600 dark:text-indigo-400 font-semibold">{sheets[p.id] ? 'Hide balance sheet' : 'View balance sheet'}</button>
          <div class="flex gap-2">
            <button type="button" on:click={() => edit(p)} class="text-neutral-600 dark:text-neutral-300">Edit</button>
            {#if confirmDeleteId === p.id}
              <button type="button" on:click={() => remove(p.id)} class="px-2 py-0.5 rounded bg-rose-600 text-white">Confirm</button>
              <button type="button" on:click={() => (confirmDeleteId = null)} class="text-neutral-500">✕</button>
            {:else}
              <button type="button" on:click={() => (confirmDeleteId = p.id)} class="text-rose-600 dark:text-red-400">Delete</button>
            {/if}
          </div>
        </div>

        {#if sheets[p.id]}
          {@const sheet = sheets[p.id]}
          <div class="space-y-2">
            {#each sheet.participants as row (row.member)}
              <div class="card-sub p-2.5 grid grid-cols-4 gap-2 text-xs items-center border-l-4" style="border-left-color: {memberColor(row.member)}">
                <span class="font-semibold text-neutral-900 dark:text-white">{memberLabel(row.member)}</span>
                <span class="tabular-nums text-neutral-600 dark:text-neutral-400">Funded {$money(row.funding)}</span>
                <span class="tabular-nums text-neutral-600 dark:text-neutral-400">Liable {$money(row.liability)}</span>
                <span class="tabular-nums font-bold text-right {row.net > 0 ? 'text-emerald-600 dark:text-emerald-400' : row.net < 0 ? 'text-rose-600 dark:text-red-400' : 'text-neutral-500'}">{row.net > 0 ? '+' : ''}{$money(row.net)}</span>
              </div>
            {/each}
            {#each sheet.debts as d (`${d.from}-${d.to}`)}
              <p class="text-xs text-neutral-600 dark:text-neutral-400">{memberLabel(d.from)} → {memberLabel(d.to)}: <strong class="tabular-nums">{$money(d.amount)}</strong></p>
            {/each}
          </div>
        {/if}
      </div>
    {:else}
      <div class="card empty-state-box py-10 lg:col-span-2"><p class="text-sm text-neutral-500">No projects to show.</p></div>
    {/each}
  </div>
</div>
