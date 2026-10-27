<script>
  /**
   * EntryEditModal.svelte — edits one expense or income entry in place.
   */
  import { createEventDispatcher } from 'svelte';
  import { entryApi } from './api.js';
  import { entryInput } from './entryInput.js';
  import SplitEditor, { toShares, fromShares } from './SplitEditor.svelte';
  import { toCents, centsInput, money } from './format.js';
  import {
    memberName, activeMembers, expenseCategories, incomeCategories, accounts, projects, tags,
    jointAccountEnabled, showProjectsInExpense, currencySymbol,
  } from './stores.js';

  /** Entry being edited, or null when closed. */
  export let entry = null;

  const dispatch = createEventDispatcher();

  let draft = null;
  let amountText = '';
  let paidByJoint = false;
  let customSplit = false;
  let shares = {};
  let saving = false;
  let error = null;
  let resolved = null;

  $: if (!entry) {
    draft = null;
  } else if (!draft || draft.id !== entry.id) {
    open(entry);
  }
  $: isExpense = draft?.kind === 'expense';
  $: categoryOptions = isExpense ? $expenseCategories : $incomeCategories;

  function open(e) {
    draft = { ...e };
    amountText = centsInput(e.amount);
    paidByJoint = e.account != null;
    customSplit = (e.overrides ?? []).length > 0;
    shares = fromShares(e.overrides);
    error = null;
    loadResolved(e);
  }

  async function loadResolved(e) {
    resolved = null;
    if (e.kind !== 'expense' || e.account != null) return;
    try {
      resolved = await entryApi.shares(e.id);
    } catch {
      resolved = null;
    }
  }

  function close() {
    entry = null;
    draft = null;
    dispatch('close');
  }

  async function save() {
    error = null;
    const amount = toCents(amountText);
    if (!amount || amount <= 0) {
      error = 'Please enter a valid positive amount.';
      return;
    }
    saving = true;
    try {
      await entryApi.update(draft.id, entryInput(draft, {
        amount,
        account: paidByJoint ? draft.account ?? $accounts[0]?.id ?? null : null,
        tag: isExpense ? draft.tag : null,
        overrides: isExpense && !paidByJoint && customSplit ? toShares(shares) : [],
      }));
      close();
    } catch (err) {
      error = err.message;
    } finally {
      saving = false;
    }
  }
</script>

<svelte:window on:keydown={(e) => { if (e.key === 'Escape' && draft) close(); }} />

{#if draft}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div
    class="fixed inset-0 z-50 !m-0 w-full h-full min-h-screen flex items-center justify-center p-3 sm:p-4 bg-black/60 dark:bg-black/75 backdrop-blur-sm animate-fadeIn"
    role="dialog"
    aria-modal="true"
    on:click|self={close}
  >
    <form on:submit|preventDefault={save} class="card max-w-lg w-full shadow-2xl space-y-4 max-h-[90vh] overflow-y-auto" id="entry-edit-form">
      <div class="flex items-center justify-between border-b border-neutral-200 dark:border-neutral-800 pb-3">
        <h3 class="text-base font-bold text-neutral-900 dark:text-white">Edit {isExpense ? 'Expense' : 'Income'}</h3>
        <button type="button" on:click={close} class="text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 text-lg p-1 rounded-lg hover:bg-neutral-100 dark:hover:bg-neutral-800 transition-colors" aria-label="Close">✕</button>
      </div>

      <div>
        <label for="edit-name" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1.5">Description</label>
        <input id="edit-name" type="text" maxlength="96" bind:value={draft.name} class="input-field" />
      </div>

      <div class="grid grid-cols-2 gap-3">
        <div>
          <label for="edit-amount" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1.5">Amount ({$currencySymbol})</label>
          <input id="edit-amount" type="text" inputmode="decimal" bind:value={amountText} class="input-field" />
        </div>
        <div>
          <label for="edit-date" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1.5">Date</label>
          <input id="edit-date" type="date" bind:value={draft.date} class="input-field" />
        </div>
      </div>

      <div class="grid grid-cols-2 gap-3">
        <div>
          <label for="edit-member" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1.5">{isExpense ? 'Paid by' : 'Earned by'}</label>
          <select id="edit-member" bind:value={draft.member} class="select-field">
            {#each $activeMembers as m (m.id)}<option value={m.id}>{m.name}</option>{/each}
          </select>
        </div>
        <div>
          <label for="edit-category" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1.5">Category</label>
          <select id="edit-category" bind:value={draft.category} class="select-field">
            {#each categoryOptions as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
          </select>
        </div>
      </div>

      {#if isExpense && $showProjectsInExpense && $projects.length > 0}
        <div>
          <label for="edit-project" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1.5">Project</label>
          <select id="edit-project" bind:value={draft.project} class="select-field">
            <option value={null}>— No project —</option>
            {#each $projects as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
          </select>
        </div>
      {/if}

      {#if isExpense && $tags.length > 0}
        <div>
          <label for="edit-tag" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1.5">Tag</label>
          <select id="edit-tag" bind:value={draft.tag} class="select-field">
            <option value={null}>— No tag —</option>
            {#each $tags as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
          </select>
        </div>
      {/if}

      {#if $jointAccountEnabled && $accounts.length > 0}
        <label class="flex items-center gap-2 text-xs font-medium text-indigo-700 dark:text-indigo-300">
          <input id="edit-joint" type="checkbox" bind:checked={paidByJoint} class="accent-indigo-600" />
          🏦 Joint account
        </label>
        {#if paidByJoint && $accounts.length > 1}
          <select id="edit-account" bind:value={draft.account} class="select-field text-xs">
            {#each $accounts as a (a.id)}<option value={a.id}>🏦 {a.name}</option>{/each}
          </select>
        {/if}
      {/if}

      {#if isExpense && !paidByJoint && $activeMembers.length >= 2}
        <label class="flex items-center gap-2 text-xs font-medium text-neutral-600 dark:text-neutral-400">
          <input id="edit-custom-split" type="checkbox" bind:checked={customSplit} class="accent-indigo-600" />
          Custom split
        </label>
        {#if customSplit}
          <SplitEditor members={$activeMembers} bind:shares idPrefix="edit-split" />
        {/if}
      {/if}

      {#if resolved}
        <div id="edit-resolved-shares" class="text-xs rounded-lg border border-neutral-200 dark:border-neutral-800 px-3 py-2 space-y-1">
          <p class="font-medium text-neutral-700 dark:text-neutral-400">Who bears what <span class="text-neutral-400">({resolved.source.replace('_', ' ')})</span></p>
          {#each Object.entries(resolved.shares) as [member, cents] (member)}
            <p class="flex justify-between text-neutral-600 dark:text-neutral-300"><span>{$memberName(Number(member))}</span><span>{$money(cents)}</span></p>
          {/each}
        </div>
      {/if}

      <div>
        <label for="edit-note" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1.5">Note</label>
        <input id="edit-note" type="text" maxlength="256" bind:value={draft.note} class="input-field" />
      </div>

      {#if error}
        <p class="text-rose-700 dark:text-red-400 text-xs bg-rose-50 dark:bg-red-950/40 border border-rose-200 dark:border-red-900 rounded-lg px-3 py-2">{error}</p>
      {/if}

      <div class="flex gap-2.5 pt-2">
        <button type="button" on:click={close} class="btn-secondary flex-1">Cancel</button>
        <button id="save-entry-edit" type="submit" disabled={saving} class="btn-primary flex-1">{saving ? 'Saving…' : 'Save Changes'}</button>
      </div>
    </form>
  </div>
{/if}
