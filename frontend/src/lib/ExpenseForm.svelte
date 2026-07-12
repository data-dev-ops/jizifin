<script>
  /**
   * ExpenseForm.svelte
   *
   * Controlled form for logging a new expense.
   * cost is entered in euros (decimal) and converted to whole cents before POST.
   * On success: expense store is updated via api.js and analytics are refreshed.
   */

  import { fly } from 'svelte/transition';
  import { createExpense } from './api.js';
  import { splits, selectedMonth, projects, tags, settlements, users, defaultPayer, defaultCategory, currencySymbol } from './stores.js';

  $: activeUsers = $users.filter((u) => u.is_active);

  // ── Lock check ────────────────────────────────────────────────────────────
  /** True if expenseDate falls in a locked month */
  $: isMonthLocked = expenseDate
    ? $settlements.some((s) => s.month === expenseDate.slice(0, 7))
    : false;

  function today() {
    const d = new Date();
    const year = d.getFullYear();
    const month = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${year}-${month}-${day}`;
  }

  // ── Custom split state ────────────────────────────────────────────────────
  let customSplit = false;
  /** { [userName]: pctNumber } */
  let overridePcts = {};

  // Initialise override inputs whenever activeUsers changes
  $: if (customSplit && activeUsers.length > 0) {
    for (const u of activeUsers) {
      if (!(u.name in overridePcts)) {
        overridePcts[u.name] = parseFloat((100 / activeUsers.length).toFixed(4));
      }
    }
  }

  $: overrideSum = Object.values(overridePcts).reduce((s, v) => s + (parseFloat(v) || 0), 0);
  $: overrideOk  = Math.abs(overrideSum - 100.0) <= 0.01;

  // ── Form state ──────────────────────────────────────────────────────────────────
  let name         = '';
  let costEuros    = '';       // user-facing input, e.g. "12.50"
  let expenseDate  = today();
  let whoPaid      = $defaultPayer;
  let category     = $defaultCategory;
  let projectId    = null;     // optional: link expense to a project
  let tagId        = null;     // optional: link expense to a tag

  let submitting   = false;
  let submitSuccess = false; // brief checkmark state on the button
  let errorMsg     = null;
  let successName  = null;
  let successTimer = null;

  function reset() {
    name        = '';
    costEuros   = '';
    expenseDate = today();
    whoPaid     = $defaultPayer;
    category    = $defaultCategory;
    projectId   = null;
    tagId       = null;
    errorMsg    = null;
    customSplit = false;
    overridePcts = {};
  }

  function dismissSuccess() {
    if (successTimer) clearTimeout(successTimer);
    successName = null;
  }

  // ── Submit ────────────────────────────────────────────────────────────────
  async function handleSubmit(e) {
    e.preventDefault();
    errorMsg   = null;
    successName = null;

    if (!name.trim()) {
      errorMsg = 'Description is required.';
      return;
    }

    // 1. Amount validation
    if (costEuros === null || costEuros === undefined || costEuros === '') {
      errorMsg = 'Amount is required.';
      return;
    }
    const parsed = parseFloat(costEuros);
    if (isNaN(parsed) || parsed <= 0) {
      errorMsg = 'Amount must be a valid positive number.';
      return;
    }
    const costCents = Math.round(parsed * 100);
    if (costCents <= 0) {
      errorMsg = `Amount must be at least ${$currencySymbol}0.01.`;
      return;
    }

    // 2. Date validation
    if (!expenseDate) {
      errorMsg = 'Date is required.';
      return;
    }
    if (!/^\d{4}-\d{2}-\d{2}$/.test(expenseDate)) {
      errorMsg = 'Date must be in YYYY-MM-DD format.';
      return;
    }
    const parsedDate = new Date(expenseDate);
    if (isNaN(parsedDate.getTime())) {
      errorMsg = 'Please enter a valid date.';
      return;
    }

    // 3. Who paid validation
    if (!whoPaid) {
      errorMsg = 'Please select who paid.';
      return;
    }

    // 4. Category validation
    if (!category) {
      errorMsg = 'Please select a category.';
      return;
    }

    submitting = true;
    try {
      const payload = {
        name:         name.trim(),
        cost_cents:   costCents,
        expense_date: expenseDate,
        who_paid:     whoPaid,
        category,
        project_id:   projectId ? Number(projectId) : null,
        tag_id:       tagId ? Number(tagId) : null,
      };
      if (customSplit && overrideOk) {
        payload.overrides = Object.entries(overridePcts).map(([user_name, pct]) => ({
          user_name,
          pct: parseFloat(parseFloat(pct).toFixed(4)),
        }));
      }
      await createExpense(payload, $selectedMonth);
      successName = name.trim();
      submitSuccess = true;
      reset();
      // Auto-dismiss success banner after 3 s
      if (successTimer) clearTimeout(successTimer);
      successTimer = setTimeout(() => { successName = null; }, 3000);
      // Reset button checkmark after 800 ms
      setTimeout(() => { submitSuccess = false; }, 800);
    } catch (err) {
      errorMsg = err.message ?? 'An unexpected error occurred.';
    } finally {
      submitting = false;
    }
  }
</script>

<form on:submit={handleSubmit} id="expense-form" class="space-y-4">

  <!-- Name -->
  <div>
    <label for="expense-name" class="block text-xs font-medium text-neutral-400 mb-1.5">
      Description
    </label>
    <input
      id="expense-name"
      type="text"
      maxlength="96"
      placeholder="e.g. Weekly groceries"
      bind:value={name}
      class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2.5 text-sm
             text-neutral-100 placeholder-neutral-600
             focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500
             transition-colors"
    />
  </div>

  <!-- Cost -->
  <div>
    <label for="expense-cost" class="block text-xs font-medium text-neutral-400 mb-1.5">
      Amount ({$currencySymbol})
    </label>
    <div class="relative">
      <span class="absolute left-3 top-1/2 -translate-y-1/2 text-neutral-500 text-sm">{$currencySymbol}</span>
      <input
        id="expense-cost"
        type="number"
        min="0.01"
        step="0.01"
        placeholder="0.00"
        bind:value={costEuros}
        class="w-full bg-neutral-800 border border-neutral-700 rounded-lg pl-7 pr-3 py-2.5 text-sm
               text-neutral-100 placeholder-neutral-600
               focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500
               transition-colors"
      />
    </div>
  </div>

  <!-- Date -->
  <div>
    <label for="expense-date" class="block text-xs font-medium text-neutral-400 mb-1.5">
      Date
    </label>
    <input
      id="expense-date"
      type="date"
      bind:value={expenseDate}
      class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2.5 text-sm
             text-neutral-100
             focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500
             transition-colors [color-scheme:dark]"
    />
  </div>

  <!-- Who paid (dynamic from users store) -->
  <div>
    <p id="who-paid-label" class="block text-xs font-medium text-neutral-400 mb-2.5">Who paid?</p>
    <div class="flex flex-wrap gap-3">
      {#each activeUsers as u (u.name)}
        <label class="flex items-center gap-2.5 text-sm text-neutral-300 cursor-pointer select-none hover:text-white transition-colors group">
          <div class="relative flex items-center justify-center">
            <input
              type="checkbox"
              id="who-paid-{u.name.toLowerCase()}"
              checked={whoPaid === u.name}
              on:change={() => (whoPaid = whoPaid === u.name ? '' : u.name)}
              class="sr-only"
            />
            <div
              class="w-5 h-5 rounded border transition-all duration-150 flex items-center justify-center
                     {whoPaid === u.name ? 'text-white shadow-sm' : 'bg-neutral-800 border-neutral-700 group-hover:border-neutral-500'}"
              style={whoPaid === u.name ? `background-color:${u.color};border-color:${u.color}` : ''}
            >
              {#if whoPaid === u.name}
                <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
                </svg>
              {/if}
            </div>
          </div>
          <span>{u.name}</span>
        </label>
      {/each}
    </div>
  </div>

  <!-- Category -->
  <div>
    <span class="block text-xs font-medium text-neutral-400 mb-2.5">
      Category
    </span>
    <div class="grid grid-cols-2 gap-3 max-h-48 overflow-y-auto border border-neutral-800 bg-neutral-900/40 rounded-xl p-3.5 scrollbar-thin scrollbar-thumb-neutral-800 scrollbar-track-transparent">
      {#each $splits as split}
        <label class="flex items-center gap-2.5 text-sm text-neutral-300 cursor-pointer select-none hover:text-white transition-colors group">
          <div class="relative flex items-center justify-center">
            <input
              type="checkbox"
              id="category-{split.category.toLowerCase().replace(/\s+/g, '-')}"
              checked={category === split.category}
              on:change={() => category = (category === split.category ? '' : split.category)}
              class="sr-only"
            />
            <div class="w-5 h-5 rounded border transition-all duration-150 flex items-center justify-center
                        {category === split.category
                          ? 'bg-indigo-600 border-indigo-500 shadow-sm shadow-indigo-900/30'
                          : 'bg-neutral-800 border-neutral-700 group-hover:border-neutral-500'}"
            >
              {#if category === split.category}
                <svg class="w-3.5 h-3.5 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
                </svg>
              {/if}
            </div>
          </div>
          <span class="truncate">{split.category}</span>
        </label>
      {/each}
    </div>
  </div>

  <!-- Project (optional) -->
  {#if $projects.length > 0}
    <div>
      <label for="expense-project" class="block text-xs font-medium text-neutral-400 mb-1.5">
        Project <span class="text-neutral-600">(optional)</span>
      </label>
      <select
        id="expense-project"
        bind:value={projectId}
        class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2.5 text-sm
               text-neutral-100
               focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500
               transition-colors"
      >
        <option value={null}>— No project —</option>
        {#each $projects as p}
          <option value={p.id}>{p.name}</option>
        {/each}
      </select>
    </div>
  {/if}

  <!-- Tag (optional) -->
  {#if $tags.length > 0}
    <div>
      <label for="expense-tag" class="block text-xs font-medium text-neutral-400 mb-1.5">
        Tag <span class="text-neutral-600">(optional)</span>
      </label>
      <select
        id="expense-tag"
        bind:value={tagId}
        class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2.5 text-sm
               text-neutral-100
               focus:outline-none focus:border-amber-500 focus:ring-1 focus:ring-amber-500
               transition-colors"
      >
        <option value={null}>— No tag —</option>
        {#each $tags as t}
          <option value={t.id}>
            {t.name}{t.total_amount > 0 ? ` (${t.total_amount.toLocaleString('en-GB', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} total)` : ''}
          </option>
        {/each}
      </select>
    </div>
  {/if}

  <!-- Custom Split Toggle (N-user dynamic inputs) -->
  <div>
    <label class="flex items-center gap-2.5 cursor-pointer select-none group">
      <div class="relative">
        <input type="checkbox" bind:checked={customSplit} class="sr-only" id="custom-split-toggle" />
        <div class="w-10 h-5 rounded-full transition-colors duration-200 {customSplit ? 'bg-indigo-600' : 'bg-neutral-700'}"></div>
        <div class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform duration-200 {customSplit ? 'translate-x-5' : ''}"></div>
      </div>
      <span class="text-xs font-medium text-neutral-400 group-hover:text-neutral-200 transition-colors">Custom Split</span>
    </label>
    {#if customSplit}
      <div class="mt-3 p-3 bg-neutral-900 border border-indigo-800/40 rounded-xl space-y-2">
        {#each activeUsers as u (u.name)}
          <div class="flex items-center gap-3">
            <span class="text-xs font-semibold w-14 truncate" style="color: {u.color}">{u.name}</span>
            <input
              type="number" min="0" max="100" step="0.1"
              bind:value={overridePcts[u.name]}
              class="w-20 bg-neutral-800 border border-neutral-700 rounded-lg px-2 py-1.5 text-sm text-neutral-100
                     focus:outline-none focus:ring-1 transition-colors"
              style="--tw-ring-color: {u.color}"
            />
            <span class="text-xs text-neutral-500">%</span>
            <div class="flex-1 h-1.5 bg-neutral-800 rounded-full overflow-hidden">
              <div class="h-full transition-all" style="width:{Math.min(parseFloat(overridePcts[u.name])||0, 100)}%; background-color:{u.color}"></div>
            </div>
          </div>
        {/each}
        <div class="pt-1 flex items-center justify-between">
          <p class="text-[10px] text-neutral-600">Must sum to exactly 100%.</p>
          <span class="text-[10px] font-semibold {overrideOk ? 'text-emerald-400' : 'text-amber-400'}">
            Sum: {overrideSum.toFixed(2)}%
          </span>
        </div>
      </div>
    {/if}
  </div>

  <!-- Lock warning -->
  {#if isMonthLocked}
    <div class="flex items-center gap-2 bg-amber-950/40 border border-amber-700/50 text-amber-400 rounded-lg px-3 py-2 text-xs">
      <svg class="w-3.5 h-3.5 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
        <path fill-rule="evenodd" d="M10 1a4.5 4.5 0 00-4.5 4.5V9H5a2 2 0 00-2 2v6a2 2 0 002 2h10a2 2 0 002-2v-6a2 2 0 00-2-2h-.5V5.5A4.5 4.5 0 0010 1zm3 8V5.5a3 3 0 10-6 0V9h6z" clip-rule="evenodd"/>
      </svg>
      This month is locked and cannot accept new expenses.
    </div>
  {/if}

  <!-- Feedback -->
  {#if errorMsg}
    <p class="text-red-400 text-xs bg-red-950/40 border border-red-900 rounded-lg px-3 py-2" transition:fly={{ y: -6, duration: 250 }}>
      {errorMsg}
    </p>
  {/if}
  {#if successName}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <p class="text-emerald-400 text-xs bg-emerald-950/40 border border-emerald-900 rounded-lg px-3 py-2
              flex items-center justify-between gap-2 cursor-pointer"
       transition:fly={{ y: -6, duration: 250 }}
       on:click={dismissSuccess}
       title="Click to dismiss">
      <span>✓ &ldquo;{successName}&rdquo; logged successfully.</span>
      <span class="text-emerald-700 hover:text-emerald-400 transition-colors text-xs leading-none" aria-hidden="true">✕</span>
    </p>
  {/if}

  <!-- Submit -->
  <button
    id="submit-expense"
    type="submit"
    disabled={submitting || isMonthLocked}
    class="w-full py-2.5 rounded-lg text-sm font-semibold
           bg-gradient-to-r from-indigo-600 to-violet-600
           hover:from-indigo-500 hover:to-violet-500
           disabled:opacity-50 disabled:cursor-not-allowed
           transition-all duration-150 shadow-md shadow-indigo-900/30
           active:scale-[0.98]"
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
