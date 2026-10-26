<script>
  /**
   * ExpenseForm.svelte — logs one expense. Amount text becomes integer cents;
   * the server validates shape, tag window, sealed months and split totals.
   */

  import { fly } from 'svelte/transition';
  import { entryApi } from './api.js';
  import DuplicatePromptModal from './DuplicatePromptModal.svelte';
  import SplitEditor, { toShares } from './SplitEditor.svelte';
  import { toCents, today } from './format.js';
  import {
    activeMembers, expenseCategories, projects, tags, seals, accounts,
    defaultPayer, defaultCategory, defaultProject, currencySymbol, jointAccountEnabled,
    activeAccountId, showProjectsInExpense, formMemoryMode, lastLoggedPayer,
    lastLoggedCategory, enabledFormFields,
  } from './stores.js';

  const JOINT = 'JOINT';

  function remembered(last, preset) {
    if ($formMemoryMode === 'empty') return '';
    return $formMemoryMode === 'remember_last' && last ? last : preset;
  }

  let name = '';
  let amountText = '';
  let date = today();
  let paidByJoint = $defaultPayer === JOINT;
  let accountId = null;
  let memberId = Number(remembered($lastLoggedPayer, $defaultPayer === JOINT ? '' : $defaultPayer)) || null;
  let categoryId = Number(remembered($lastLoggedCategory, $defaultCategory)) || null;
  let projectId = Number($defaultProject) || null;
  let tagId = null;
  let customSplit = false;
  let shares = {};

  let submitting = false;
  let submitSuccess = false;
  let errorMsg = null;
  let successName = null;
  let successTimer = null;

  let showDuplicateModal = false;
  let duplicateItems = [];
  let pendingPayload = null;

  $: activeTags = $tags.filter((t) => (!t.valid_from || t.valid_from <= date) && (!t.valid_to || t.valid_to >= date));
  $: isMonthLocked = $seals.some((s) => s.month === date?.slice(0, 7));
  $: if (paidByJoint && !accountId) accountId = $activeAccountId ?? $accounts[0]?.id ?? null;
  $: account = $accounts.find((a) => a.id === accountId);
  $: isUncoupledJointCategory = paidByJoint && categoryId && account && !account.categories.some((c) => c.category === categoryId);

  function reset() {
    name = '';
    amountText = '';
    date = today();
    paidByJoint = $defaultPayer === JOINT;
    memberId = Number(remembered($lastLoggedPayer, $defaultPayer === JOINT ? '' : $defaultPayer)) || null;
    categoryId = Number(remembered($lastLoggedCategory, $defaultCategory)) || null;
    projectId = Number($defaultProject) || null;
    tagId = null;
    customSplit = false;
    shares = {};
    errorMsg = null;
  }

  function payload() {
    return {
      kind: 'expense',
      date,
      amount: toCents(amountText),
      member: memberId,
      category: categoryId,
      account: paidByJoint ? accountId : null,
      project: projectId,
      tag: tagId,
      name,
      overrides: !paidByJoint && customSplit ? toShares(shares) : [],
    };
  }

  function missing(p) {
    if (!p.name.trim()) return 'Description is required.';
    if (!p.amount || p.amount <= 0) return 'Amount must be greater than 0.';
    if (!p.member) return 'Please select who paid.';
    if (!p.category) return 'Please select a category.';
    return null;
  }

  async function handleSubmit(e) {
    e.preventDefault();
    successName = null;
    const p = payload();
    errorMsg = missing(p);
    if (errorMsg) return;
    try {
      const [found] = await entryApi.duplicates([{ category: p.category, amount: p.amount, date: p.date }]);
      if (found) {
        pendingPayload = p;
        duplicateItems = [{ prospective: p, matches: found.matches }];
        showDuplicateModal = true;
        return;
      }
      await save(p);
    } catch (err) {
      errorMsg = err.message;
    }
  }

  async function save(p) {
    submitting = true;
    try {
      await entryApi.create(p);
      if (!paidByJoint) lastLoggedPayer.set(String(p.member));
      lastLoggedCategory.set(String(p.category));
      successName = p.name;
      submitSuccess = true;
      reset();
      clearTimeout(successTimer);
      successTimer = setTimeout(() => { successName = null; }, 3000);
      setTimeout(() => { submitSuccess = false; }, 800);
    } catch (err) {
      errorMsg = err.message;
    } finally {
      submitting = false;
    }
  }

  async function confirmDuplicate() {
    showDuplicateModal = false;
    const p = pendingPayload;
    pendingPayload = null;
    duplicateItems = [];
    if (p) await save(p);
  }

  function cancelDuplicate() {
    showDuplicateModal = false;
    pendingPayload = null;
    duplicateItems = [];
  }

  function dismissSuccess() {
    clearTimeout(successTimer);
    successName = null;
  }
</script>

<form on:submit={handleSubmit} id="expense-form" class="space-y-4">

  <!-- 1. Description (Name) -->
  <div>
    <label for="expense-name" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1.5">
      Description
    </label>
    <input
      id="expense-name"
      type="text"
      maxlength="96"
      placeholder="e.g. Weekly groceries"
      bind:value={name}
      class="input-field"
    />
  </div>

  <!-- 2. Amount -->
  <div>
    <label for="expense-cost" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1.5">
      Amount ({$currencySymbol})
    </label>
    <div class="relative">
      <span class="absolute left-3 top-1/2 -translate-y-1/2 text-neutral-500 text-sm">{$currencySymbol}</span>
      <input
        id="expense-cost"
        type="text"
        inputmode="decimal"
        placeholder="0.00"
        bind:value={amountText}
        class="input-field pl-7"
      />
    </div>
  </div>

  <!-- 3. Date -->
  <div>
    <label for="expense-date" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1.5">
      Date
    </label>
    <input
      id="expense-date"
      type="date"
      bind:value={date}
      class="input-field"
    />
  </div>

  <!-- 4. Project (above categories, shown only if projects exist and feature enabled) -->
  {#if $enabledFormFields.projects && $showProjectsInExpense && $projects.length > 0}
    <div>
      <label for="expense-project" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1.5">
        Project <span class="text-neutral-500 dark:text-neutral-600">(optional)</span>
      </label>
      <select
        id="expense-project"
        bind:value={projectId}
        class="select-field"
      >
        <option value={null}>— No project —</option>
        {#each $projects as p}
          <option value={p.id}>{p.name}</option>
        {/each}
      </select>
    </div>
  {/if}

  <!-- 5. Category -->
  <div>
      <label for="expense-category" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1.5">
        Category
      </label>
      <select
        id="expense-category"
        bind:value={categoryId}
        class="select-field"
      >
        <option value={null}>— Select category —</option>
        {#each $expenseCategories as c (c.id)}
          <option value={c.id}>{c.name}</option>
        {/each}
      </select>
  </div>

  <!-- 6. Tag (dropdown) — only visible when active tags exist -->
  {#if $enabledFormFields.tags && activeTags.length > 0}
    <div>
      <label for="expense-tag" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1.5">
        Tag <span class="text-neutral-500 dark:text-neutral-600">(optional)</span>
      </label>
      <select
        id="expense-tag"
        bind:value={tagId}
        class="select-field"
      >
        <option value={null}>— No tag —</option>
        {#each activeTags as t}
          <option value={t.id}>{t.name}</option>
        {/each}
      </select>
    </div>
  {/if}

  <!-- 7. Who paid (Final field) -->
  <div>
    <p id="who-paid-label" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-2.5">Who paid?</p>
    <div class="flex flex-wrap gap-3">
      {#each $activeMembers as u (u.id)}
        <label class="flex items-center gap-2.5 text-sm text-neutral-700 dark:text-neutral-300 cursor-pointer select-none hover:text-neutral-900 dark:hover:text-white transition-colors group">
          <div class="relative flex items-center justify-center">
            <input
              type="checkbox"
              id="who-paid-{u.id}"
              checked={memberId === u.id}
              on:change={() => { memberId = memberId === u.id ? null : u.id; }}
              class="sr-only"
            />
            <div
              class="w-5 h-5 rounded border transition-all duration-150 flex items-center justify-center
                     {memberId === u.id ? 'text-white shadow-sm' : 'bg-white dark:bg-neutral-800 border-neutral-300 dark:border-neutral-700 group-hover:border-neutral-400 dark:group-hover:border-neutral-500'}"
              style={memberId === u.id ? `background-color:${u.color};border-color:${u.color}` : ''}
            >
              {#if memberId === u.id}
                <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
                </svg>
              {/if}
            </div>
          </div>
          <span>{u.name}</span>
        </label>
      {/each}

      {#if $enabledFormFields.joint && $jointAccountEnabled && $accounts.length > 0}
        <!-- Paid by Joint Account option -->
        <label class="flex items-center gap-2.5 text-sm text-indigo-700 dark:text-indigo-300 cursor-pointer select-none hover:opacity-90 transition-colors group border border-indigo-200 dark:border-indigo-500/30 bg-indigo-50 dark:bg-indigo-500/10 px-3 py-1.5 rounded-lg">
          <div class="relative flex items-center justify-center">
            <input
              type="checkbox"
              id="paid-by-joint-checkbox"
              checked={paidByJoint}
              on:change={() => { paidByJoint = !paidByJoint; }}
              class="sr-only"
            />
            <div
              class="w-5 h-5 rounded border transition-all duration-150 flex items-center justify-center
                     {paidByJoint ? 'bg-indigo-600 border-indigo-500 text-white shadow-sm' : 'bg-white dark:bg-neutral-800 border-neutral-300 dark:border-neutral-700 group-hover:border-neutral-400 dark:group-hover:border-neutral-500'}"
            >
              {#if paidByJoint}
                <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
                </svg>
              {/if}
            </div>
          </div>
          <span class="font-medium flex items-center gap-1">🏦 Joint Account</span>
        </label>
      {/if}
    </div>

    <!-- Joint Account Selector (if multiple exist) -->
    {#if paidByJoint && $accounts.length > 1}
      <div class="mt-2.5 p-2.5 rounded-xl bg-indigo-50 dark:bg-indigo-950/40 border border-indigo-200 dark:border-indigo-700/40 space-y-1.5 animate-fadeIn">
        <label for="expense-joint-account" class="block text-xs font-semibold text-indigo-700 dark:text-indigo-300">
          Select Joint Account
        </label>
        <select
          id="expense-joint-account"
          bind:value={accountId}
          class="select-field text-xs py-1.5"
        >
          {#each $accounts as acc (acc.id)}
            <option value={acc.id}>🏦 {acc.name}</option>
          {/each}
        </select>
      </div>
    {/if}
  </div>

  <!-- Warning for uncoupled joint category -->
  {#if isUncoupledJointCategory}
    <div class="p-2.5 bg-amber-50 dark:bg-amber-950/40 border border-amber-200 dark:border-amber-800/60 text-amber-800 dark:text-amber-300 rounded-lg text-xs flex items-center gap-2">
      <span class="text-amber-500 dark:text-amber-400 font-bold">⚠️</span>
      <span>This category is not coupled to {account.name}. The expense will still be charged to the joint account.</span>
    </div>
  {/if}

  <!-- Custom split -->
  {#if $enabledFormFields.splitOverride && !paidByJoint && $activeMembers.length >= 2}
    <div class="space-y-3">
      <label class="flex items-center gap-2.5 cursor-pointer select-none group">
        <div class="relative">
          <input type="checkbox" bind:checked={customSplit} class="sr-only" id="custom-split-toggle" />
          <div class="w-10 h-5 rounded-full transition-colors duration-200 {customSplit ? 'bg-indigo-600' : 'bg-neutral-300 dark:bg-neutral-700'}"></div>
          <div class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform duration-200 {customSplit ? 'translate-x-5' : ''}"></div>
        </div>
        <span class="text-xs font-medium text-neutral-600 dark:text-neutral-400 group-hover:text-neutral-900 dark:group-hover:text-neutral-200 transition-colors">Custom Split</span>
      </label>
      {#if customSplit}
        <SplitEditor members={$activeMembers} bind:shares idPrefix="expense-split" />
      {/if}
    </div>
  {/if}

  <!-- Lock warning -->
  {#if isMonthLocked}
    <div class="flex items-center gap-2 bg-amber-50 dark:bg-amber-950/40 border border-amber-200 dark:border-amber-700/50 text-amber-800 dark:text-amber-400 rounded-lg px-3 py-2 text-xs">
      <svg class="w-3.5 h-3.5 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
        <path fill-rule="evenodd" d="M10 1a4.5 4.5 0 00-4.5 4.5V9H5a2 2 0 00-2 2v6a2 2 0 002 2h10a2 2 0 002-2v-6a2 2 0 00-2-2h-.5V5.5A4.5 4.5 0 0010 1zm3 8V5.5a3 3 0 10-6 0V9h6z" clip-rule="evenodd"/>
      </svg>
      This month is locked and cannot accept new expenses.
    </div>
  {/if}

  <!-- Feedback -->
  {#if errorMsg}
    <p class="text-rose-700 dark:text-red-400 text-xs bg-rose-50 dark:bg-red-950/40 border border-rose-200 dark:border-red-900 rounded-lg px-3 py-2" transition:fly={{ y: -6, duration: 250 }}>
      {errorMsg}
    </p>
  {/if}
  {#if successName}
    <button
      type="button"
      class="text-emerald-700 dark:text-emerald-400 text-xs bg-emerald-50 dark:bg-emerald-950/40 border border-emerald-200 dark:border-emerald-900 rounded-lg px-3 py-2
             flex items-center justify-between gap-2 cursor-pointer w-full text-left"
      transition:fly={{ y: -6, duration: 250 }}
      on:click={dismissSuccess}
      title="Click to dismiss">
      <span>✓ &ldquo;{successName}&rdquo; logged successfully.</span>
      <span class="text-emerald-600 dark:text-emerald-400 hover:opacity-80 transition-colors text-xs leading-none" aria-hidden="true">✕</span>
    </button>
  {/if}

  <!-- Submit -->
  <button
    id="submit-expense"
    type="submit"
    disabled={submitting || isMonthLocked}
    class="btn-primary w-full"
  >
    {#if submitting}
      Saving…
    {:else if submitSuccess}
      <span class="flex items-center justify-center gap-1.5">
        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"/></svg>
        Logged!
      </span>
    {:else}
      Log Expense
    {/if}
  </button>
</form>

<DuplicatePromptModal
  isOpen={showDuplicateModal}
  items={duplicateItems}
  isBatch={false}
  on:confirm={confirmDuplicate}
  on:cancel={cancelDuplicate}
/>

