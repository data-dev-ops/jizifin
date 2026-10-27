<script>
  /**
   * JointAccountTab.svelte — joint accounts: settings, coupled categories with expected
   * costs, deposits and corrections. Balance, target, top-up, status and proposed
   * deposits come from the Auditor; deposits and corrections are ledger entries.
   */

  import { accountApi, audit, entryApi } from './api.js';
  import { money, fmtDate, fmtPct, toCents, centsInput, today, monthBounds } from './format.js';
  import { selectedMonth, ledgerVersion, accounts, activeAccountId, activeMembers, memberById, expenseCategories, categoryName, currencySymbol } from './stores.js';

  const MODES = [['equal', 'Equal'], ['salary_ratio', 'Salary ratio'], ['fixed', 'Fixed amounts']];
  const ROUND_STEPS = [0, 500, 1000, 2500, 5000];

  let section = 'overview';
  let dash = null;
  let proposal = null;
  let deposits = [];
  let corrections = [];
  let roundTo = 0;
  let errorMsg = '';
  let successMsg = '';

  $: if ($activeAccountId == null && $accounts.length) activeAccountId.set($accounts[0].id);
  $: account = $accounts.find((a) => a.id === $activeAccountId);
  $: load(account?.id, $selectedMonth, roundTo, $ledgerVersion);

  async function load(id, month, step) {
    if (id == null) return;
    try {
      const bounds = monthBounds(month);
      const [d, p, dep, cor] = await Promise.all([
        audit.jointDashboard(id, month), audit.depositProposal(id, month, step || null),
        entryApi.page({ kind: 'deposit', account: id, ...bounds }), entryApi.page({ kind: 'correction', account: id }),
      ]);
      [dash, proposal, deposits, corrections] = [d.value, p.value, dep.entries, cor.entries];
    } catch (e) {
      errorMsg = e.message;
    }
  }

  function flash(msg) {
    successMsg = msg;
    errorMsg = '';
    setTimeout(() => (successMsg = ''), 3500);
  }

  const memberLabel = (id) => $memberById.get(Number(id))?.name ?? `#${id}`;
  const memberColor = (id) => $memberById.get(Number(id))?.color ?? '#6366f1';

  /** The account as an `AccountInput` with `patch` applied. */
  function input(a, patch = {}) {
    return {
      name: a.name, safety_margin_pct: a.safety_margin_pct, expected_override: a.expected_override,
      deposit_mode: a.deposit_mode, members: a.members, categories: a.categories, ...patch,
    };
  }

  async function saveAccount(patch, message) {
    try {
      await accountApi.update(account.id, input(account, patch));
      flash(message);
    } catch (e) {
      errorMsg = e.message;
    }
  }

  // ── Create / settings ──────────────────────────────────────────────────────
  let showCreate = false;
  let draft = null;

  function openSettings(a = null) {
    draft = a
      ? { name: a.name, margin: String(a.safety_margin_pct), mode: a.deposit_mode, members: a.members.map((m) => ({ ...m, fixed: centsInput(m.fixed_deposit) })) }
      : { name: '', margin: '10', mode: 'equal', members: $activeMembers.map((m) => ({ member: m.id, due_day: 1, fixed: '' })) };
    showCreate = !a;
    section = a ? 'settings' : section;
  }

  function draftInput() {
    const members = draft.members.map((m) => ({ member: m.member, due_day: Number(m.due_day) || 1, fixed_deposit: toCents(m.fixed) }));
    return { name: draft.name, safety_margin_pct: Number(draft.margin) || 0, deposit_mode: draft.mode, members };
  }

  function toggleDraftMember(id) {
    const has = draft.members.some((m) => m.member === id);
    draft.members = has ? draft.members.filter((m) => m.member !== id) : [...draft.members, { member: id, due_day: 1, fixed: '' }];
  }

  async function create() {
    try {
      const categories = $expenseCategories.filter((c) => c.pay_mode !== 'personal').map((c) => ({ category: c.id, expected: 0 }));
      const id = await accountApi.create({ ...draftInput(), expected_override: null, categories });
      activeAccountId.set(id);
      showCreate = false;
      flash('Joint account created with shared categories coupled.');
    } catch (e) {
      errorMsg = e.message;
    }
  }

  async function remove() {
    if (!confirm('Delete this joint account?')) return;
    try {
      await accountApi.remove(account.id);
      activeAccountId.set(null);
      flash('Joint account removed.');
    } catch (e) {
      errorMsg = e.message;
    }
  }

  // ── Categories & expected costs ────────────────────────────────────────────
  let expectedDraft = {};
  let grossMode = false;
  let grossText = '';

  function enterCategories() {
    expectedDraft = Object.fromEntries(account.categories.map((c) => [c.category, centsInput(c.expected)]));
    grossMode = account.expected_override != null;
    grossText = centsInput(account.expected_override);
    section = 'categories';
  }

  function toggleCategory(id) {
    if (id in expectedDraft) {
      const { [id]: _, ...rest } = expectedDraft;
      expectedDraft = rest;
    } else expectedDraft = { ...expectedDraft, [id]: '0.00' };
  }

  function saveCategories() {
    const categories = Object.entries(expectedDraft).map(([category, text]) => ({ category: Number(category), expected: toCents(text) ?? 0 }));
    saveAccount({ categories, expected_override: grossMode ? toCents(grossText) : null }, 'Categories and expected costs saved.');
  }

  // ── Deposits ───────────────────────────────────────────────────────────────
  $: depositedBy = new Map(deposits.map((d) => [d.member, d]));
  $: suggested = proposal?.rounded ?? proposal?.per_member ?? {};

  async function markPaid(member) {
    try {
      await entryApi.create({ kind: 'deposit', member, account: account.id, amount: suggested[member], date: today() });
      flash(`Deposit recorded for ${memberLabel(member)}.`);
    } catch (e) {
      errorMsg = e.message;
    }
  }

  async function unmark(entry) {
    await entryApi.remove(entry.id).catch((e) => { errorMsg = e.message; });
  }

  function lockProposalAsFixed() {
    const members = account.members.map((m) => ({ ...m, fixed_deposit: suggested[m.member] ?? m.fixed_deposit }));
    saveAccount({ deposit_mode: 'fixed', members }, 'Proposed amounts saved as fixed deposits.');
  }

  // ── Corrections ────────────────────────────────────────────────────────────
  let corr = { amount: '', date: today(), note: '', negative: false };

  async function addCorrection() {
    const cents = toCents(corr.amount);
    if (!cents) {
      errorMsg = 'Amount required.';
      return;
    }
    try {
      await entryApi.create({ kind: 'correction', account: account.id, amount: corr.negative ? -cents : cents, date: corr.date, note: corr.note });
      corr = { amount: '', date: today(), note: '', negative: false };
      flash('Correction logged.');
    } catch (e) {
      errorMsg = e.message;
    }
  }
</script>

<div class="space-y-6">
  <header class="page-header">
    <div>
      <h1 class="page-title flex items-center gap-2.5"><span>🏦</span> Joint Accounts</h1>
      <p class="page-subtitle">Shared accounts with coupled categories, deposit targets and corrections.</p>
    </div>
    <div class="flex items-center gap-2">
      {#if $accounts.length > 1}
        <select bind:value={$activeAccountId} class="select-field text-xs py-1.5 w-auto">
          {#each $accounts as a (a.id)}<option value={a.id}>🏦 {a.name}</option>{/each}
        </select>
      {/if}
      <button id="btn-create-joint" type="button" on:click={() => openSettings()} class="btn-primary text-xs">+ New account</button>
    </div>
  </header>

  {#if errorMsg}<div class="text-xs text-rose-700 dark:text-red-400 bg-rose-50 dark:bg-red-950/40 border border-rose-200 dark:border-red-800 rounded-xl px-3 py-2">{errorMsg}</div>{/if}
  {#if successMsg}<div class="text-xs text-emerald-700 dark:text-emerald-400 bg-emerald-50 dark:bg-emerald-950/40 border border-emerald-200 dark:border-emerald-800 rounded-xl px-3 py-2">{successMsg}</div>{/if}

  {#if !account}
    <div class="card empty-state-box py-16"><p class="text-sm text-neutral-500">No joint account yet. Create one to start tracking shared costs.</p></div>
  {:else}
    <nav class="flex flex-wrap gap-2 border-b border-neutral-200 dark:border-neutral-800 pb-3">
      {#each [['overview', '📊 Overview'], ['categories', '🏷️ Categories & Expected'], ['deposits', '💶 Deposits'], ['corrections', '✏️ Corrections'], ['settings', '⚙️ Settings']] as [id, label]}
        <button id="joint-nav-{id}" type="button"
          on:click={() => (id === 'categories' ? enterCategories() : id === 'settings' ? openSettings(account) : (section = id))}
          class="px-4 py-2 rounded-xl text-xs font-semibold {section === id ? 'bg-indigo-600 text-white' : 'bg-neutral-100 dark:bg-neutral-900 text-neutral-600 dark:text-neutral-400 border border-neutral-200 dark:border-neutral-800'}">{label}</button>
      {/each}
    </nav>

    {#if section === 'overview' && dash}
      <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <div class="card p-4"><p class="text-[11px] text-neutral-500 uppercase">Balance</p><p class="text-xl font-bold tabular-nums text-neutral-900 dark:text-white">{$money(dash.balance)}</p></div>
        <div class="card p-4"><p class="text-[11px] text-neutral-500 uppercase">Spent {dash.month}</p><p class="text-xl font-bold tabular-nums text-neutral-900 dark:text-white">{$money(dash.actual_total)}</p><p class="text-[11px] text-neutral-500">Expected {$money(dash.expected_total)}</p></div>
        <div class="card p-4"><p class="text-[11px] text-neutral-500 uppercase">Deposit target</p><p class="text-xl font-bold tabular-nums text-indigo-700 dark:text-indigo-300">{$money(dash.target)}</p><p class="text-[11px] text-neutral-500">+{dash.safety_margin_pct}% margin</p></div>
        <div class="card p-4"><p class="text-[11px] text-neutral-500 uppercase">Deposited</p><p class="text-xl font-bold tabular-nums text-neutral-900 dark:text-white">{$money(dash.deposited_total)}</p>
          <p class="text-[11px] font-semibold {dash.status.status === 'target_met' ? 'text-emerald-600' : dash.status.status === 'pending' ? 'text-indigo-600' : 'text-amber-600'}">
            {dash.status.status === 'target_met' ? '✓ Target met' : dash.status.status === 'pending' ? `⏳ Pending (due day ${dash.status.due_day})` : '⚠ Below target'}
          </p>
        </div>
      </div>
      <div class="card p-4 text-xs flex flex-wrap gap-4 justify-between">
        <span>Month {dash.month_surplus >= 0 ? 'surplus' : 'deficit'}: <strong class="tabular-nums {dash.month_surplus >= 0 ? 'text-emerald-600 dark:text-emerald-400' : 'text-rose-600 dark:text-red-400'}">{$money(dash.month_surplus)}</strong></span>
        {#if dash.top_up > 0}<span>💡 Top up at least <strong class="tabular-nums">{$money(dash.top_up)}</strong> to reach the target.</span>{/if}
      </div>
      <div class="card p-4 space-y-2">
        <h3 class="text-xs font-bold uppercase text-neutral-600 dark:text-neutral-400">Category usage</h3>
        {#each dash.categories as c (c.category)}
          <div class="space-y-1">
            <div class="flex justify-between text-xs"><span class="font-semibold text-neutral-800 dark:text-neutral-200">{$categoryName(c.category)}</span><span class="tabular-nums">{$money(c.actual)} / {$money(c.expected)} ({fmtPct(c.pct_used)})</span></div>
            <div class="w-full h-1.5 bg-neutral-200 dark:bg-neutral-800 rounded-full overflow-hidden"><div class="h-full rounded-full bg-indigo-500" style="width: {Math.min(c.pct_used, 100)}%"></div></div>
          </div>
        {/each}
      </div>
    {/if}

    {#if section === 'categories'}
      <div class="card p-5 space-y-3">
        <label class="flex items-center gap-2 text-xs text-neutral-600 dark:text-neutral-400"><input type="checkbox" bind:checked={grossMode} class="accent-indigo-600" /> Use one gross expected total instead of per-category costs</label>
        {#if grossMode}
          <input id="joint-gross" type="text" inputmode="decimal" bind:value={grossText} placeholder="Expected monthly total" class="input-field max-w-xs tabular-nums" />
        {/if}
        <div class="space-y-1.5">
          {#each $expenseCategories as c (c.id)}
            <div class="flex items-center gap-3 text-xs">
              <label class="flex items-center gap-2 flex-1"><input type="checkbox" checked={c.id in expectedDraft} on:change={() => toggleCategory(c.id)} class="accent-indigo-600" /> {c.name}{c.pay_mode === 'personal' ? ' (personal)' : ''}</label>
              {#if c.id in expectedDraft && !grossMode}
                <input type="text" inputmode="decimal" bind:value={expectedDraft[c.id]} class="input-field py-1 text-xs w-28 tabular-nums" />
              {/if}
            </div>
          {/each}
        </div>
        <button id="joint-save-categories" type="button" on:click={saveCategories} class="btn-primary">Save</button>
      </div>
    {/if}

    {#if section === 'deposits' && proposal}
      <div class="card p-5 space-y-3">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <h3 class="text-sm font-bold text-neutral-800 dark:text-neutral-200">Proposed deposits · {MODES.find(([m]) => m === proposal.mode)?.[1]} · target {$money(proposal.target)}</h3>
          <label class="text-xs text-neutral-600 dark:text-neutral-400">Round up to
            <select bind:value={roundTo} class="select-field text-xs py-1 w-auto ml-1">
              {#each ROUND_STEPS as step}<option value={step}>{step ? $money(step, true) : 'exact'}</option>{/each}
            </select>
          </label>
        </div>
        {#each account.members as m (m.member)}
          {@const paid = depositedBy.get(m.member)}
          <div class="card-sub p-3 flex flex-wrap items-center gap-3 text-xs border-l-4" style="border-left-color: {memberColor(m.member)}">
            <span class="font-semibold text-neutral-900 dark:text-white flex-1">{memberLabel(m.member)} <span class="text-neutral-500 font-normal">· due day {m.due_day}</span></span>
            <span class="tabular-nums font-bold">{$money(suggested[m.member] ?? 0)}</span>
            {#if paid}
              <span class="text-emerald-600 dark:text-emerald-400">✓ {$money(paid.amount)} on {fmtDate(paid.date)}</span>
              <button type="button" on:click={() => unmark(paid)} class="text-neutral-500">Undo</button>
            {:else}
              <button type="button" on:click={() => markPaid(m.member)} class="btn-secondary text-xs">Mark paid</button>
            {/if}
          </div>
        {/each}
        {#if proposal.mode !== 'fixed'}
          <button type="button" on:click={lockProposalAsFixed} class="btn-ghost text-xs">Save these amounts as fixed deposits</button>
        {/if}
      </div>
    {/if}

    {#if section === 'corrections'}
      <div class="grid grid-cols-1 lg:grid-cols-5 gap-6">
        <form class="lg:col-span-2 card p-5 space-y-3" on:submit|preventDefault={addCorrection}>
          <h3 class="text-sm font-bold text-neutral-800 dark:text-neutral-200">Balance correction</h3>
          <div class="grid grid-cols-2 gap-3">
            <input id="corr-amount" type="text" inputmode="decimal" bind:value={corr.amount} placeholder="Amount ({$currencySymbol})" class="input-field tabular-nums" />
            <input id="corr-date" type="date" bind:value={corr.date} class="input-field" />
          </div>
          <input id="corr-note" type="text" bind:value={corr.note} placeholder="Note" class="input-field" />
          <label class="flex items-center gap-2 text-xs text-neutral-600 dark:text-neutral-400"><input type="checkbox" bind:checked={corr.negative} class="accent-rose-600" /> Negative (money out)</label>
          <button id="corr-save" type="submit" class="btn-primary">Log correction</button>
        </form>
        <div class="lg:col-span-3 space-y-2">
          {#each corrections as c (c.id)}
            <div class="card-sub p-3 flex items-center justify-between gap-3 text-xs">
              <span><span class="font-mono text-neutral-400 mr-2">{fmtDate(c.date)}</span>{c.note || '—'}</span>
              <span class="flex items-center gap-3"><strong class="tabular-nums {c.amount >= 0 ? 'text-emerald-600 dark:text-emerald-400' : 'text-rose-600 dark:text-red-400'}">{$money(c.amount)}</strong>
                <button type="button" on:click={() => entryApi.remove(c.id)} class="text-rose-600 dark:text-red-400">Delete</button></span>
            </div>
          {:else}
            <p class="text-xs text-neutral-500">No corrections.</p>
          {/each}
        </div>
      </div>
    {/if}

    {#if section === 'settings' && draft}
      <div class="card p-5 space-y-3 max-w-2xl">
        <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
          <label class="block text-xs text-neutral-600 dark:text-neutral-400 sm:col-span-2">Name <input type="text" bind:value={draft.name} class="input-field mt-1" /></label>
          <label class="block text-xs text-neutral-600 dark:text-neutral-400">Safety margin % <input type="number" min="0" step="0.5" bind:value={draft.margin} class="input-field mt-1" /></label>
        </div>
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">Deposit split
          <select bind:value={draft.mode} class="select-field mt-1">{#each MODES as [value, label]}<option {value}>{label}</option>{/each}</select>
        </label>
        {#each draft.members as m (m.member)}
          <div class="flex items-center gap-3 text-xs">
            <span class="flex-1 font-semibold" style="color: {memberColor(m.member)}">{memberLabel(m.member)}</span>
            <label>Due day <input type="number" min="1" max="31" bind:value={m.due_day} class="input-field py-1 w-16 inline" /></label>
            {#if draft.mode === 'fixed'}<label>Fixed <input type="text" inputmode="decimal" bind:value={m.fixed} class="input-field py-1 w-24 inline tabular-nums" /></label>{/if}
            <button type="button" on:click={() => toggleDraftMember(m.member)} class="text-rose-600 dark:text-red-400">Remove</button>
          </div>
        {/each}
        <div class="flex flex-wrap gap-2">
          {#each $activeMembers.filter((x) => !draft.members.some((m) => m.member === x.id)) as x (x.id)}
            <button type="button" on:click={() => toggleDraftMember(x.id)} class="btn-ghost text-xs">+ {x.name}</button>
          {/each}
        </div>
        <div class="flex gap-2">
          <button type="button" on:click={() => saveAccount(draftInput(), 'Settings saved.')} class="btn-primary">Save settings</button>
          <button type="button" on:click={remove} class="btn-secondary text-rose-600">Delete account</button>
        </div>
      </div>
    {/if}
  {/if}
</div>

{#if showCreate && draft}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm" role="dialog" aria-modal="true">
    <form class="card max-w-md w-full shadow-2xl space-y-3" on:submit|preventDefault={create}>
      <h3 class="text-base font-bold text-neutral-900 dark:text-white">New joint account</h3>
      <input id="joint-name" name="account-name" type="text" bind:value={draft.name} placeholder="Account name" aria-label="Account name" class="input-field" />
      <div class="grid grid-cols-2 gap-3">
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">Safety margin % <input id="joint-margin" type="number" min="0" bind:value={draft.margin} class="input-field mt-1" /></label>
        <label class="block text-xs text-neutral-600 dark:text-neutral-400">Deposit split
          <select id="joint-mode" bind:value={draft.mode} class="select-field mt-1">{#each MODES as [value, label]}<option {value}>{label}</option>{/each}</select>
        </label>
      </div>
      <div class="flex flex-wrap gap-2">
        {#each $activeMembers as x (x.id)}
          {@const on = draft.members.some((m) => m.member === x.id)}
          <button type="button" on:click={() => toggleDraftMember(x.id)} class="px-2.5 py-1 rounded-lg text-xs font-semibold border {on ? 'text-white' : 'border-neutral-200 dark:border-neutral-800 text-neutral-600'}" style={on ? `background-color:${x.color};border-color:${x.color}` : ''}>{x.name}</button>
        {/each}
      </div>
      <p class="text-[11px] text-neutral-500">Opening balance: log a correction after creating the account.</p>
      <div class="flex gap-2">
        <button type="button" on:click={() => (showCreate = false)} class="btn-secondary flex-1">Cancel</button>
        <button id="joint-create" type="submit" class="btn-primary flex-1">Create</button>
      </div>
    </form>
  </div>
{/if}
