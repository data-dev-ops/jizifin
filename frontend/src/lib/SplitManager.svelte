<script>
  /**
   * SplitManager.svelte — categories and SCD2 split agreements. The effective split
   * per category, salary ratios and even/salary proposals come from the Auditor;
   * the server validates every agreement (shares total 100).
   */

  import { createEventDispatcher } from 'svelte';
  import { audit, agreementApi, categoryApi } from './api.js';
  import SplitEditor, { toShares, fromShares } from './SplitEditor.svelte';
  import { fmtDate, fmtPct, monthBounds } from './format.js';
  import { selectedMonth, ledgerVersion, activeMembers, memberById, categories, expenseCategories, incomeCategories, accounts } from './stores.js';

  const dispatch = createEventDispatcher();

  const PAY_MODES = [['shared', 'Shared'], ['personal', 'Personal (payer bears all)'], ['income_ratio', 'Income ratio']];
  const SOURCE_LABEL = { personal: 'Personal', income_ratio: 'Income ratio', agreement: 'Agreement', equal: 'Equal split', override: 'Override' };

  let section = 'agreements';
  let overview = [];
  let ratios = {};
  let loadError = '';

  $: load($selectedMonth, $ledgerVersion);

  async function load(month) {
    loadError = '';
    try {
      const [splits, rat] = await Promise.all([audit.splits(month), audit.salaryRatios(month)]);
      overview = splits.value.categories;
      ratios = rat.value.pct;
      if (expanded != null) timeline = await agreementApi.list(expanded);
    } catch (err) {
      loadError = err.message;
    }
  }

  $: splitOf = new Map(overview.map((s) => [s.category, s]));
  $: jointCategoryIds = new Set($accounts.flatMap((a) => a.categories.map((c) => c.category)));
  $: sharedCategories = $expenseCategories.filter((c) => c.pay_mode !== 'personal' && !jointCategoryIds.has(c.id));
  const memberLabel = (id) => $memberById.get(Number(id))?.name ?? `#${id}`;
  const memberColor = (id) => $memberById.get(Number(id))?.color ?? '#6366f1';

  // ── Categories ─────────────────────────────────────────────────────────────
  let newName = '';
  let newKind = 'expense';
  let categoryError = '';
  let renaming = null;
  let confirmDelete = null;

  async function addCategory() {
    categoryError = '';
    try {
      await categoryApi.create({ kind: newKind, name: newName.trim().toUpperCase(), pay_mode: 'shared' });
      newName = '';
    } catch (err) {
      categoryError = err.message;
    }
  }

  async function saveCategory(c, patch) {
    categoryError = '';
    try {
      await categoryApi.update(c.id, { kind: c.kind, name: c.name, pay_mode: c.pay_mode, ...patch });
      renaming = null;
    } catch (err) {
      categoryError = err.message;
    }
  }

  async function deleteCategory(id) {
    categoryError = '';
    try {
      await categoryApi.remove(id);
      confirmDelete = null;
    } catch (err) {
      categoryError = err.message;
    }
  }

  // ── Agreements timeline & overrides ────────────────────────────────────────
  let expanded = null;
  let timeline = [];
  let editor = null; // { id?, category, valid_from, valid_to, note, shares }
  let editorError = '';

  async function toggleTimeline(category) {
    expanded = expanded === category ? null : category;
    timeline = expanded == null ? [] : await agreementApi.list(category);
  }

  function openEditor(category, agreement = null) {
    editorError = '';
    const { from, to } = monthBounds($selectedMonth);
    editor = agreement
      ? { ...agreement, valid_to: agreement.valid_to ?? '', note: agreement.note ?? '', shares: fromShares(agreement.shares) }
      : { category, valid_from: from, valid_to: to, note: '', shares: fromShares(Object.entries(splitOf.get(category)?.pct ?? {}).map(([member, pct]) => ({ member: Number(member), pct }))) };
  }

  async function saveEditor() {
    editorError = '';
    const input = { valid_from: editor.valid_from, valid_to: editor.valid_to || null, note: editor.note, shares: toShares(editor.shares) };
    try {
      if (editor.id) await agreementApi.update(editor.id, input);
      else await agreementApi.create(editor.category, input);
      editor = null;
    } catch (err) {
      editorError = err.message;
    }
  }

  async function deleteAgreement(id) {
    await agreementApi.remove(id).catch((err) => { loadError = err.message; });
  }

  // ── Matrix (baseline per category) ─────────────────────────────────────────
  let baseline = {};
  let rowMessage = {};

  $: for (const c of sharedCategories) if (!baseline[c.id]) baseline[c.id] = fromShares(Object.entries(splitOf.get(c.id)?.pct ?? {}).map(([member, pct]) => ({ member: Number(member), pct })));

  async function propose(categoryId, strategy, members = []) {
    const { value } = await audit.splitProposal(strategy, members, $selectedMonth);
    baseline[categoryId] = value.pct;
  }

  function singlePayer(categoryId, member) {
    baseline[categoryId] = Object.fromEntries($activeMembers.map((m) => [m.id, m.id === member ? 100 : 0]));
  }

  async function saveBaseline(categoryId) {
    rowMessage[categoryId] = '';
    try {
      await agreementApi.setBaseline(categoryId, toShares(baseline[categoryId]));
      rowMessage[categoryId] = '✓ Saved';
    } catch (err) {
      rowMessage[categoryId] = err.message;
    }
  }

  // ── Batch ──────────────────────────────────────────────────────────────────
  let batchCategories = [];
  let batchMembers = [];
  let batchStrategy = 'even';
  let batchPayer = null;
  let batchMessage = '';

  $: if (!batchMembers.length && $activeMembers.length) batchMembers = $activeMembers.map((m) => m.id);
  const toggle = (list, id) => (list.includes(id) ? list.filter((x) => x !== id) : [...list, id]);

  async function applyBatch() {
    batchMessage = '';
    try {
      const shares = batchStrategy === 'single'
        ? [{ member: batchPayer, pct: 100 }]
        : Object.entries((await audit.splitProposal(batchStrategy, batchMembers, $selectedMonth)).value.pct).map(([member, pct]) => ({ member: Number(member), pct }));
      for (const id of batchCategories) await agreementApi.setBaseline(id, shares);
      batchMessage = `✓ Updated ${batchCategories.length} category baseline(s).`;
    } catch (err) {
      batchMessage = err.message;
    }
  }
</script>

<div class="space-y-6">
  <div>
    <h1 class="page-title flex items-center gap-2.5"><span>⚖️</span> Categories & Split Agreements</h1>
    <p class="page-subtitle">Splits as the Auditor applies them; edit baselines and date-bounded overrides.</p>
  </div>

  <nav class="flex flex-wrap gap-2 border-b border-neutral-200 dark:border-neutral-800 pb-3" aria-label="Categories sub-navigation">
    {#each [['agreements', '⚖️ Split Agreements'], ['matrix', '📊 Split Matrix'], ['batch', '⚡ Batch Rule Builder'], ['categories', '🏷️ Categories']] as [id, label]}
      <button id="split-nav-{id}" type="button" on:click={() => (section = id)}
        class="px-4 py-2 rounded-xl text-xs font-semibold transition-all cursor-pointer {section === id ? 'bg-indigo-600 text-white shadow-sm' : 'bg-neutral-100 dark:bg-neutral-900 text-neutral-600 dark:text-neutral-400 border border-neutral-200 dark:border-neutral-800'}">
        {label}
      </button>
    {/each}
  </nav>

  {#if loadError}<p class="text-xs text-rose-600 dark:text-red-400">{loadError}</p>{/if}

  <!-- Salary ratio banner (Auditor) -->
  <div class="card p-4 sm:p-5 flex flex-col sm:flex-row sm:items-center justify-between gap-3">
    <div class="flex flex-wrap gap-2 items-center">
      <span class="text-xs font-semibold text-neutral-700 dark:text-neutral-300 uppercase tracking-wider mr-2">Income ratio {$selectedMonth}:</span>
      {#each Object.entries(ratios) as [m, pct]}
        <span class="text-xs font-bold px-2.5 py-1 rounded-xl bg-neutral-50 dark:bg-neutral-950/80 border border-neutral-200 dark:border-neutral-800" style="color: {memberColor(m)}">{memberLabel(m)} {fmtPct(pct)}</span>
      {/each}
    </div>
    <button id="link-manage-income" type="button" on:click={() => dispatch('navigateIncome')} class="px-3.5 py-1.5 rounded-xl text-xs font-semibold bg-indigo-50 dark:bg-indigo-950/80 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-700/60 self-start sm:self-auto">Manage in Income Tab →</button>
  </div>

  {#if section === 'agreements'}
    <div class="space-y-3">
      {#each $expenseCategories as c (c.id)}
        {@const split = splitOf.get(c.id)}
        <div class="card p-4 space-y-3">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div class="flex items-center gap-2">
              <span class="text-sm font-bold text-neutral-900 dark:text-white">{c.name}</span>
              <span class="badge-neutral text-[10px]">{SOURCE_LABEL[split?.source] ?? '—'}</span>
              {#if split?.is_override}<span class="badge-amber text-[10px]">Override active</span>{/if}
              {#if jointCategoryIds.has(c.id)}<span class="badge-indigo text-[10px]">🏦 Joint</span>{/if}
            </div>
            <div class="flex items-center gap-2">
              <button id="timeline-{c.id}" type="button" on:click={() => toggleTimeline(c.id)} class="btn-ghost text-xs">{expanded === c.id ? 'Hide timeline' : 'Timeline'}</button>
              {#if c.pay_mode !== 'personal'}
                <button id="add-override-{c.id}" type="button" on:click={() => openEditor(c.id)} class="btn-secondary text-xs">+ Override</button>
              {/if}
            </div>
          </div>
          {#if split && Object.keys(split.pct).length}
            <div class="flex h-5 rounded-lg overflow-hidden bg-neutral-100 dark:bg-neutral-800/40">
              {#each Object.entries(split.pct) as [m, pct]}
                <div class="h-full flex items-center justify-center text-[10px] font-bold text-white/90" style="width: {pct}%; background-color: {memberColor(m)}" title="{memberLabel(m)} {fmtPct(pct, 2)}">
                  {#if pct >= 12}{memberLabel(m)} {fmtPct(pct)}{/if}
                </div>
              {/each}
            </div>
          {:else if split?.source === 'personal'}
            <p class="text-xs text-neutral-500">Each payer bears their own expenses in this category.</p>
          {/if}
          {#if expanded === c.id}
            <div class="space-y-2 pt-2 border-t border-neutral-200 dark:border-neutral-800">
              {#each timeline as a (a.id)}
                <div class="card-sub p-2.5 flex flex-wrap items-center justify-between gap-2 text-xs">
                  <span class="font-mono text-neutral-600 dark:text-neutral-400">{fmtDate(a.valid_from)} → {a.valid_to ? fmtDate(a.valid_to) : 'open-ended'}</span>
                  <span class="flex-1 truncate text-neutral-700 dark:text-neutral-300">{a.shares.map((s) => `${memberLabel(s.member)} ${fmtPct(s.pct, 2)}`).join(' / ')}{a.note ? ` · ${a.note}` : ''}</span>
                  <button type="button" on:click={() => openEditor(c.id, a)} class="text-indigo-600 dark:text-indigo-400">Edit</button>
                  <button type="button" on:click={() => deleteAgreement(a.id)} class="text-rose-600 dark:text-red-400">Delete</button>
                </div>
              {:else}
                <p class="text-xs text-neutral-500">No agreements; the equal split applies.</p>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  {#if section === 'matrix'}
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
      {#each sharedCategories as c (c.id)}
        <div class="card p-4 space-y-3">
          <div class="flex items-center justify-between">
            <span class="text-sm font-bold text-neutral-900 dark:text-white">{c.name}</span>
            <div class="flex gap-1.5 text-[11px]">
              <button type="button" on:click={() => propose(c.id, 'even')} class="btn-ghost text-[11px]">Even</button>
              <button type="button" on:click={() => propose(c.id, 'salary')} class="btn-ghost text-[11px]">Salary</button>
              {#each $activeMembers as m (m.id)}
                <button type="button" on:click={() => singlePayer(c.id, m.id)} class="btn-ghost text-[11px]" style="color: {m.color}">{m.name} 100%</button>
              {/each}
            </div>
          </div>
          {#if baseline[c.id]}
            <SplitEditor members={$activeMembers} bind:shares={baseline[c.id]} idPrefix="matrix-{c.id}" />
          {/if}
          <div class="flex items-center justify-between">
            <span class="text-xs {rowMessage[c.id]?.startsWith('✓') ? 'text-emerald-600 dark:text-emerald-400' : 'text-rose-600 dark:text-red-400'}">{rowMessage[c.id] ?? ''}</span>
            <button id="save-baseline-{c.id}" type="button" on:click={() => saveBaseline(c.id)} class="btn-primary text-xs">Save baseline</button>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  {#if section === 'batch'}
    <div class="card p-5 space-y-4">
      <div>
        <p class="text-xs font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-2">Categories</p>
        <div class="flex flex-wrap gap-2">
          {#each sharedCategories as c (c.id)}
            <button type="button" on:click={() => (batchCategories = toggle(batchCategories, c.id))} class="px-2.5 py-1 rounded-lg text-xs font-semibold border {batchCategories.includes(c.id) ? 'bg-indigo-600 text-white border-indigo-500' : 'border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400'}">{c.name}</button>
          {/each}
        </div>
      </div>
      <div class="flex flex-wrap gap-4">
        <div>
          <p class="text-xs font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-2">Strategy</p>
          <select id="batch-strategy" bind:value={batchStrategy} class="select-field text-xs">
            <option value="even">Even split</option>
            <option value="salary">Salary ratio</option>
            <option value="single">Single payer</option>
          </select>
        </div>
        {#if batchStrategy === 'single'}
          <div>
            <p class="text-xs font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-2">Payer</p>
            <select id="batch-payer" bind:value={batchPayer} class="select-field text-xs">
              {#each $activeMembers as m (m.id)}<option value={m.id}>{m.name}</option>{/each}
            </select>
          </div>
        {:else}
          <div>
            <p class="text-xs font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-2">Members</p>
            <div class="flex flex-wrap gap-2">
              {#each $activeMembers as m (m.id)}
                <button type="button" on:click={() => (batchMembers = toggle(batchMembers, m.id))} class="px-2.5 py-1 rounded-lg text-xs font-semibold border {batchMembers.includes(m.id) ? 'text-white' : 'border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400'}" style={batchMembers.includes(m.id) ? `background-color:${m.color};border-color:${m.color}` : ''}>{m.name}</button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
      {#if batchMessage}<p class="text-xs {batchMessage.startsWith('✓') ? 'text-emerald-600 dark:text-emerald-400' : 'text-rose-600 dark:text-red-400'}">{batchMessage}</p>{/if}
      <button id="apply-batch" type="button" on:click={applyBatch} disabled={!batchCategories.length} class="btn-primary">Apply to {batchCategories.length} categories</button>
    </div>
  {/if}

  {#if section === 'categories'}
    <div class="card space-y-3">
      <div class="flex items-end gap-3 flex-wrap">
        <div class="flex-1 min-w-[200px]">
          <label for="new-category" class="block text-xs font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-1.5">New Category Name</label>
          <input id="new-category" type="text" bind:value={newName} class="input-field uppercase" on:keydown={(e) => e.key === 'Enter' && addCategory()} />
        </div>
        <div class="inline-flex rounded-xl bg-neutral-100 dark:bg-neutral-950 p-1 border border-neutral-200 dark:border-neutral-800 h-[42px] items-center">
          {#each ['expense', 'income'] as k}
            <button id="cat-type-{k}" type="button" on:click={() => (newKind = k)} class="px-3 py-1.5 rounded-lg text-xs font-semibold capitalize {newKind === k ? 'bg-indigo-600 text-white' : 'text-neutral-600 dark:text-neutral-400'}">{k}</button>
          {/each}
        </div>
        <button id="add-category-btn" type="button" on:click={addCategory} disabled={!newName.trim()} class="btn-primary h-[42px]">+ Add Category</button>
      </div>
      {#if categoryError}<p class="text-xs text-rose-700 dark:text-red-400 bg-rose-50 dark:bg-red-950/40 border border-rose-200 dark:border-red-800 rounded-xl px-3 py-2">{categoryError}</p>{/if}
    </div>

    {#each [['Expense categories', $expenseCategories], ['Income categories', $incomeCategories]] as [heading, list]}
      <div class="card space-y-2">
        <h3 class="text-xs font-bold uppercase tracking-wider text-neutral-700 dark:text-neutral-300">{heading} ({list.length})</h3>
        {#each list as c (c.id)}
          <div class="card-sub p-2.5 flex flex-wrap items-center gap-2 text-xs">
            {#if renaming?.id === c.id}
              <input type="text" bind:value={renaming.name} class="input-field py-1 text-xs flex-1 uppercase" />
              <button type="button" on:click={() => saveCategory(c, { name: renaming.name.trim().toUpperCase() })} class="btn-primary text-xs">Save</button>
              <button type="button" on:click={() => (renaming = null)} class="btn-ghost text-xs">Cancel</button>
            {:else}
              <span class="font-semibold text-neutral-900 dark:text-white flex-1">{c.name}</span>
              {#if c.kind === 'expense'}
                <select value={c.pay_mode} on:change={(e) => saveCategory(c, { pay_mode: e.target.value })} class="select-field text-xs py-1 w-auto">
                  {#each PAY_MODES as [mode, label]}<option value={mode}>{label}</option>{/each}
                </select>
              {/if}
              <button type="button" on:click={() => (renaming = { id: c.id, name: c.name })} class="text-indigo-600 dark:text-indigo-400">Rename</button>
              {#if confirmDelete === c.id}
                <button type="button" on:click={() => deleteCategory(c.id)} class="px-2 py-0.5 rounded bg-rose-600 text-white">Confirm</button>
                <button type="button" on:click={() => (confirmDelete = null)} class="btn-ghost text-xs">✕</button>
              {:else}
                <button type="button" on:click={() => (confirmDelete = c.id)} class="text-rose-600 dark:text-red-400">Delete</button>
              {/if}
            {/if}
          </div>
        {/each}
      </div>
    {/each}
  {/if}
</div>

{#if editor}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm" role="dialog" aria-modal="true">
    <form class="card max-w-lg w-full shadow-2xl space-y-3" on:submit|preventDefault={saveEditor}>
      <h3 class="text-base font-bold text-neutral-900 dark:text-white">{editor.id ? 'Edit agreement' : 'New override'} · {$categories.find((c) => c.id === editor.category)?.name}</h3>
      <div class="grid grid-cols-2 gap-3">
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">From <input id="agreement-from" type="date" bind:value={editor.valid_from} class="input-field mt-1" /></label>
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">To (empty = open-ended) <input id="agreement-to" type="date" bind:value={editor.valid_to} class="input-field mt-1" /></label>
      </div>
      <SplitEditor members={$activeMembers} bind:shares={editor.shares} idPrefix="agreement" />
      <input id="agreement-note" type="text" bind:value={editor.note} placeholder="Note (optional)" class="input-field" />
      {#if editorError}<p class="text-xs text-rose-600 dark:text-red-400">{editorError}</p>{/if}
      <div class="flex gap-2">
        <button type="button" on:click={() => (editor = null)} class="btn-secondary flex-1">Cancel</button>
        <button id="save-agreement" type="submit" class="btn-primary flex-1">Save</button>
      </div>
    </form>
  </div>
{/if}
