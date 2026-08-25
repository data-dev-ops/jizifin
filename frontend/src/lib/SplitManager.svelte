<script>
  /**
   * SplitManager.svelte
   *
   * Subtab-Driven Categories & Household Split Agreements with full Light/Dark mode theming:
   *  - Subtabs:
   *      1. ⚖️ Split Agreements (Per-category editor with arbitrary subgroup selection, salary ratios, and quick presets)
   *      2. 📊 Split Matrix (High-level cross-member matrix & rules summary)
   *      3. ⚡ Batch Rule Builder (Apply split agreements across multiple categories for arbitrary member selections)
   *      4. 💰 Income Categories (Registry of income classification categories)
   *  - Multi-user arbitrary subgroup support (All members, Joint Account presets, or custom member chips)
   *  - Largest Remainder Method (Hamilton Method) for exact integer / 100% split distributions
   */

  import { createEventDispatcher, onMount } from 'svelte';
  import * as api from './api.js';
  import {
    splits,
    selectedMonth,
    users,
    currencySymbol,
    splitInputMode,
    jointCategories,
    jointAccounts,
    incomeAnalytics,
    incomeCategories,
  } from './stores.js';

  const dispatch = createEventDispatcher();

  // ── Navigation State ───────────────────────────────────────────────────────
  let section = 'agreements'; // 'agreements' | 'matrix' | 'batch' | 'income'

  // ── Active Household Data ──────────────────────────────────────────────────
  $: activeUsers = $users.filter((u) => u.is_active);

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

  $: variableSplits = uniqueByCategory($splits.filter((s) => !PERSONAL_PAY.has(s.category)));
  $: personalSplits = uniqueByCategory($splits.filter((s) => PERSONAL_PAY.has(s.category)));

  // ── Search & Filter State ──────────────────────────────────────────────────
  let categorySearch = '';
  $: filteredSplits = variableSplits.filter((s) =>
    s.category.toLowerCase().includes(categorySearch.trim().toLowerCase())
  );

  // ── Salary Inputs & Ratios ─────────────────────────────────────────────────
  /** { [userName]: euroAmount } */
  let salaryValues = {};
  let salaryLoading = false;

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
   * Uses Largest Remainder Method (Hamilton Method).
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
    const remainingPoints = 100 - sumFloors;

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

  function salaryPct(userName) {
    return salaryRatios[userName] ?? 0;
  }

  // ── Subgroup & Arbitrary User Selection for Agreements ──────────────────────
  let selectedSubgroupMode = 'all'; // 'all' | 'custom' | joint_account_id
  let customSelectedUsers = [];

  function selectSubgroup(mode) {
    selectedSubgroupMode = mode;
    if (mode === 'all') {
      customSelectedUsers = activeUsers.map((u) => u.name);
    } else if (typeof mode === 'number') {
      const acc = ($jointAccounts || []).find((a) => a.id === mode);
      if (acc && acc.member_names) {
        customSelectedUsers = [...acc.member_names];
      }
    }
  }

  function toggleCustomUser(userName) {
    if (customSelectedUsers.includes(userName)) {
      if (customSelectedUsers.length > 1) {
        customSelectedUsers = customSelectedUsers.filter((n) => n !== userName);
      }
    } else {
      customSelectedUsers = [...customSelectedUsers, userName];
    }
  }

  // Initialise customSelectedUsers with all users
  $: if (customSelectedUsers.length === 0 && activeUsers.length > 0) {
    customSelectedUsers = activeUsers.map((u) => u.name);
  }

  // ── Unified Add Category ──────────────────────────────────────────────────
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

  // ── Per-row edit state ─────────────────────────────────────────────────────
  /** { [category]: { [userName]: pctString } } */
  let editValues = {};
  let saving = {};
  let rowError = {};
  let rowSuccess = {};

  function initEditValues(split) {
    if (split.category in editValues) return;
    const storedAllocs = split.allocations ?? [];
    const entry = {};

    if (storedAllocs.length > 0) {
      for (const alloc of storedAllocs) {
        entry[alloc.user_name] = String(Math.round(alloc.pct));
      }
      for (const u of activeUsers) {
        if (!(u.name in entry)) entry[u.name] = '0';
      }
    } else {
      const n = activeUsers.length;
      if (n > 0) {
        const exact = 100 / n;
        const floor = Math.floor(exact);
        const sumFloors = floor * n;
        const extra = 100 - sumFloors;
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

  function rowSum(category, values) {
    const vals = values[category] ?? {};
    return Number(Object.values(vals).reduce((acc, v) => acc + (parseFloat(v) || 0), 0).toFixed(2));
  }

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

  function applyEvenSplit(cat) {
    if (!editValues[cat]) return;
    const n = activeUsers.length;
    if (n === 0) return;
    const exact = Number((100 / n).toFixed(2));
    const fresh = {};
    activeUsers.forEach((u) => {
      fresh[u.name] = String(exact);
    });
    editValues[cat] = fresh;
    editValues = { ...editValues };
  }

  function applySubgroupSplit(cat, memberNames, mode = 'even') {
    if (!editValues[cat]) return;
    const set = new Set(memberNames);
    const matched = activeUsers.filter((u) => set.has(u.name));
    if (matched.length === 0) return;

    const fresh = {};
    activeUsers.forEach((u) => {
      fresh[u.name] = '0';
    });

    if (mode === 'salary' && totalSalary > 0) {
      const ratios = calculateSalaryRatios(matched, salaryValues);
      matched.forEach((u) => {
        fresh[u.name] = String(ratios[u.name] ?? 0);
      });
    } else {
      const n = matched.length;
      const exact = 100 / n;
      const floor = Math.floor(exact);
      const extra = 100 - floor * n;
      matched.forEach((u, idx) => {
        fresh[u.name] = String(floor + (idx < extra ? 1 : 0));
      });
    }

    editValues[cat] = fresh;
    editValues = { ...editValues };
  }

  function setSinglePayer(cat, userName) {
    if (!editValues[cat]) return;
    const fresh = {};
    activeUsers.forEach((u) => {
      fresh[u.name] = u.name === userName ? '100' : '0';
    });
    editValues[cat] = fresh;
    editValues = { ...editValues };
  }

  async function save(category) {
    rowError[category] = null;
    rowSuccess[category] = false;
    const vals = editValues[category] ?? {};
    const allocations = activeUsers.map((u) => {
      const parsed = parseFloat(vals[u.name] ?? '0');
      return {
        user_name: u.name,
        pct: isNaN(parsed) ? 0 : Number(parsed.toFixed(4)),
      };
    });
    const total = Number(allocations.reduce((s, a) => s + a.pct, 0).toFixed(2));
    if (Math.abs(total - 100) > 0.05) {
      rowError[category] = `Percentages must sum to 100% (currently ${total}%).`;
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

  // ── Batch Rule Builder State ───────────────────────────────────────────────
  let batchSelectedCategories = [];
  let batchSelectedMembers = [];
  let batchStrategy = 'even'; // 'even' | 'salary' | 'single'
  let batchSinglePayer = '';
  let batchApplying = false;
  let batchMessage = '';
  let batchError = '';

  function toggleBatchCategory(cat) {
    if (batchSelectedCategories.includes(cat)) {
      batchSelectedCategories = batchSelectedCategories.filter((c) => c !== cat);
    } else {
      batchSelectedCategories = [...batchSelectedCategories, cat];
    }
  }

  function toggleBatchAllCategories() {
    if (batchSelectedCategories.length === variableSplits.length) {
      batchSelectedCategories = [];
    } else {
      batchSelectedCategories = variableSplits.map((s) => s.category);
    }
  }

  function toggleBatchMember(uName) {
    if (batchSelectedMembers.includes(uName)) {
      if (batchSelectedMembers.length > 1) {
        batchSelectedMembers = batchSelectedMembers.filter((m) => m !== uName);
      }
    } else {
      batchSelectedMembers = [...batchSelectedMembers, uName];
    }
  }

  $: if (batchSelectedMembers.length === 0 && activeUsers.length > 0) {
    batchSelectedMembers = activeUsers.map((u) => u.name);
    batchSinglePayer = activeUsers[0]?.name || '';
  }

  async function handleApplyBatch() {
    if (batchSelectedCategories.length === 0) {
      batchError = 'Please select at least one category.';
      return;
    }
    batchApplying = true;
    batchMessage = '';
    batchError = '';

    try {
      for (const cat of batchSelectedCategories) {
        if (jointCategorySet.has(cat)) continue; // Skip locked joint categories

        const matched = activeUsers.filter((u) => batchSelectedMembers.includes(u.name));
        const allocations = activeUsers.map((u) => ({ user_name: u.name, pct: 0 }));

        if (batchStrategy === 'single') {
          allocations.forEach((a) => {
            a.pct = a.user_name === batchSinglePayer ? 100 : 0;
          });
        } else if (batchStrategy === 'salary' && totalSalary > 0) {
          const ratios = calculateSalaryRatios(matched, salaryValues);
          allocations.forEach((a) => {
            a.pct = ratios[a.user_name] ?? 0;
          });
        } else {
          // Even
          const n = matched.length;
          const exact = 100 / n;
          const floor = Math.floor(exact);
          const extra = 100 - floor * n;
          matched.forEach((u, idx) => {
            const alloc = allocations.find((a) => a.user_name === u.name);
            if (alloc) alloc.pct = floor + (idx < extra ? 1 : 0);
          });
        }

        await api.updateSplit(cat, { allocations });

        // Update local edit values
        const fresh = {};
        allocations.forEach((a) => {
          fresh[a.user_name] = String(a.pct);
        });
        editValues[cat] = fresh;
      }
      editValues = { ...editValues };
      batchMessage = `✓ Successfully updated ${batchSelectedCategories.length} category split agreement(s).`;
      setTimeout(() => { batchMessage = ''; }, 4000);
    } catch (err) {
      batchError = err.message || 'Batch update failed.';
    } finally {
      batchApplying = false;
    }
  }

  // ── Lifecycle ─────────────────────────────────────────────────────────────
  onMount(async () => {
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
      console.error('ONMOUNT SALARY ERROR:', err);
    } finally {
      salaryLoading = false;
    }
  });
</script>

<div class="space-y-6">

  <!-- ── Top Header & Sub-Tab Navigation Bar ───────────────────────────────── -->
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
    <div>
      <h1 class="page-title flex items-center gap-2.5">
        <span>⚖️</span> Categories & Split Agreements
      </h1>
      <p class="page-subtitle">
        Configure expense split allocations between arbitrary subgroups, couples, and joint accounts
      </p>
    </div>
  </div>

  <!-- Sub-Navigation Pills -->
  <nav class="flex flex-wrap gap-2 border-b border-neutral-200 dark:border-neutral-800 pb-3" aria-label="Categories sub-navigation">
    {#each [
      ['agreements', '⚖️ Split Agreements'],
      ['matrix', '📊 Split Matrix'],
      ['batch', '⚡ Batch Rule Builder'],
      ['income', '💰 Income Categories'],
    ] as [id, label]}
      <button
        id="split-nav-{id}"
        type="button"
        on:click={() => (section = id)}
        class="px-4 py-2 rounded-xl text-xs font-semibold transition-all duration-150 cursor-pointer
               {section === id
                 ? 'bg-indigo-600 text-white shadow-sm'
                 : 'bg-neutral-100 dark:bg-neutral-900 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200 border border-neutral-200 dark:border-neutral-800 hover:border-neutral-300 dark:hover:border-neutral-700'}"
      >
        {label}
      </button>
    {/each}
  </nav>

  <!-- ── Salary Header Banner (Shared across tabs) ────────────────────────── -->
  <div class="card p-4 sm:p-5">
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 mb-3">
      <div>
        <p class="text-xs font-semibold text-neutral-700 dark:text-neutral-300 uppercase tracking-wider">Current Monthly Salaries</p>
        <p class="text-xs text-neutral-500 mt-0.5">Base employment salaries define the default income-proportional split ratios.</p>
      </div>
      <button
        id="link-manage-income"
        type="button"
        on:click={() => dispatch('navigateIncome')}
        class="px-3.5 py-1.5 rounded-xl text-xs font-semibold bg-indigo-50 dark:bg-indigo-950/80 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-700/60 hover:bg-indigo-100 dark:hover:bg-indigo-900 transition-colors flex items-center gap-1.5 self-start sm:self-auto cursor-pointer"
      >
        <span>Manage in Income Tab</span>
        <span>→</span>
      </button>
    </div>

    {#if salaryLoading}
      <p class="text-xs text-neutral-500">Loading household salaries…</p>
    {:else}
      <div class="flex flex-wrap gap-3 items-center">
        {#each activeUsers as u (u.name)}
          <div class="flex items-center gap-2 px-3 py-1.5 rounded-xl bg-neutral-50 dark:bg-neutral-950/80 border border-neutral-200 dark:border-neutral-800">
            <div
              class="w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold shrink-0 text-white shadow-sm"
              style="background-color: {u.color}"
            >
              {u.name.charAt(0).toUpperCase()}
            </div>
            <span class="text-xs font-medium" style="color: {u.color}">{u.name}:</span>
            <span class="text-xs font-bold text-neutral-900 dark:text-neutral-100 tabular-nums">
              {$currencySymbol}{(salaryValues[u.name] || 0).toFixed(2)}
            </span>
          </div>
        {/each}

        {#if totalSalary > 0 && activeUsers.length >= 2}
          <div class="flex items-center gap-1.5 text-xs text-neutral-500 dark:text-neutral-400 ml-auto bg-neutral-50 dark:bg-neutral-950/90 px-3 py-1.5 rounded-xl border border-neutral-200 dark:border-neutral-800">
            <span class="text-neutral-500 font-medium">Household Salary Ratio:</span>
            {#each activeUsers as u, i}
              <span class="font-bold" style="color: {u.color}">{salaryPct(u.name)}%</span>
              {#if i < activeUsers.length - 1}<span class="text-neutral-400 dark:text-neutral-600">/</span>{/if}
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- ── Unified Add Category Bar ─────────────────────────────────────────── -->
  <div class="card space-y-3">
    <div class="flex items-end gap-3 flex-wrap">
      <div class="flex-1 min-w-[200px]">
        <label for="new-category" class="block text-xs font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-1.5">
          New Category Name
        </label>
        <input
          id="new-category"
          type="text"
          bind:value={newCategoryName}
          placeholder={newCatType === 'expense' ? 'e.g. UTILITIES, GROCERIES, PETS' : 'e.g. FREELANCE, DIVIDENDS'}
          class="input-field uppercase"
          on:keydown={(e) => e.key === 'Enter' && handleAddCategory()}
        />
      </div>

      <!-- Type toggle -->
      <div class="flex flex-col">
        <span class="block text-xs font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-1.5">Category Type</span>
        <div class="inline-flex rounded-xl bg-neutral-100 dark:bg-neutral-950 p-1 border border-neutral-200 dark:border-neutral-800 h-[42px] items-center">
          <button
            id="cat-type-expense"
            type="button"
            on:click={() => (newCatType = 'expense')}
            class="px-3 py-1.5 rounded-lg text-xs font-semibold transition-colors cursor-pointer {newCatType === 'expense' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
          >
            Expense
          </button>
          <button
            id="cat-type-income"
            type="button"
            on:click={() => (newCatType = 'income')}
            class="px-3 py-1.5 rounded-lg text-xs font-semibold transition-colors cursor-pointer {newCatType === 'income' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
          >
            Income
          </button>
        </div>
      </div>

      <button
        id="add-category-btn"
        type="button"
        on:click={handleAddCategory}
        disabled={creatingCategory || !newCategoryName.trim()}
        class="btn-primary h-[42px] self-end"
      >
        {creatingCategory ? 'Adding...' : '+ Add Category'}
      </button>
    </div>
    {#if createCategoryError}
      <p class="text-xs text-rose-700 dark:text-red-400 bg-rose-50 dark:bg-red-950/40 border border-rose-200 dark:border-red-800 rounded-xl px-3 py-2">{createCategoryError}</p>
    {/if}
  </div>

  <!-- ═════════════════════════════════════════════════════════════════════════
       SUBTAB 1: ⚖️ SPLIT AGREEMENTS (Focused Editor with Subgroups)
       ═════════════════════════════════════════════════════════════════════════ -->
  {#if section === 'agreements'}
    <div class="space-y-6">

      <!-- Subgroup & Arbitrary User Selection Toolbar -->
      <div class="card p-4 sm:p-5 space-y-4 border-indigo-200 dark:border-indigo-500/20 bg-gradient-to-r from-neutral-50 via-neutral-50 to-indigo-50/40 dark:from-neutral-900 dark:via-neutral-900 dark:to-indigo-950/20">
        <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div>
            <h2 class="text-sm font-bold text-neutral-900 dark:text-white flex items-center gap-2">
              <span>👥</span> Subgroup Split Preset Bar
            </h2>
            <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">
              Select a subgroup to quickly tune category agreements between specific housemates.
            </p>
          </div>

          <!-- Mode Picker Pills -->
          <div class="flex flex-wrap items-center gap-1.5">
            <button
              type="button"
              on:click={() => selectSubgroup('all')}
              class="px-3 py-1.5 rounded-xl text-xs font-semibold transition-colors cursor-pointer {selectedSubgroupMode === 'all' ? 'bg-indigo-600 text-white shadow-sm' : 'bg-neutral-200 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300 hover:bg-neutral-300 dark:hover:bg-neutral-700'}"
            >
              All Members ({activeUsers.length})
            </button>

            {#each $jointAccounts || [] as acc}
              <button
                type="button"
                on:click={() => selectSubgroup(acc.id)}
                class="px-3 py-1.5 rounded-xl text-xs font-semibold transition-colors cursor-pointer flex items-center gap-1.5 {selectedSubgroupMode === acc.id ? 'bg-indigo-600 text-white shadow-sm' : 'bg-neutral-200 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300 hover:bg-neutral-300 dark:hover:bg-neutral-700'}"
              >
                <span>🏦</span>
                <span>{acc.name}</span>
                {#if acc.member_names && acc.member_names.length > 0}
                  <span class="text-[10px] opacity-75">({acc.member_names.length})</span>
                {/if}
              </button>
            {/each}

            <button
              type="button"
              on:click={() => selectSubgroup('custom')}
              class="px-3 py-1.5 rounded-xl text-xs font-semibold transition-colors cursor-pointer {selectedSubgroupMode === 'custom' ? 'bg-indigo-600 text-white shadow-sm' : 'bg-neutral-200 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300 hover:bg-neutral-300 dark:hover:bg-neutral-700'}"
            >
              Custom Selection…
            </button>
          </div>
        </div>

        <!-- Custom User Chip Multi-Selector -->
        <div class="flex flex-wrap items-center gap-2 pt-2 border-t border-neutral-200 dark:border-neutral-800/80">
          <span class="text-xs text-neutral-500 dark:text-neutral-400 font-medium mr-1">Participating Users:</span>
          {#each activeUsers as u}
            {@const isSelected = customSelectedUsers.includes(u.name)}
            <button
              type="button"
              on:click={() => {
                selectedSubgroupMode = 'custom';
                toggleCustomUser(u.name);
              }}
              class="inline-flex items-center gap-2 px-3 py-1.5 rounded-xl text-xs font-semibold border transition-all cursor-pointer {isSelected ? 'bg-indigo-50 dark:bg-indigo-950/80 border-indigo-500 text-indigo-900 dark:text-white shadow-sm font-bold' : 'bg-white dark:bg-neutral-900 border-neutral-300 dark:border-neutral-800 text-neutral-500 hover:text-neutral-800 dark:hover:text-neutral-300'}"
            >
              <span class="w-2.5 h-2.5 rounded-full" style="background-color: {u.color}"></span>
              <span>{u.name}</span>
              {#if isSelected}
                <span class="text-[10px] text-indigo-600 dark:text-indigo-400 font-bold">✓</span>
              {/if}
            </button>
          {/each}
          <span class="text-xs text-neutral-500 ml-auto font-mono">
            {customSelectedUsers.length} of {activeUsers.length} selected
          </span>
        </div>
      </div>

      <!-- Category Filter Search -->
      <div class="flex items-center justify-between gap-3">
        <div class="relative flex-1 max-w-sm">
          <input
            type="text"
            bind:value={categorySearch}
            placeholder="Search categories…"
            class="input-field py-2 text-xs"
          />
        </div>
        <p class="text-xs text-neutral-500">
          Showing {filteredSplits.length} of {variableSplits.length} variable categories
        </p>
      </div>

      <!-- Categories List -->
      <div class="space-y-4">
        {#each filteredSplits as split (split.category)}
          {#if editValues[split.category]}
            {@const sum = rowSum(split.category, editValues)}
            {@const sumOk = Math.abs(sum - 100) < 0.05}
            {@const isJointCategory = jointCategorySet.has(split.category)}

            <div class="card p-4 sm:p-5 space-y-4 {isJointCategory ? 'border-indigo-300 dark:border-indigo-800/40 bg-indigo-50/50 dark:bg-indigo-950/10' : ''}">
              <!-- Header Row: Category Name, Badges, Quick Subgroup Presets -->
              <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
                <div class="flex items-center gap-2 flex-wrap">
                  <span class="text-sm font-bold text-neutral-900 dark:text-white px-3 py-1 rounded-xl bg-neutral-100 dark:bg-neutral-800 border border-neutral-200 dark:border-neutral-700/80">
                    {split.category}
                  </span>
                  {#if isJointCategory}
                    <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-lg text-xs bg-indigo-100 dark:bg-indigo-950 text-indigo-700 dark:text-indigo-300 border border-indigo-300 dark:border-indigo-700/60 font-semibold" title="Category is managed directly by the Joint Account">
                      🏦 Joint Account (Locked)
                    </span>
                  {/if}
                </div>

                <!-- Presets applied to this category -->
                {#if !isJointCategory}
                  <div class="flex items-center gap-1.5 flex-wrap">
                    <span class="text-[10px] text-neutral-500 uppercase tracking-wider font-semibold mr-1">Quick Apply:</span>
                    <button
                      type="button"
                      on:click={() => applyEvenSplit(split.category)}
                      class="px-2.5 py-1 rounded-lg bg-neutral-100 dark:bg-neutral-800 hover:bg-neutral-200 dark:hover:bg-neutral-700 text-neutral-700 dark:text-neutral-300 text-xs font-medium transition-colors cursor-pointer border border-neutral-200 dark:border-transparent"
                      title="Split evenly among all members"
                    >
                      Even ({Math.floor(100 / activeUsers.length)}%)
                    </button>
                    {#each $jointAccounts || [] as acc}
                      {#if acc.member_names && acc.member_names.length > 0 && acc.member_names.length < activeUsers.length}
                        <button
                          type="button"
                          on:click={() => applySubgroupSplit(split.category, acc.member_names, 'even')}
                          class="px-2.5 py-1 rounded-lg bg-indigo-50 dark:bg-indigo-950/70 hover:bg-indigo-100 dark:hover:bg-indigo-900/80 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800/50 text-xs font-medium transition-colors cursor-pointer"
                          title="Split among {acc.name} members ({acc.member_names.join(' & ')})"
                        >
                          🏦 {acc.name} ({acc.member_names.join('+')})
                        </button>
                      {/if}
                    {/each}
                    {#if customSelectedUsers.length < activeUsers.length && customSelectedUsers.length > 0 && selectedSubgroupMode === 'custom'}
                      <button
                        type="button"
                        on:click={() => applySubgroupSplit(split.category, customSelectedUsers, 'even')}
                        class="px-2.5 py-1 rounded-lg bg-indigo-50 dark:bg-indigo-950 hover:bg-indigo-100 dark:hover:bg-indigo-900 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800 text-xs font-medium transition-colors cursor-pointer"
                        title="Split equally among active selected subgroup ({customSelectedUsers.join(' & ')})"
                      >
                        Subgroup ({customSelectedUsers.join('+')})
                      </button>
                    {/if}
                    {#if totalSalary > 0}
                      <button
                        type="button"
                        on:click={() => resetToSalary(split.category)}
                        class="px-2.5 py-1 rounded-lg bg-amber-50 dark:bg-neutral-800 hover:bg-amber-100 dark:hover:bg-neutral-700 text-amber-800 dark:text-amber-300 text-xs font-medium transition-colors cursor-pointer border border-amber-200 dark:border-transparent"
                        title="Distribute proportionally to monthly salary ratios"
                      >
                        Salary Ratio
                      </button>
                    {/if}
                  </div>
                {/if}
              </div>

              <!-- Visual Split Percentage Bar -->
              <div class="space-y-1.5">
                <div class="h-2.5 w-full bg-neutral-200 dark:bg-neutral-950 rounded-full overflow-hidden flex shadow-inner">
                  {#each activeUsers as u}
                    {@const pctVal = parseFloat(editValues[split.category][u.name] || '0')}
                    {#if pctVal > 0}
                      <div
                        class="h-full transition-all duration-300"
                        style="width: {pctVal}%; background-color: {u.color}"
                        title="{u.name}: {pctVal}%"
                      ></div>
                    {/if}
                  {/each}
                </div>
                <div class="flex justify-between items-center text-[11px] text-neutral-500 dark:text-neutral-400">
                  <div class="flex items-center gap-3 flex-wrap">
                    {#each activeUsers as u}
                      {@const pctVal = parseFloat(editValues[split.category][u.name] || '0')}
                      <span class="inline-flex items-center gap-1 font-semibold {pctVal > 0 ? '' : 'opacity-40'}">
                        <span class="w-2.5 h-2.5 rounded-full" style="background-color: {u.color}"></span>
                        <span style="color: {u.color}">{u.name}: {pctVal}%</span>
                      </span>
                    {/each}
                  </div>
                  <span class="font-bold tabular-nums {sumOk ? 'text-emerald-600 dark:text-emerald-400' : 'text-amber-600 dark:text-amber-400'}">
                    Total: {sum}%
                  </span>
                </div>
              </div>

              <!-- Input Controls Row / Slider Row -->
              {#if $splitInputMode === 'slider' && activeUsers.length === 2}
                <!-- 2-User Slider Mode -->
                {@const sliderVal = Math.round(parseFloat(editValues[split.category][activeUsers[0].name] || '0'))}
                <div class="flex items-center gap-3 pt-2">
                  <div class="flex-1">
                    <input
                      id="slider-{split.category}"
                      type="range"
                      min="0"
                      max="100"
                      step="1"
                      value={sliderVal}
                      disabled={isJointCategory}
                      on:input={(e) => {
                        const val = Math.round(parseFloat(e.target.value));
                        editValues[split.category][activeUsers[0].name] = String(val);
                        editValues[split.category][activeUsers[1].name] = String(100 - val);
                        editValues = { ...editValues };
                      }}
                      class="w-full h-2.5 rounded-full cursor-pointer slider-split disabled:opacity-40"
                      style="background: linear-gradient(to right, {activeUsers[0].color} {sliderVal}%, {activeUsers[1].color} {sliderVal}%)"
                    />
                  </div>
                </div>
              {:else}
                <!-- Multi-User Inputs Grid -->
                <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-3 pt-2">
                  {#each activeUsers as u}
                    <div class="bg-neutral-50 dark:bg-neutral-950/70 p-2.5 rounded-xl border border-neutral-200 dark:border-neutral-800 space-y-1.5">
                      <div class="flex items-center justify-between text-xs">
                        <span class="font-semibold" style="color: {u.color}">{u.name}</span>
                        <button
                          type="button"
                          on:click={() => setSinglePayer(split.category, u.name)}
                          disabled={isJointCategory}
                          class="text-[10px] text-neutral-500 hover:text-indigo-600 dark:hover:text-indigo-400 transition-colors"
                          title="Assign 100% to {u.name}"
                        >
                          100%
                        </button>
                      </div>
                      <div class="relative">
                        <input
                          id="split-{u.name}-{split.category}"
                          type="number"
                          min="0"
                          max="100"
                          step="1"
                          disabled={isJointCategory}
                          bind:value={editValues[split.category][u.name]}
                          class="w-full bg-white dark:bg-neutral-900 border border-neutral-300 dark:border-neutral-700/80 rounded-lg px-2.5 py-1.5 text-sm font-semibold tabular-nums text-neutral-900 dark:text-neutral-100 disabled:opacity-40 focus:outline-none focus:ring-1"
                          style="--tw-ring-color: {u.color}"
                        />
                        <span class="absolute right-2.5 top-1/2 -translate-y-1/2 text-xs text-neutral-500 font-bold">%</span>
                      </div>
                    </div>
                  {/each}
                </div>
              {/if}

              <!-- Actions & Status Notifications -->
              <div class="flex items-center justify-between pt-2 border-t border-neutral-200 dark:border-neutral-800/80">
                <div>
                  {#if rowSuccess[split.category]}
                    <span class="text-xs text-emerald-600 dark:text-emerald-400 font-semibold">✓ Agreement Saved Successfully</span>
                  {/if}
                  {#if rowError[split.category]}
                    <span class="text-xs text-rose-700 dark:text-red-400">{rowError[split.category]}</span>
                  {/if}
                </div>

                <div class="flex items-center gap-2">
                  <button
                    id="reset-split-{split.category}"
                    type="button"
                    on:click={() => resetToSalary(split.category)}
                    disabled={isJointCategory || totalSalary === 0}
                    class="btn-secondary py-1.5 text-xs"
                    title="Reset to salary ratio"
                  >
                    Reset
                  </button>
                  <button
                    id="save-split-{split.category}"
                    type="button"
                    on:click={() => save(split.category)}
                    disabled={isJointCategory || saving[split.category] || !sumOk}
                    class="btn-primary py-1.5 text-xs"
                  >
                    {saving[split.category] ? 'Saving…' : 'Save Agreement'}
                  </button>
                </div>
              </div>
            </div>
          {/if}
        {/each}

        {#if filteredSplits.length === 0}
          <div class="card empty-state-box">
            <p class="text-neutral-600 dark:text-neutral-400 text-sm">No matching categories found.</p>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- ═════════════════════════════════════════════════════════════════════════
       SUBTAB 2: 📊 SPLIT MATRIX & OVERVIEW
       ═════════════════════════════════════════════════════════════════════════ -->
  {#if section === 'matrix'}
    <div class="space-y-6">
      <div class="card p-5 sm:p-6 space-y-4">
        <div>
          <h2 class="text-sm font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span>📊</span> Household Category Agreement Matrix
          </h2>
          <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-1">
            Overview of all active expense categories and how liability is divided among members.
          </p>
        </div>

        <div class="overflow-x-auto -mx-1">
          <table class="w-full text-sm border-collapse">
            <thead>
              <tr class="border-b border-neutral-200 dark:border-neutral-800 text-left text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">
                <th class="py-3 px-3">Category</th>
                <th class="py-3 px-3">Agreement Type</th>
                {#each activeUsers as u}
                  <th class="py-3 px-3" style="color: {u.color}">{u.name}</th>
                {/each}
                <th class="py-3 px-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-neutral-200/60 dark:divide-neutral-800/60">
              {#each variableSplits as split}
                {@const isJoint = jointCategorySet.has(split.category)}
                {@const vals = editValues[split.category] || {}}
                <tr class="hover:bg-neutral-50 dark:hover:bg-neutral-800/30 transition-colors">
                  <td class="py-3 px-3 font-semibold text-neutral-900 dark:text-neutral-200">
                    {split.category}
                  </td>
                  <td class="py-3 px-3">
                    {#if isJoint}
                      <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-semibold bg-indigo-50 dark:bg-indigo-950 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800/60">
                        🏦 Joint Account
                      </span>
                    {:else}
                      <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-semibold bg-neutral-100 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300 border border-neutral-200 dark:border-neutral-700">
                        ⚖️ Household Split
                      </span>
                    {/if}
                  </td>
                  {#each activeUsers as u}
                    {@const pctVal = parseFloat(vals[u.name] || '0')}
                    <td class="py-3 px-3 tabular-nums font-semibold {pctVal > 0 ? '' : 'text-neutral-400 dark:text-neutral-600'}">
                      {pctVal}%
                    </td>
                  {/each}
                  <td class="py-3 px-3 text-right">
                    <button
                      type="button"
                      on:click={() => {
                        categorySearch = split.category;
                        section = 'agreements';
                      }}
                      class="text-xs text-indigo-600 dark:text-indigo-400 hover:underline font-semibold"
                    >
                      Edit →
                    </button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>

      <!-- Self-Pay Personal Categories -->
      {#if personalSplits.length > 0}
        <div class="card p-5 space-y-3">
          <div class="flex items-center gap-2">
            <span class="text-base">🔒</span>
            <div>
              <h3 class="text-xs font-semibold text-neutral-800 dark:text-neutral-200 uppercase tracking-wider">Self-Pay Categories</h3>
              <p class="text-xs text-neutral-500 mt-0.5">
                Expenses in these categories are carried 100% by whoever paid — cross-member split percentages are excluded.
              </p>
            </div>
          </div>
          <div class="flex flex-wrap gap-2 pt-2">
            {#each personalSplits as ps}
              <span class="px-3 py-1.5 rounded-xl bg-neutral-100 dark:bg-neutral-950 border border-neutral-200 dark:border-neutral-800 text-xs font-semibold text-neutral-700 dark:text-neutral-400">
                {ps.category} (100% Payer)
              </span>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}

  <!-- ═════════════════════════════════════════════════════════════════════════
       SUBTAB 3: ⚡ BATCH RULE BUILDER
       ═════════════════════════════════════════════════════════════════════════ -->
  {#if section === 'batch'}
    <div class="card p-5 sm:p-6 space-y-6">
      <div>
        <h2 class="text-sm font-bold text-neutral-900 dark:text-white flex items-center gap-2">
          <span>⚡</span> Batch Category Split Rule Applicator
        </h2>
        <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-1">
          Apply a unified split rule across multiple categories at once for an arbitrary selection of members.
        </p>
      </div>

      {#if batchMessage}
        <div class="p-3.5 bg-emerald-50 dark:bg-emerald-950/60 border border-emerald-200 dark:border-emerald-800/80 rounded-xl text-emerald-700 dark:text-emerald-300 text-xs">
          {batchMessage}
        </div>
      {/if}
      {#if batchError}
        <div class="p-3.5 bg-rose-50 dark:bg-red-950/60 border border-rose-200 dark:border-red-800/80 rounded-xl text-rose-700 dark:text-red-300 text-xs">
          {batchError}
        </div>
      {/if}

      <!-- Step 1: Select Categories -->
      <div class="space-y-2">
        <div class="flex items-center justify-between">
          <label class="text-xs font-semibold text-neutral-700 dark:text-neutral-300 uppercase tracking-wider">
            1. Select Target Categories ({batchSelectedCategories.length} selected)
          </label>
          <button
            type="button"
            on:click={toggleBatchAllCategories}
            class="text-xs text-indigo-600 dark:text-indigo-400 hover:underline font-semibold"
          >
            {batchSelectedCategories.length === variableSplits.length ? 'Deselect All' : 'Select All'}
          </button>
        </div>

        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-2 bg-neutral-50 dark:bg-neutral-950/60 p-4 rounded-xl border border-neutral-200 dark:border-neutral-800 max-h-56 overflow-y-auto">
          {#each variableSplits as s}
            {@const isChecked = batchSelectedCategories.includes(s.category)}
            {@const isJoint = jointCategorySet.has(s.category)}
            <button
              type="button"
              disabled={isJoint}
              on:click={() => toggleBatchCategory(s.category)}
              class="flex items-center gap-2 px-3 py-2 rounded-lg text-xs font-medium border text-left transition-all {isJoint ? 'opacity-40 cursor-not-allowed border-neutral-200 dark:border-neutral-800 bg-neutral-100 dark:bg-neutral-900/40 text-neutral-400 dark:text-neutral-600' : isChecked ? 'bg-indigo-50 dark:bg-indigo-950 border-indigo-500 text-indigo-900 dark:text-indigo-200 shadow-sm font-semibold' : 'bg-white dark:bg-neutral-900 border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
            >
              <input type="checkbox" checked={isChecked} disabled={isJoint} class="rounded border-neutral-300 dark:border-neutral-700 pointer-events-none" />
              <span class="truncate">{s.category}</span>
            </button>
          {/each}
        </div>
      </div>

      <!-- Step 2: Select Participating Members -->
      <div class="space-y-2">
        <label class="block text-xs font-semibold text-neutral-700 dark:text-neutral-300 uppercase tracking-wider">
          2. Participating Household Members
        </label>
        <div class="flex flex-wrap gap-2">
          {#each activeUsers as u}
            {@const isChecked = batchSelectedMembers.includes(u.name)}
            <button
              type="button"
              on:click={() => toggleBatchMember(u.name)}
              class="inline-flex items-center gap-2 px-3.5 py-2 rounded-xl text-xs font-semibold border transition-all cursor-pointer {isChecked ? 'bg-indigo-50 dark:bg-indigo-950 border-indigo-500 text-indigo-900 dark:text-white font-bold' : 'bg-white dark:bg-neutral-900 border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-500'}"
            >
              <span class="w-2.5 h-2.5 rounded-full" style="background-color: {u.color}"></span>
              <span>{u.name}</span>
              {#if isChecked}<span class="text-[10px] text-indigo-600 dark:text-indigo-400 font-bold">✓</span>{/if}
            </button>
          {/each}
        </div>
      </div>

      <!-- Step 3: Split Strategy -->
      <div class="space-y-2">
        <label class="block text-xs font-semibold text-neutral-700 dark:text-neutral-300 uppercase tracking-wider">
          3. Distribution Strategy
        </label>
        <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
          <button
            type="button"
            on:click={() => (batchStrategy = 'even')}
            class="p-3 rounded-xl border text-left transition-all cursor-pointer {batchStrategy === 'even' ? 'bg-indigo-50 dark:bg-indigo-950/80 border-indigo-500 text-indigo-900 dark:text-white font-semibold' : 'bg-white dark:bg-neutral-950 border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400'}"
          >
            <p class="text-xs font-bold">Equal / Even Split</p>
            <p class="text-[11px] text-neutral-500 mt-1">Split 100% equally among selected participants.</p>
          </button>

          <button
            type="button"
            on:click={() => (batchStrategy = 'salary')}
            class="p-3 rounded-xl border text-left transition-all cursor-pointer {batchStrategy === 'salary' ? 'bg-indigo-50 dark:bg-indigo-950/80 border-indigo-500 text-indigo-900 dark:text-white font-semibold' : 'bg-white dark:bg-neutral-950 border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400'}"
          >
            <p class="text-xs font-bold">Salary-Proportional</p>
            <p class="text-[11px] text-neutral-500 mt-1">Calculate based on active base salary ratios.</p>
          </button>

          <button
            type="button"
            on:click={() => (batchStrategy = 'single')}
            class="p-3 rounded-xl border text-left transition-all cursor-pointer {batchStrategy === 'single' ? 'bg-indigo-50 dark:bg-indigo-950/80 border-indigo-500 text-indigo-900 dark:text-white font-semibold' : 'bg-white dark:bg-neutral-950 border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400'}"
          >
            <p class="text-xs font-bold">100% Single Payer</p>
            <p class="text-[11px] text-neutral-500 mt-1">Assign 100% liability to one specific person.</p>
          </button>
        </div>

        {#if batchStrategy === 'single'}
          <div class="pt-2 flex items-center gap-3">
            <span class="text-xs text-neutral-600 dark:text-neutral-400">Assigned Member:</span>
            <select bind:value={batchSinglePayer} class="select-field max-w-xs py-1.5 text-xs">
              {#each activeUsers as u}
                <option value={u.name}>{u.name}</option>
              {/each}
            </select>
          </div>
        {/if}
      </div>

      <button
        type="button"
        on:click={handleApplyBatch}
        disabled={batchApplying || batchSelectedCategories.length === 0}
        class="btn-primary w-full py-3"
      >
        {batchApplying ? 'Applying Batch Rules…' : `Apply Split Rule to ${batchSelectedCategories.length} Category(ies)`}
      </button>
    </div>
  {/if}

  <!-- ═════════════════════════════════════════════════════════════════════════
       SUBTAB 4: 💰 INCOME CATEGORIES
       ═════════════════════════════════════════════════════════════════════════ -->
  {#if section === 'income'}
    <div class="card p-5 sm:p-6 space-y-6">
      <div>
        <h2 class="text-sm font-bold text-neutral-900 dark:text-white flex items-center gap-2">
          <span>💰</span> Income Classification Categories
        </h2>
        <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-1">
          Income categories are used to classify one-off income, freelance streams, bonuses, gifts, and dividends in the Income ledger.
        </p>
      </div>

      <div class="bg-neutral-50 dark:bg-neutral-950/70 border border-neutral-200 dark:border-neutral-800 rounded-2xl p-4 sm:p-5">
        {#if $incomeCategories.length === 0}
          <p class="text-xs text-neutral-500">No income categories defined yet. Use the category creation bar above to add one.</p>
        {:else}
          <div class="flex flex-wrap gap-2.5">
            {#each $incomeCategories as c (c.category)}
              <div class="flex items-center gap-2.5 px-3.5 py-2 rounded-xl bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-700/80 text-xs font-semibold text-neutral-800 dark:text-neutral-200 shadow-sm">
                <span>{c.category}</span>
                <button
                  id="delete-income-cat-{c.category}"
                  type="button"
                  on:click={() => handleRemoveIncomeCategory(c.category)}
                  disabled={deletingIncomeCat === c.category}
                  class="w-5 h-5 rounded-lg flex items-center justify-center text-neutral-400 hover:text-rose-600 dark:hover:text-red-400 hover:bg-rose-50 dark:hover:bg-red-950/40 transition-colors disabled:opacity-30 cursor-pointer"
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
  {/if}

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
    border: 2px solid rgba(99, 102, 241, 0.8);
    cursor: pointer;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.25);
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
    border: 2px solid rgba(99, 102, 241, 0.8);
    cursor: pointer;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.25);
  }
</style>
