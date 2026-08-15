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
  import { splits, selectedMonth, projects, tags, settlements, users, defaultPayer, defaultCategory, defaultProject, currencySymbol, jointAccountEnabled, jointAccount, jointCategories, showProjectsInExpense } from './stores.js';

  $: activeUsers = $users.filter((u) => u.is_active);
  $: activeTags  = $tags.filter((t) => t.is_active !== false && t.is_active !== 0);

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
  let useSlider = true;
  let sliderVal = 50;

  function handleSliderInput(e) {
    const val = Math.round(parseFloat(e.target.value));
    sliderVal = val;
    if (activeUsers.length === 2) {
      const u0 = activeUsers[0].name;
      const u1 = activeUsers[1].name;
      overridePcts[u0] = val;
      overridePcts[u1] = 100 - val;
    }
  }

  // Sync sliderVal from overridePcts if modified in manual mode or initialized
  $: if (customSplit && activeUsers.length === 2) {
    const val = overridePcts[activeUsers[0].name];
    if (val !== undefined && val !== sliderVal) {
      sliderVal = Math.round(val);
    }
  }

  // Initialise override inputs whenever activeUsers changes
  $: if (customSplit && activeUsers.length > 0) {
    const n = activeUsers.length;
    const base = Math.floor(100 / n);
    const rem = 100 - (base * n);
    activeUsers.forEach((u, i) => {
      if (!(u.name in overridePcts)) {
        overridePcts[u.name] = base + (i < rem ? 1 : 0);
      }
    });
  }

  $: overrideSum = Object.values(overridePcts).reduce((s, v) => s + (parseInt(v, 10) || 0), 0);
  $: overrideOk  = overrideSum === 100;

  // ── Form state ──────────────────────────────────────────────────────────────────
  let name         = '';
  let costEuros    = '';       // user-facing input, e.g. "12.50"
  let expenseDate  = today();
  let paidByJoint  = $defaultPayer === 'Joint Account';
  let whoPaid      = $defaultPayer === 'Joint Account' ? '' : $defaultPayer;
  let category     = $defaultCategory;
  let projectId    = $defaultProject ? Number($defaultProject) : null;
  let tagId        = null;     // optional: link expense to a tag

  let submitting   = false;
  let submitSuccess = false; // brief checkmark state on the button
  let errorMsg     = null;
  let successName  = null;
  let successTimer = null;

  // Reactive project derived values
  $: selectedProject = $projects.find((p) => p.id === Number(projectId)) || null;
  $: hideCategory = selectedProject && selectedProject.allow_subcategories === false;

  // When selected project is linked to joint account, auto-select Joint Account as default payer
  $: if (selectedProject && selectedProject.is_joint) {
    paidByJoint = true;
  }

  // Plain joint category names set for warning check
  $: jointCategorySet = new Set(($jointCategories || []).map((c) => (typeof c === 'string' ? c : c.plain)));
  $: isUncoupledJointCategory = paidByJoint && category && !jointCategorySet.has(category);

  function reset() {
    name        = '';
    costEuros   = '';
    expenseDate = today();
    paidByJoint = $defaultPayer === 'Joint Account';
    whoPaid     = $defaultPayer === 'Joint Account' ? '' : $defaultPayer;
    category    = $defaultCategory;
    projectId   = $defaultProject ? Number($defaultProject) : null;
    tagId       = null;
    errorMsg    = null;
    customSplit = false;
    overridePcts = {};
    useSlider   = true;
    sliderVal   = 50;
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
    if (!paidByJoint && !whoPaid) {
      errorMsg = 'Please select who paid.';
      return;
    }

    // 4. Category validation (if subcategories enabled)
    let finalCategory = category;
    if (hideCategory) {
      finalCategory = $defaultCategory || ($splits[0]?.category) || 'General';
    } else if (!finalCategory) {
      errorMsg = 'Please select a category.';
      return;
    }

    submitting = true;
    try {
      const payload = {
        name:         name.trim(),
        cost_cents:   costCents,
        expense_date: expenseDate,
        who_paid:     paidByJoint ? (activeUsers[0]?.name ?? 'John') : whoPaid,
        category:     finalCategory,
        project_id:   projectId ? Number(projectId) : null,
        tag_id:       tagId ? Number(tagId) : null,
        is_joint:     paidByJoint,
      };
      if (!paidByJoint && customSplit && overrideOk) {
        payload.overrides = Object.entries(overridePcts).map(([user_name, pct]) => ({
          user_name,
          pct: Math.round(parseFloat(pct)),
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

  <!-- 1. Description (Name) -->
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
      class="input-field"
    />
  </div>

  <!-- 2. Amount -->
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
        class="input-field pl-7"
      />
    </div>
  </div>

  <!-- 3. Date -->
  <div>
    <label for="expense-date" class="block text-xs font-medium text-neutral-400 mb-1.5">
      Date
    </label>
    <input
      id="expense-date"
      type="date"
      bind:value={expenseDate}
      class="input-field"
    />
  </div>

  <!-- 4. Project (above categories, shown only if projects exist and feature enabled) -->
  {#if $showProjectsInExpense && $projects.length > 0}
    <div>
      <label for="expense-project" class="block text-xs font-medium text-neutral-400 mb-1.5">
        Project <span class="text-neutral-600">(optional)</span>
      </label>
      <select
        id="expense-project"
        bind:value={projectId}
        class="select-field"
      >
        <option value={null}>— No project —</option>
        {#each $projects as p}
          <option value={p.id}>{p.name}{p.is_joint ? ' 🏦' : ''}</option>
        {/each}
      </select>
    </div>
  {/if}

  <!-- 5. Category (dropdown, hidden if project disables subcategories) -->
  {#if !hideCategory}
    <div>
      <label for="expense-category" class="block text-xs font-medium text-neutral-400 mb-1.5">
        Category
      </label>
      <select
        id="expense-category"
        bind:value={category}
        class="select-field"
      >
        <option value="">— Select category —</option>
        {#each $splits as split}
          <option value={split.category}>{split.category}</option>
        {/each}
      </select>
    </div>
  {/if}

  <!-- 6. Tag (dropdown) — only visible when active tags exist -->
  {#if activeTags.length > 0}
    <div>
      <label for="expense-tag" class="block text-xs font-medium text-neutral-400 mb-1.5">
        Tag <span class="text-neutral-600">(optional)</span>
      </label>
      <select
        id="expense-tag"
        bind:value={tagId}
        class="select-field"
      >
        <option value={null}>— No tag —</option>
        {#each activeTags as t}
          <option value={t.id}>
            {t.name}{t.total_amount > 0 ? ` (${t.total_amount.toLocaleString('en-GB', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} total)` : ''}
          </option>
        {/each}
      </select>
    </div>
  {/if}

  <!-- 7. Who paid (Final field) -->
  <div>
    <p id="who-paid-label" class="block text-xs font-medium text-neutral-400 mb-2.5">Who paid?</p>
    <div class="flex flex-wrap gap-3">
      {#each activeUsers as u (u.name)}
        <label class="flex items-center gap-2.5 text-sm text-neutral-300 cursor-pointer select-none hover:text-white transition-colors group">
          <div class="relative flex items-center justify-center">
            <input
              type="checkbox"
              id="who-paid-{u.name.toLowerCase()}"
              checked={whoPaid === u.name && !paidByJoint}
              on:change={() => {
                whoPaid = whoPaid === u.name ? '' : u.name;
                paidByJoint = false;
              }}
              class="sr-only"
            />
            <div
              class="w-5 h-5 rounded border transition-all duration-150 flex items-center justify-center
                     {whoPaid === u.name && !paidByJoint ? 'text-white shadow-sm' : 'bg-neutral-800 border-neutral-700 group-hover:border-neutral-500'}"
              style={whoPaid === u.name && !paidByJoint ? `background-color:${u.color};border-color:${u.color}` : ''}
            >
              {#if whoPaid === u.name && !paidByJoint}
                <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
                </svg>
              {/if}
            </div>
          </div>
          <span>{u.name}</span>
        </label>
      {/each}

      {#if $jointAccountEnabled || $jointAccount}
        <!-- Paid by Joint Account option -->
        <label class="flex items-center gap-2.5 text-sm text-indigo-300 cursor-pointer select-none hover:text-indigo-200 transition-colors group border border-indigo-500/30 bg-indigo-500/10 px-3 py-1.5 rounded-lg">
          <div class="relative flex items-center justify-center">
            <input
              type="checkbox"
              id="paid-by-joint-checkbox"
              checked={paidByJoint}
              on:change={() => {
                paidByJoint = !paidByJoint;
                if (paidByJoint && !whoPaid && activeUsers.length > 0) {
                  whoPaid = activeUsers[0].name;
                }
              }}
              class="sr-only"
            />
            <div
              class="w-5 h-5 rounded border transition-all duration-150 flex items-center justify-center
                     {paidByJoint ? 'bg-indigo-600 border-indigo-500 text-white shadow-sm' : 'bg-neutral-800 border-neutral-700 group-hover:border-neutral-500'}"
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
  </div>

  <!-- Warning for uncoupled joint category -->
  {#if isUncoupledJointCategory}
    <div class="p-2.5 bg-amber-950/40 border border-amber-800/60 text-amber-300 rounded-lg text-xs flex items-center gap-2">
      <span class="text-amber-400 font-bold">⚠️</span>
      <span>Category &ldquo;<strong>{category}</strong>&rdquo; is not in the coupled Joint Account categories list. The expense will still be saved to the joint account.</span>
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
        {#if activeUsers.length === 2 && useSlider}
          <!-- Slider split view -->
          <div class="flex items-center justify-between gap-4 py-2">
            <div class="text-right w-24 flex-shrink-0">
              <span class="text-xs font-semibold block truncate" style="color: {activeUsers[0].color}">
                {activeUsers[0].name}
              </span>
              <span class="text-lg font-bold text-neutral-100">{Math.round(overridePcts[activeUsers[0].name] ?? 50)}%</span>
            </div>

            <div class="flex-1 relative flex items-center">
              <input
                type="range"
                min="0"
                max="100"
                step="1"
                value={sliderVal}
                on:input={handleSliderInput}
                class="w-full h-2 bg-neutral-800 rounded-lg appearance-none cursor-pointer accent-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500/50"
                style="background: linear-gradient(to right, {activeUsers[0].color} 0%, {activeUsers[0].color} {sliderVal}%, {activeUsers[1].color} {sliderVal}%, {activeUsers[1].color} 100%);"
              />
            </div>

            <div class="text-left w-24 flex-shrink-0">
              <span class="text-xs font-semibold block truncate" style="color: {activeUsers[1].color}">
                {activeUsers[1].name}
              </span>
              <span class="text-lg font-bold text-neutral-100">{Math.round(overridePcts[activeUsers[1].name] ?? 50)}%</span>
            </div>
          </div>

          <div class="pt-1 flex justify-between items-center text-[10px] text-neutral-500 border-t border-neutral-850 pt-2 mt-1">
            <button
              type="button"
              on:click={() => useSlider = false}
              class="hover:text-neutral-300 transition-colors underline"
            >
              Switch to manual inputs
            </button>
            <button
              type="button"
              on:click={() => {
                sliderVal = 50;
                overridePcts[activeUsers[0].name] = 50;
                overridePcts[activeUsers[1].name] = 50;
              }}
              class="hover:text-neutral-300 transition-colors"
            >
              Reset to 50/50
            </button>
          </div>
        {:else}
          <!-- Manual / N-user split inputs -->
          {#each activeUsers as u (u.name)}
            <div class="flex items-center gap-3">
              <span class="text-xs font-semibold w-14 truncate" style="color: {u.color}">{u.name}</span>
              <input
                type="number" min="0" max="100" step="1"
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
              Sum: {overrideSum}%
            </span>
          </div>

          {#if activeUsers.length === 2}
            <div class="pt-1 flex justify-start border-t border-neutral-850 pt-2 mt-1">
              <button
                type="button"
                on:click={() => {
                  useSlider = true;
                  sliderVal = overridePcts[activeUsers[0].name] || 50;
                }}
                class="text-[10px] text-neutral-500 hover:text-neutral-300 transition-colors underline"
              >
                Switch to slider
              </button>
            </div>
          {/if}
        {/if}
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
