<script>
  /**
   * SplitManager.svelte
   *
   * Lets the user edit split percentages per category for any number of
   * household members.  Each category row shows one input per active user;
   * validation requires the inputs to sum to 100 before saving.
   *
   * Salary section at the top:
   *   - One input per active user, pre-filled from their latest SALARY entry.
   *   - "Save Income" inserts a new income row for each user with a value > 0.
   *   - "Reset to salary ratio" distributes each category proportionally to
   *     the entered salaries.
   */

  import { createEventDispatcher, onMount } from 'svelte';
  import * as api from './api.js';
  import { splits, selectedMonth, users, currencySymbol, splitInputMode, jointCategories, jointAccounts } from './stores.js';

  const dispatch = createEventDispatcher();
  let isMobile = false;

  /** Set of plain category names assigned to joint account */
  $: jointCategorySet = new Set(($jointCategories || []).map((c) => (typeof c === 'string' ? c : c.plain)));

  /** Categories where the payer always bears 100% — percentages are irrelevant. */
  const PERSONAL_PAY = new Set(['PERSONAL COST', 'GIFT', 'LEISURE']);

  function uniqueByCategory(list) {
    const seen = new Set();
    return list.filter((s) => {
      if (!s || !s.category || seen.has(s.category)) return false;
      seen.add(s.category);
      return true;
    });
  }

  $: activeUsers    = $users.filter((u) => u.is_active);
  $: variableSplits = uniqueByCategory($splits.filter((s) => !PERSONAL_PAY.has(s.category)));
  $: personalSplits = uniqueByCategory($splits.filter((s) =>  PERSONAL_PAY.has(s.category)));

  // ── Salary inputs ─────────────────────────────────────────────────────────
  /** { [userName]: euroAmount } */
  let salaryValues  = {};
  let salaryLoading = false;
  let salarySaving  = false;
  let salarySuccess = false;
  let salaryError   = null;

  import { incomeAnalytics, incomeCategories } from './stores.js';

  let newCategoryName = '';
  let newCatType = 'expense'; // 'expense' | 'income'
  let creatingCategory = false;
  let createCategoryError = '';
  let deletingIncomeCat = '';

  async function handleAddCategory() {
    if (!newCategoryName.trim()) return;
    const cat = newCategoryName.trim().toUpperCase();
    creatingCategory = true;
    createCategoryError = '';
    try {
      if (newCatType === 'expense') {
        await api.createSplit({ category: cat, allocations: [] });
      } else {
        await api.createIncomeCategory(cat);
      }
      newCategoryName = '';
    } catch (err) {
      createCategoryError = err.message || 'Failed to create category.';
    } finally {
      creatingCategory = false;
    }
  }

  async function handleRemoveIncomeCategory(name) {
    deletingIncomeCat = name;
    try {
      await api.deleteIncomeCategory(name);
    } catch (err) {
      console.error(err);
    } finally {
      deletingIncomeCat = '';
    }
  }

  $: {
    const fresh = { ...salaryValues };
    let changed = false;
    for (const u of activeUsers) {
      if (!(u.name in fresh)) {
        fresh[u.name] = 0;
        changed = true;
      }
    }
    for (const item of $incomeAnalytics || []) {
      if (item && item.who) {
        const val = (item.salary_cents || item.amount_cents || item.total_cents || 0) / 100;
        if (fresh[item.who] !== val) {
          fresh[item.who] = val;
          changed = true;
        }
      }
    }
    if (changed) salaryValues = fresh;
  }

  $: totalSalary = activeUsers.reduce((sum, u) => sum + (Number(salaryValues[u.name]) || 0), 0);

  /**
   * Calculate integer percentage split allocations based on salary ratios.
   * Uses Largest Remainder Method (Hamilton Method) with top-earner tie-breaker
   * for exact middle remainders.
   */
  function calculateSalaryRatios(usersList, salariesDict) {
    if (!usersList || usersList.length === 0) return {};
    const n = usersList.length;
    const total = usersList.reduce((sum, u) => sum + (Number(salariesDict[u.name]) || 0), 0);

    const items = usersList.map((u) => {
      const salary = Number(salariesDict[u.name]) || 0;
      const exactPct = total > 0 ? (salary / total) * 100 : 100 / n;
      const floor = Math.floor(exactPct);
      const remainder = exactPct - floor;
      return { user: u, salary, exactPct, floor, remainder };
    });

    const sumFloors = items.reduce((s, item) => s + item.floor, 0);
    let remainingPoints = 100 - sumFloors;

    // Sort candidates by remainder descending.
    // On tie in remainder (e.g. 0.5 vs 0.5), sort by salary descending (top earner first), then by name.
    items.sort((a, b) => {
      if (Math.abs(b.remainder - a.remainder) > 1e-9) {
        return b.remainder - a.remainder;
      }
      if (b.salary !== a.salary) {
        return b.salary - a.salary;
      }
      return a.user.name.localeCompare(b.user.name);
    });

    const result = {};
    items.forEach((item, idx) => {
      const extra = idx < remainingPoints ? 1 : 0;
      result[item.user.name] = item.floor + extra;
    });

    return result;
  }

  $: salaryRatios = calculateSalaryRatios(activeUsers, salaryValues);

  /** Compute salary-implied percentage for a given user (always whole integer). */
  function salaryPct(userName) {
    return salaryRatios[userName] ?? 0;
  }

  onMount(async () => {
    const mq = window.matchMedia('(max-width: 767px)');
    isMobile = mq?.matches ?? false;
    mq?.addEventListener?.('change', (e) => { isMobile = e.matches; });

    api.fetchIncomeCategories().catch(console.error);

    try {
      salaryLoading = true;
      let latest = [];
      try {
        latest = await api.fetchLatestSalaries();
      } catch {
        latest = await api.fetchIncomeByPerson($selectedMonth);
      }
      const fresh = { ...salaryValues };
      for (const row of latest || []) {
        if (row && row.who) {
          fresh[row.who] = (row.amount_cents || row.salary_cents || row.total_cents || 0) / 100;
        }
      }
      salaryValues = fresh;
    } catch (err) {
      console.error("ONMOUNT SALARY ERROR:", err);
    } finally {
      salaryLoading = false;
    }
  });

  function firstOfMonth() {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-01`;
  }

  async function saveSalaries() {
    salaryError   = null;
    salarySuccess = false;
    const entries = [];
    for (const u of activeUsers) {
      const cents = Math.round((Number(salaryValues[u.name]) || 0) * 100);
      if (cents > 0) {
        entries.push({
          name:         `Salary ${u.name}`,
          amount_cents: cents,
          who:          u.name,
          category:     'SALARY',
          income_date:  firstOfMonth(),
        });
      }
    }
    if (entries.length === 0) {
      salaryError = 'Enter at least one salary value greater than zero.';
      return;
    }
    salarySaving = true;
    try {
      await createIncome(entries, $selectedMonth);
      salarySuccess = true;
      setTimeout(() => { salarySuccess = false; }, 3000);
    } catch (err) {
      salaryError = err.message ?? 'Save failed.';
    } finally {
      salarySaving = false;
    }
  }

  // ── Per-row edit state ─────────────────────────────────────────────────────
  /** { [category]: { [userName]: pctString } } */
  let editValues = {};
  let saving     = {};
  let rowError   = {};
  let rowSuccess = {};


  /** Initialise edit state for a category from the split's current allocations. */
  function initEditValues(split) {
    if (split.category in editValues) return;
    const storedAllocs = split.allocations ?? [];
    const entry = {};

    if (storedAllocs.length > 0) {
      // Use stored values from the database
      for (const alloc of storedAllocs) {
        entry[alloc.user_name] = String(Math.round(alloc.pct));
      }
      // Fill in any active users missing from stored allocs with 0
      for (const u of activeUsers) {
        if (!(u.name in entry)) entry[u.name] = '0';
      }
    } else {
      // No stored allocations — default to an equal split using LRM so
      // integer percentages sum to exactly 100 and Save is immediately active.
      const n = activeUsers.length;
      if (n === 0) {
        // no-op
      } else {
        const exact = 100 / n;
        const floor = Math.floor(exact);
        const remainder = exact - floor;
        const sumFloors = floor * n;
        let extra = 100 - sumFloors; // number of users that get floor+1

        // Give the extra point(s) to users in natural order
        activeUsers.forEach((u, idx) => {
          entry[u.name] = String(floor + (idx < extra ? 1 : 0));
        });
      }
    }

    editValues[split.category] = entry;
  }

  $: {
    for (const s of variableSplits) initEditValues(s);
  }

  /** Sum of the currently-entered percentages for a category. */
  function rowSum(category, values) {
    const vals = values[category] ?? {};
    return Object.values(vals).reduce((acc, v) => acc + (parseInt(v, 10) || 0), 0);
  }

  /** Reset all percentages for a category to the salary-implied ratio. */
  function resetToSalary(category) {
    if (!editValues[category]) return;
    const fresh = {};
    const ratios = calculateSalaryRatios(activeUsers, salaryValues);
    for (const u of activeUsers) {
      fresh[u.name] = String(ratios[u.name] ?? 0);
    }
    editValues[category] = fresh;
    editValues = { ...editValues };
  }

  /** Split equally among all active members */
  function applyEvenSplit(cat) {
    if (!editValues[cat]) return;
    const n = activeUsers.length;
    if (n === 0) return;
    const exact = 100 / n;
    const floor = Math.floor(exact);
    const extra = 100 - (floor * n);
    const fresh = {};
    activeUsers.forEach((u, idx) => {
      fresh[u.name] = String(floor + (idx < extra ? 1 : 0));
    });
    editValues[cat] = fresh;
    editValues = { ...editValues };
  }

  /** Split 50/50 (or equal) between specific members (e.g. couple), with 0% for others */
  function applyCoupleSplit(cat, memberNames) {
    if (!editValues[cat]) return;
    const set = new Set(memberNames);
    const matched = activeUsers.filter((u) => set.has(u.name));
    if (matched.length === 0) return;
    const n = matched.length;
    const exact = 100 / n;
    const floor = Math.floor(exact);
    const extra = 100 - (floor * n);
    const fresh = {};
    activeUsers.forEach((u) => {
      if (set.has(u.name)) {
        const idx = matched.findIndex((m) => m.name === u.name);
        fresh[u.name] = String(floor + (idx < extra ? 1 : 0));
      } else {
        fresh[u.name] = '0';
      }
    });
    editValues[cat] = fresh;
    editValues = { ...editValues };
  }

  async function save(category) {
    rowError[category]   = null;
    rowSuccess[category] = false;
    const vals = editValues[category] ?? {};
    const allocations = activeUsers.map((u) => {
      const parsed = Math.round(parseFloat(vals[u.name] ?? '0'));
      return {
        user_name: u.name,
        pct:       isNaN(parsed) ? 0 : parsed,
      };
    });
    const total = allocations.reduce((s, a) => s + a.pct, 0);
    if (total !== 100) {
      rowError[category] = `Percentages must sum to 100 (currently ${total}).`;
      return;
    }
    saving[category] = true;
    try {
      await api.updateSplit(category, { allocations });
      rowSuccess[category] = true;
      setTimeout(() => { rowSuccess[category] = false; }, 3000);
    } catch (err) {
      rowError[category] = err.message ?? 'Save failed.';
    } finally {
      saving[category] = false;
    }
  }
</script>

<div class="space-y-6">

  <!-- ── Salary header (Read-only entries with link to Income tab) ── -->
  <div class="bg-neutral-800/60 border border-neutral-700/60 rounded-xl p-4">
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 mb-3">
      <div>
        <p class="text-xs font-semibold text-neutral-400 uppercase tracking-wider">Current Monthly Salaries</p>
        <p class="text-xs text-neutral-500 mt-0.5">Salaries are defined in the Income tab and determine split ratios.</p>
      </div>
      <button
        id="link-manage-income"
        on:click={() => dispatch('navigateIncome')}
        class="px-3 py-1.5 rounded-lg text-xs font-semibold bg-indigo-950/80 text-indigo-300 border border-indigo-700/60 hover:bg-indigo-900 transition-colors flex items-center gap-1.5 self-start sm:self-auto cursor-pointer"
      >
        <span>Manage in Income Tab</span>
        <span>→</span>
      </button>
    </div>

    {#if salaryLoading}
      <p class="text-xs text-neutral-500">Loading salaries…</p>
    {:else}
      <div class="flex flex-wrap gap-4 items-center">
        {#each activeUsers as u (u.name)}
          <div class="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-neutral-900 border border-neutral-700/80">
            <div
              class="w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold shrink-0 text-white"
              style="background-color: {u.color}"
            >{u.name.charAt(0).toUpperCase()}</div>
            <span class="text-xs font-medium" style="color: {u.color}">{u.name}:</span>
            <span class="text-xs font-bold text-neutral-100 tabular-nums">
              {$currencySymbol}{(salaryValues[u.name] || 0).toFixed(2)}
            </span>
          </div>
        {/each}

        {#if totalSalary > 0 && activeUsers.length >= 2}
          <div class="flex items-center gap-1 text-[11px] text-neutral-400 ml-auto bg-neutral-900/60 px-2.5 py-1 rounded-md border border-neutral-800">
            <span class="text-neutral-500">Salary Ratio:</span>
            {#each activeUsers as u, i}
              <span class="font-semibold" style="color: {u.color}">{salaryPct(u.name)}%</span>
              {#if i < activeUsers.length - 1}<span>/</span>{/if}
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- ── Unified Add Category ────────────────────────────────────────────────── -->
  <div class="mb-6 card-sub space-y-3">
    <div class="flex items-end gap-3 flex-wrap">
      <div class="flex-1 min-w-[200px]">
        <label for="new-category" class="block text-xs font-semibold text-neutral-400 uppercase tracking-wider mb-1.5">New Category Name</label>
        <input
          id="new-category"
          type="text"
          bind:value={newCategoryName}
          placeholder={newCatType === 'expense' ? 'e.g. SUBSCRIPTIONS' : 'e.g. FREELANCE'}
          class="input-field uppercase"
          on:keydown={(e) => e.key === 'Enter' && handleAddCategory()}
        />
      </div>

      <!-- Type toggle -->
      <div class="flex flex-col">
        <span class="block text-xs font-semibold text-neutral-400 uppercase tracking-wider mb-1.5">Category Type</span>
        <div class="inline-flex rounded-xl bg-neutral-950 p-1 border border-neutral-800 h-[42px] items-center">
          <button
            id="cat-type-expense"
            type="button"
            on:click={() => newCatType = 'expense'}
            class="px-3 py-1.5 rounded-lg text-xs font-semibold transition-colors cursor-pointer {newCatType === 'expense' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-400 hover:text-neutral-200'}"
          >
            Expense
          </button>
          <button
            id="cat-type-income"
            type="button"
            on:click={() => newCatType = 'income'}
            class="px-3 py-1.5 rounded-lg text-xs font-semibold transition-colors cursor-pointer {newCatType === 'income' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-400 hover:text-neutral-200'}"
          >
            Income
          </button>
        </div>
      </div>

      <button
        id="add-category-btn"
        on:click={handleAddCategory}
        disabled={creatingCategory || !newCategoryName.trim()}
        class="btn-primary h-[42px] self-end"
      >
        {creatingCategory ? 'Adding...' : 'Add Category'}
      </button>
    </div>
    {#if createCategoryError}
      <p class="mt-1 text-xs text-red-400 bg-red-950/40 border border-red-800 rounded-xl px-3 py-2">{createCategoryError}</p>
    {/if}
  </div>

  <!-- ── Subsection 1: Expense Categories & Splits ────────────────────────────── -->
  <div class="pt-2">
    <h3 class="text-xs font-bold text-neutral-400 uppercase tracking-wider mb-3 flex items-center gap-2">
      <span>💸</span> Expense Categories & Household Split Allocations
    </h3>

    {#if $splitInputMode === 'slider' && activeUsers.length !== 2}
      <div class="mb-3 px-3.5 py-2.5 rounded-xl bg-amber-950/40 border border-amber-800/50 text-amber-300 text-xs flex items-center gap-2">
        <span class="text-amber-400 font-bold">ℹ️</span>
        <span>Slider mode requires exactly 2 active users. Percentage inputs are shown below for {activeUsers.length} active household members.</span>
      </div>
    {/if}

  <!-- ── Splits table ───────────────────────────────────────────────────────── -->
  <div class="space-y-1">
    <!-- Dynamic header row -->
    <div class="grid gap-3 px-1 mb-3" style="grid-template-columns: minmax(80px,1fr) {activeUsers.map(() => 'minmax(70px,1fr)').join(' ')} auto">
      <span class="text-xs font-medium text-neutral-500 uppercase tracking-wider">Category</span>
      {#each activeUsers as u}
        <span class="text-xs font-medium uppercase tracking-wider" style="color: {u.color}">{u.name} %</span>
      {/each}
      <span></span>
    </div>

    {#each variableSplits as split, i (split.category + '_' + i)}
      {#if editValues[split.category]}
        {@const sum = rowSum(split.category, editValues)}
        {@const sumOk = sum === 100}
        {@const isJointCategory = jointCategorySet.has(split.category)}

        <div class="group px-1 py-3 rounded-xl hover:bg-neutral-800/50 transition-colors">
          {#if $splitInputMode === 'slider' && activeUsers.length === 2}
              <!-- ── Slider mode (2-user households) ── -->
              {@const sliderVal = Math.round(parseFloat(editValues[split.category][activeUsers[0].name] || '0'))}
              <div class="flex items-center gap-3">
                <!-- Category badge -->
                <div class="min-w-[120px] flex items-center gap-1 flex-wrap">
                  <span class="inline-flex items-center px-2.5 py-1 rounded-lg bg-neutral-800 border border-neutral-700 text-xs text-neutral-300 font-medium truncate max-w-full">
                    {split.category}
                  </span>
                  {#if isJointCategory}
                    <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] bg-indigo-950/80 text-indigo-300 border border-indigo-700/60 font-semibold" title="Category is managed by the Joint Account">
                      🏦 Joint Account (Locked)
                    </span>
                  {/if}
                </div>

                <!-- Slider column -->
                <div class="flex-1 min-w-0">
                  <div class="flex justify-between text-xs font-semibold tabular-nums mb-1.5">
                    <span style="color: {activeUsers[0].color}">
                      {activeUsers[0].name}: {Math.round(parseFloat(editValues[split.category][activeUsers[0].name] || '0'))}%
                    </span>
                    <span style="color: {activeUsers[1].color}">
                      {activeUsers[1].name}: {Math.round(parseFloat(editValues[split.category][activeUsers[1].name] || '0'))}%
                    </span>
                  </div>
                  <input
                    id="slider-{split.category}"
                    type="range" min="0" max="100" step="1"
                    value={sliderVal}
                    disabled={isJointCategory}
                    on:input={(e) => {
                      const val = Math.round(parseFloat(e.target.value));
                      editValues[split.category][activeUsers[0].name] = String(val);
                      editValues[split.category][activeUsers[1].name] = String(100 - val);
                      editValues = {...editValues};
                    }}
                    class="w-full h-2 rounded-full cursor-pointer slider-split disabled:opacity-40 disabled:cursor-not-allowed"
                    style="background: linear-gradient(to right, {activeUsers[0].color} {sliderVal}%, {activeUsers[1].color} {sliderVal}%)"
                  />
                </div>

                <!-- Actions -->
                <div class="flex flex-col items-end gap-1 min-w-[80px]">
                  <div class="flex gap-1.5">
                    <button
                      id="reset-split-{split.category}"
                      on:click={() => resetToSalary(split.category)}
                      disabled={isJointCategory || totalSalary === 0}
                      title={isJointCategory ? 'Locked — Category is managed by Joint Account' : totalSalary === 0 ? 'Enter salaries above to enable reset' : 'Reset to salary ratio'}
                      class="px-2.5 py-1.5 rounded-lg text-xs font-semibold
                             bg-neutral-700 hover:bg-neutral-600 disabled:opacity-30 disabled:cursor-not-allowed
                             transition-colors active:scale-95"
                    >Reset</button>
                    <button
                      id="save-split-{split.category}"
                      on:click={() => save(split.category)}
                      disabled={isJointCategory || saving[split.category] || !sumOk}
                      title={isJointCategory ? 'Locked — Category is managed by Joint Account' : ''}
                      class="px-3 py-1.5 rounded-lg text-xs font-semibold
                             bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 disabled:cursor-not-allowed
                             transition-colors active:scale-95"
                    >
                      {saving[split.category] ? '…' : 'Save'}
                    </button>
                  </div>
                  {#if rowSuccess[split.category]}
                    <span class="text-[10px] text-emerald-400">Saved ✓</span>
                  {/if}
                  {#if rowError[split.category]}
                    <span class="text-[10px] text-red-400">{rowError[split.category]}</span>
                  {/if}
                </div>
              </div>
            {:else}
              <!-- ── Editable row (inputs mode, or >2 users fallback) ── -->
              <div class="grid gap-3 items-center" style="grid-template-columns: minmax(80px,1fr) {activeUsers.map(() => 'minmax(70px,1fr)').join(' ')} auto">

              <!-- Category badge & presets -->
              <div class="flex flex-col gap-1">
                <div class="flex items-center gap-1 flex-wrap">
                  <span class="inline-flex items-center px-2.5 py-1 rounded-lg bg-neutral-800 border border-neutral-700 text-xs text-neutral-300 font-medium truncate max-w-full">
                    {split.category}
                  </span>
                  {#if isJointCategory}
                    <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] bg-indigo-950/80 text-indigo-300 border border-indigo-700/60 font-semibold" title="Category is managed by the Joint Account">
                      🏦 Joint Account (Locked)
                    </span>
                  {/if}
                </div>

                <!-- Presets for multi-member households -->
                {#if activeUsers.length > 2 && !isJointCategory}
                  <div class="flex items-center gap-1 text-[10px] flex-wrap mt-0.5">
                    <span class="text-neutral-500 text-[9px] uppercase font-semibold">Presets:</span>
                    <button
                      type="button"
                      on:click={() => applyEvenSplit(split.category)}
                      class="px-1.5 py-0.5 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-300 text-[10px] font-medium transition-colors"
                      title="Split equally ({Math.floor(100 / activeUsers.length)}% each)"
                    >
                      Even ({Math.floor(100 / activeUsers.length)}%)
                    </button>
                    {#each $jointAccounts as acc}
                      {#if acc.member_names && acc.member_names.length > 0 && acc.member_names.length < activeUsers.length}
                        <button
                          type="button"
                          on:click={() => applyCoupleSplit(split.category, acc.member_names)}
                          class="px-1.5 py-0.5 rounded bg-indigo-950/70 hover:bg-indigo-900/80 text-indigo-300 border border-indigo-800/50 text-[10px] font-medium transition-colors"
                          title="Split among {acc.name} members ({acc.member_names.join(' & ')})"
                        >
                          🏦 {acc.name} ({acc.member_names.join('+')})
                        </button>
                      {/if}
                    {/each}
                  </div>
                {/if}
              </div>

              <!-- One input per active user -->
              {#each activeUsers as u}
                <div class="flex items-center gap-1">
                  <input
                    id="split-{u.name}-{split.category}"
                    type="number" min="0" max="100" step="1"
                    disabled={isJointCategory}
                    bind:value={editValues[split.category][u.name]}
                    class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-2 py-2 text-sm
                           font-semibold tabular-nums text-neutral-200 disabled:opacity-40 disabled:cursor-not-allowed
                           focus:outline-none focus:ring-1 transition-colors"
                    style="--tw-ring-color: {u.color}; border-color: {editValues[split.category][u.name] !== '0' ? u.color + '60' : ''}"
                  />
                  <span class="text-xs text-neutral-500 shrink-0">%</span>
                </div>
              {/each}

              <!-- Actions -->
              <div class="flex flex-col items-end gap-1 min-w-[80px]">
                <div class="flex gap-1.5">
                  <button
                    id="reset-split-{split.category}"
                    on:click={() => resetToSalary(split.category)}
                    disabled={isJointCategory || totalSalary === 0}
                    title={isJointCategory ? 'Locked — Category is managed by Joint Account' : totalSalary === 0 ? 'Enter salaries above to enable reset' : 'Reset to salary ratio'}
                    class="px-2.5 py-1.5 rounded-lg text-xs font-semibold
                           bg-neutral-700 hover:bg-neutral-600 disabled:opacity-30 disabled:cursor-not-allowed
                           transition-colors active:scale-95"
                  >Reset</button>
                  <button
                    id="save-split-{split.category}"
                    on:click={() => save(split.category)}
                    disabled={isJointCategory || saving[split.category] || !sumOk}
                    title={isJointCategory ? 'Locked — Category is managed by Joint Account' : ''}
                    class="px-3 py-1.5 rounded-lg text-xs font-semibold
                           bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 disabled:cursor-not-allowed
                           transition-colors active:scale-95"
                  >
                    {saving[split.category] ? '…' : 'Save'}
                  </button>
                </div>
                {#if rowSuccess[split.category]}
                  <span class="text-[10px] text-emerald-400">Saved ✓</span>
                {/if}
                {#if rowError[split.category]}
                  <span class="text-[10px] text-red-400">{rowError[split.category]}</span>
                {/if}
              </div>
            </div>
          {/if}

          <!-- Sum indicator bar -->
          <div class="mt-2 mx-1">
            <div class="h-1.5 rounded-full bg-neutral-800 overflow-hidden">
              <div
                class="h-full rounded-full transition-all duration-300 {sumOk ? 'bg-emerald-500' : sum > 100 ? 'bg-red-500' : 'bg-amber-500'}"
                style="width: {Math.min(sum, 100)}%"
              ></div>
            </div>
            {#if !sumOk}
              <p class="text-[10px] text-amber-400 mt-0.5 text-right">sum = {sum}%</p>
            {/if}
          </div>
        </div>
      {/if}
    {/each}

    {#if variableSplits.length === 0}
      <p class="text-neutral-500 text-sm text-center py-8">No variable split categories found.</p>
    {/if}
  </div>

  <!-- ── Personal-pay categories (read-only) ─────────────────────────────── -->
  {#if personalSplits.length > 0}
    <div class="mt-6 border border-neutral-800 rounded-xl overflow-hidden">
      <div class="flex items-center gap-2.5 px-4 py-3 bg-neutral-800/40 border-b border-neutral-800">
        <span class="text-base leading-none">🔒</span>
        <div>
          <p class="text-xs font-semibold text-neutral-300">Self-pay categories</p>
          <p class="text-[11px] text-neutral-500 mt-0.5">
            Expenses in these categories are carried 100% by whoever paid — split percentages do not apply.
          </p>
        </div>
      </div>
      <div class="divide-y divide-neutral-800/60">
        {#each personalSplits as split, i (split.category + '_' + i)}
          <div class="flex items-center justify-between px-4 py-3 gap-4">
            <span class="inline-flex items-center px-2.5 py-1 rounded-lg bg-neutral-800/50 border border-neutral-700/50 text-xs text-neutral-400 font-medium">
              {split.category}
            </span>
            <span class="flex-1 text-[11px] text-neutral-500 hidden sm:block">
              100% borne by the person who logged the expense
            </span>
            <div class="flex items-center gap-2 flex-none flex-wrap">
              {#each activeUsers as u}
                <span class="text-[11px] tabular-nums font-semibold" style="color: {u.color}60">{u.name.charAt(0)}: varies</span>
              {/each}
              <span class="ml-1 text-[10px] px-2 py-0.5 rounded-full bg-neutral-800 border border-neutral-700 text-neutral-500 font-medium">locked</span>
            </div>
          </div>
        {/each}
      </div>
      <div class="px-4 py-3 bg-neutral-950/30 border-t border-neutral-800">
        <p class="text-[11px] text-neutral-600 leading-relaxed">
          When a member logs a <em class="text-neutral-500">PERSONAL COST</em>, <em class="text-neutral-500">GIFT</em>, or <em class="text-neutral-500">LEISURE</em> expense,
          they bear the full cost. No cross-transfer is generated for these categories.
        </p>
      </div>
    </div>
  {/if}
  </div>

  <!-- ── Subsection 2: Income Categories ────────────────────────────────────── -->
  <div class="mt-8 pt-6 border-t border-neutral-800">
    <h3 class="text-xs font-bold text-neutral-400 uppercase tracking-wider mb-4 flex items-center gap-2">
      <span>💰</span> Income Categories
    </h3>
    <div class="bg-neutral-900/50 border border-neutral-800 rounded-xl p-4">
      {#if $incomeCategories.length === 0}
        <p class="text-xs text-neutral-500">No income categories defined yet. Use the form above to add one.</p>
      {:else}
        <div class="flex flex-wrap gap-2">
          {#each $incomeCategories as c (c.category)}
            <div class="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-neutral-800 border border-neutral-700/80 text-xs font-semibold text-neutral-200">
              <span>{c.category}</span>
              <button
                id="delete-income-cat-{c.category}"
                on:click={() => handleRemoveIncomeCategory(c.category)}
                disabled={deletingIncomeCat === c.category}
                class="w-4 h-4 rounded flex items-center justify-center text-neutral-400 hover:text-red-400 hover:bg-red-950/40 transition-colors disabled:opacity-30"
                title="Remove income category"
              >
                {deletingIncomeCat === c.category ? '…' : '×'}
              </button>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>

</div>

<style>
  .slider-split {
    -webkit-appearance: none;
    appearance: none;
  }
  .slider-split::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: white;
    border: 2px solid rgba(99, 102, 241, 0.6);
    cursor: pointer;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
    transition: transform 0.15s ease;
  }
  .slider-split::-webkit-slider-thumb:hover {
    transform: scale(1.15);
  }
  .slider-split::-moz-range-thumb {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: white;
    border: 2px solid rgba(99, 102, 241, 0.6);
    cursor: pointer;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
  }
</style>
