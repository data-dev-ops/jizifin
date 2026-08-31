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

  // ── Rename Category Modal State ───────────────────────────────────────────
  let showRenameModal = false;
  let renameCategoryTarget = '';
  let renameCategoryNewName = '';
  let renameCategoryType = 'expense'; // 'expense' | 'income'
  let renameCategoryLoading = false;
  let renameCategoryError = '';

  function openRenameModal(name, type = 'expense') {
    renameCategoryTarget = name;
    renameCategoryNewName = name;
    renameCategoryType = type;
    renameCategoryError = '';
    showRenameModal = true;
  }

  function closeRenameModal() {
    showRenameModal = false;
    renameCategoryTarget = '';
    renameCategoryNewName = '';
    renameCategoryError = '';
  }

  async function handleRenameCategory() {
    const trimmed = renameCategoryNewName.trim().toUpperCase();
    if (!trimmed) {
      renameCategoryError = 'Category name cannot be empty.';
      return;
    }
    if (trimmed === renameCategoryTarget) {
      closeRenameModal();
      return;
    }
    renameCategoryLoading = true;
    renameCategoryError = '';
    try {
      if (renameCategoryType === 'expense') {
        await api.renameSplit(renameCategoryTarget, trimmed);
        if (editValues[renameCategoryTarget]) {
          editValues[trimmed] = { ...editValues[renameCategoryTarget] };
          delete editValues[renameCategoryTarget];
          editValues = { ...editValues };
        }
      } else {
        await api.updateIncomeCategory(renameCategoryTarget, trimmed);
      }
      closeRenameModal();
    } catch (err) {
      renameCategoryError = err.message || 'Failed to rename category.';
    } finally {
      renameCategoryLoading = false;
    }
  }

  // ── Delete Category Modal State ───────────────────────────────────────────
  let showDeleteModal = false;
  let deleteCategoryTarget = '';
  let deleteCategoryType = 'expense'; // 'expense' | 'income'
  let deleteCategoryLoading = false;
  let deleteCategoryError = '';

  function openDeleteModal(name, type = 'expense') {
    deleteCategoryTarget = name;
    deleteCategoryType = type;
    deleteCategoryError = '';
    showDeleteModal = true;
  }

  function closeDeleteModal() {
    showDeleteModal = false;
    deleteCategoryTarget = '';
    deleteCategoryError = '';
  }

  async function handleConfirmDeleteCategory() {
    deleteCategoryLoading = true;
    deleteCategoryError = '';
    try {
      if (deleteCategoryType === 'expense') {
        await api.deleteSplit(deleteCategoryTarget);
        if (editValues[deleteCategoryTarget]) {
          delete editValues[deleteCategoryTarget];
          editValues = { ...editValues };
        }
      } else {
        await api.deleteIncomeCategory(deleteCategoryTarget);
      }
      closeDeleteModal();
    } catch (err) {
      deleteCategoryError = err.message || 'Failed to delete category.';
    } finally {
      deleteCategoryLoading = false;
    }
  }

  // ── SCD2 Split Timeline & Temporary Override State ──────────────────────────
  let expandedTimelines = {};
  let showOverrideModal = false;
  let overrideModalMode = 'create'; // 'create' | 'edit'
  let overrideId = null;
  let overrideCategory = '';
  let overrideStartDate = '';
  let overrideEndDate = '';
  let overrideNote = '';
  let overrideAllocations = {};
  let overrideLoading = false;
  let overrideError = '';

  function toggleTimeline(category) {
    expandedTimelines[category] = !expandedTimelines[category];
    expandedTimelines = { ...expandedTimelines };
  }

  function openCreateOverrideModal(category) {
    overrideModalMode = 'create';
    overrideId = null;
    overrideCategory = category;
    const now = new Date();
    const startStr = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-01`;
    const lastDay = new Date(now.getFullYear(), now.getMonth() + 1, 0).getDate();
    const endStr = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(lastDay).padStart(2, '0')}`;
    overrideStartDate = startStr;
    overrideEndDate = endStr;
    overrideNote = '';

    const base = editValues[category] || {};
    const allocs = {};
    for (const u of activeUsers) {
      allocs[u.name] = base[u.name] || '0';
    }
    overrideAllocations = allocs;
    overrideError = '';
    showOverrideModal = true;
  }

  function openEditOverrideModal(agreement) {
    overrideModalMode = 'edit';
    overrideId = agreement.id;
    overrideCategory = agreement.category;
    overrideStartDate = agreement.start_date;
    overrideEndDate = agreement.end_date || '';
    overrideNote = agreement.note || '';

    const allocs = {};
    for (const u of activeUsers) {
      const found = (agreement.allocations || []).find((a) => a.user_name === u.name);
      allocs[u.name] = found ? String(Math.round(found.pct)) : '0';
    }
    overrideAllocations = allocs;
    overrideError = '';
    showOverrideModal = true;
  }

  function closeOverrideModal() {
    showOverrideModal = false;
    overrideId = null;
    overrideCategory = '';
    overrideError = '';
  }

  function overrideSum() {
    return Number(Object.values(overrideAllocations).reduce((acc, v) => acc + (parseFloat(v) || 0), 0).toFixed(2));
  }

  async function handleSaveOverride() {
    if (!overrideStartDate) {
      overrideError = 'Start date is required.';
      return;
    }
    if (overrideEndDate && overrideEndDate < overrideStartDate) {
      overrideError = 'End date cannot be earlier than start date.';
      return;
    }
    const sum = overrideSum();
    if (Math.abs(sum - 100) >= 0.05) {
      overrideError = `Total allocation must equal 100% (currently ${sum}%).`;
      return;
    }

    overrideLoading = true;
    overrideError = '';
    try {
      const payload = {
        category: overrideCategory,
        start_date: overrideStartDate,
        end_date: overrideEndDate || null,
        is_active: true,
        note: overrideNote.trim() || null,
        allocations: activeUsers.map((u) => ({
          user_name: u.name,
          pct: parseFloat(overrideAllocations[u.name] || '0')
        }))
      };

      if (overrideModalMode === 'create') {
        await api.createSplitAgreement(overrideCategory, payload);
      } else {
        await api.updateSplitAgreement(overrideId, payload, overrideCategory);
      }
      closeOverrideModal();
    } catch (err) {
      overrideError = err.message || 'Failed to save split override.';
    } finally {
      overrideLoading = false;
    }
  }

  async function handleDeleteOverride(id, category) {
    if (!confirm('Are you sure you want to remove this temporary split override?')) return;
    try {
      await api.deleteSplitAgreement(id, category);
    } catch (err) {
      alert(err.message || 'Failed to delete split agreement.');
    }
  }

  // ── Per-row edit state & SCD2 Month-Aware Agreement Resolution ───────────
  /** { [category]: { [userName]: pctString } } */
  let editValues = {};
  let saving = {};
  let rowError = {};
  let rowSuccess = {};
  let lastEvaluatedMonth = null;
  let lastEvaluatedSplitsKey = '';

  /**
   * Resolves the effective split agreement for a given category and targetMonth (YYYY-MM).
   */
  export function getEffectiveSplitAgreement(split, targetMonth) {
    if (!split) return null;
    const agrs = split.agreements || [];
    if (!targetMonth || targetMonth === 'ALL') {
      const baseline = agrs.find((a) => a.is_active && a.end_date === null);
      if (baseline) return { ...baseline, is_override: false };
      return agrs[0] ? { ...agrs[0], is_override: agrs[0].end_date !== null } : null;
    }

    const monthStart = `${targetMonth}-01`;
    const monthEnd = `${targetMonth}-31`;

    const validAgrs = agrs.filter(
      (a) => a.is_active && a.start_date <= monthEnd && (a.end_date === null || a.end_date >= monthStart)
    );

    // Priority 1: Bounded temporary overrides covering targetMonth
    const bounded = validAgrs
      .filter((a) => a.end_date !== null)
      .sort((a, b) => b.start_date.localeCompare(a.start_date) || b.id - a.id);

    if (bounded.length > 0) {
      return { ...bounded[0], is_override: true };
    }

    // Priority 2: Baseline open-ended agreement
    const openEnded = validAgrs
      .filter((a) => a.end_date === null)
      .sort((a, b) => b.start_date.localeCompare(a.start_date) || b.id - a.id);

    if (openEnded.length > 0) {
      return { ...openEnded[0], is_override: false };
    }

    // Priority 3: Any active baseline agreement
    const fallbackBaseline = agrs.find((a) => a.is_active && a.end_date === null);
    if (fallbackBaseline) return { ...fallbackBaseline, is_override: false };

    return null;
  }

  export function isAgreementActiveForMonth(agreement, targetMonth) {
    if (!agreement || !agreement.is_active) return false;
    if (!targetMonth || targetMonth === 'ALL') return agreement.end_date === null;
    const monthStart = `${targetMonth}-01`;
    const monthEnd = `${targetMonth}-31`;
    return agreement.start_date <= monthEnd && (agreement.end_date === null || agreement.end_date >= monthStart);
  }

  function getEffectiveSplitAllocations(split, targetMonth, usersList) {
    const agreement = getEffectiveSplitAgreement(split, targetMonth);
    const entry = {};

    if (agreement && agreement.allocations && agreement.allocations.length > 0) {
      for (const alloc of agreement.allocations) {
        entry[alloc.user_name] = String(Math.round(alloc.pct));
      }
      for (const u of usersList) {
        if (!(u.name in entry)) entry[u.name] = '0';
      }
      return entry;
    }

    if (split && split.allocations && split.allocations.length > 0) {
      for (const alloc of split.allocations) {
        entry[alloc.user_name] = String(Math.round(alloc.pct));
      }
      for (const u of usersList) {
        if (!(u.name in entry)) entry[u.name] = '0';
      }
      return entry;
    }

    const n = usersList.length;
    if (n > 0) {
      const exact = 100 / n;
      const floor = Math.floor(exact);
      const sumFloors = floor * n;
      const extra = 100 - sumFloors;
      usersList.forEach((u, idx) => {
        entry[u.name] = String(floor + (idx < extra ? 1 : 0));
      });
    }
    return entry;
  }

  function computeSplitsKey(splitsList) {
    return (splitsList || [])
      .map((s) => `${s.category}:${(s.agreements || []).map((a) => `${a.id}-${a.start_date}-${a.end_date}-${(a.allocations || []).map((x) => x.pct).join(',')}`).join(';')}`)
      .join('|');
  }

  function syncEditValuesForMonth(month, force = false) {
    const currentKey = computeSplitsKey(variableSplits);
    if (!force && month === lastEvaluatedMonth && currentKey === lastEvaluatedSplitsKey && Object.keys(editValues).length >= variableSplits.length) {
      return;
    }
    for (const s of variableSplits) {
      editValues[s.category] = getEffectiveSplitAllocations(s, month, activeUsers);
    }
    editValues = { ...editValues };
    lastEvaluatedMonth = month;
    lastEvaluatedSplitsKey = currentKey;
  }

  $: if ($selectedMonth || variableSplits) {
    syncEditValuesForMonth($selectedMonth);
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
            {@const agreementsList = (split.agreements || []).filter(a => a.end_date !== null)}
            {@const baselineAgr = (split.agreements || []).find(a => a.end_date === null) || (split.agreements || [])[0]}
            {@const effectiveAgr = getEffectiveSplitAgreement(split, $selectedMonth)}
            {@const hasActiveOverride = effectiveAgr?.is_override === true}
            {@const isTimelineOpen = !!expandedTimelines[split.category]}

            <div class="card p-3.5 sm:p-4 space-y-2.5 {isJointCategory ? 'border-indigo-300 dark:border-indigo-800/40 bg-indigo-50/30 dark:bg-indigo-950/10' : ''}">
              <!-- Header Row: Category Name, Actions, Quick Presets & Save/Reset -->
              <div class="flex flex-col lg:flex-row lg:items-center justify-between gap-2.5">
                <!-- Left: Category Title, Rename, Delete, Joint Badge, Timeline Toggle -->
                <div class="flex items-center gap-1.5 flex-wrap">
                  <span class="text-xs sm:text-sm font-bold text-neutral-900 dark:text-white px-2.5 py-0.5 rounded-lg bg-neutral-100 dark:bg-neutral-800 border border-neutral-200 dark:border-neutral-700/80">
                    {split.category}
                  </span>
                  <button
                    id="rename-cat-{split.category}"
                    type="button"
                    on:click={() => openRenameModal(split.category, 'expense')}
                    disabled={isJointCategory}
                    class="p-1 rounded-md text-neutral-400 hover:text-indigo-600 dark:hover:text-indigo-400 hover:bg-indigo-50 dark:hover:bg-indigo-950/50 transition-colors cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
                    title="Rename category"
                    aria-label="Rename {split.category}"
                  >
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
                    </svg>
                  </button>
                  <button
                    id="delete-cat-{split.category}"
                    type="button"
                    on:click={() => openDeleteModal(split.category, 'expense')}
                    disabled={isJointCategory}
                    class="p-1 rounded-md text-neutral-400 hover:text-rose-600 dark:hover:text-red-400 hover:bg-rose-50 dark:hover:bg-red-950/50 transition-colors cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
                    title="Delete category"
                    aria-label="Delete {split.category}"
                  >
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                    </svg>
                  </button>

                  {#if isJointCategory}
                    <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[11px] bg-indigo-100 dark:bg-indigo-950 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800/60 font-semibold" title="Category is managed directly by the Joint Account">
                      🏦 Joint Account
                    </span>
                  {/if}

                  {#if hasActiveOverride}
                    <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[11px] bg-amber-100 dark:bg-amber-950/80 text-amber-800 dark:text-amber-300 border border-amber-300 dark:border-amber-700/80 font-bold" title="Temporary split override is active for {$selectedMonth}">
                      ⚡ Override Active ({$selectedMonth})
                    </span>
                  {/if}

                  <!-- Timeline & Overrides Button -->
                  <button
                    id="toggle-timeline-{split.category}"
                    type="button"
                    on:click={() => toggleTimeline(split.category)}
                    class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[11px] font-semibold transition-colors cursor-pointer border {isTimelineOpen ? 'bg-indigo-600 text-white border-indigo-600 shadow-xs' : hasActiveOverride ? 'bg-amber-50 dark:bg-amber-950/80 text-amber-800 dark:text-amber-300 border-amber-300 dark:border-amber-700/80 hover:bg-amber-100 dark:hover:bg-amber-900' : agreementsList.length > 0 ? 'bg-neutral-100 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-400 border-neutral-200 dark:border-neutral-700 hover:bg-neutral-200 dark:hover:bg-neutral-700' : 'bg-neutral-100 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-400 border-neutral-200 dark:border-neutral-700 hover:bg-neutral-200 dark:hover:bg-neutral-700'}"
                  >
                    <span>📅</span>
                    <span>{agreementsList.length > 0 ? `${agreementsList.length} override${agreementsList.length === 1 ? '' : 's'}` : 'Timeline'}</span>
                    <svg class="w-3 h-3 transition-transform {isTimelineOpen ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                    </svg>
                  </button>
                </div>

                <!-- Right: Quick Presets & Save/Reset Actions -->
                <div class="flex items-center gap-1.5 flex-wrap justify-start lg:justify-end">
                  {#if !isJointCategory && activeUsers.length > 0}
                    <div class="flex items-center gap-1 flex-wrap">
                      <span class="text-[10px] text-neutral-400 uppercase tracking-wider font-semibold mr-0.5">Quick:</span>
                      <button
                        type="button"
                        on:click={() => applyEvenSplit(split.category)}
                        class="px-2 py-0.5 rounded-md bg-neutral-100 dark:bg-neutral-800 hover:bg-neutral-200 dark:hover:bg-neutral-700 text-neutral-700 dark:text-neutral-300 text-[11px] font-medium transition-colors cursor-pointer border border-neutral-200 dark:border-transparent"
                        title="Split evenly among all members"
                      >
                        Even ({Math.floor(100 / activeUsers.length)}%)
                      </button>
                      {#each $jointAccounts || [] as acc}
                        {#if acc.member_names && acc.member_names.length > 0 && acc.member_names.length < activeUsers.length}
                          <button
                            type="button"
                            on:click={() => applySubgroupSplit(split.category, acc.member_names, 'even')}
                            class="px-2 py-0.5 rounded-md bg-indigo-50 dark:bg-indigo-950/70 hover:bg-indigo-100 dark:hover:bg-indigo-900/80 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800/50 text-[11px] font-medium transition-colors cursor-pointer"
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
                          class="px-2 py-0.5 rounded-md bg-indigo-50 dark:bg-indigo-950 hover:bg-indigo-100 dark:hover:bg-indigo-900 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800 text-[11px] font-medium transition-colors cursor-pointer"
                          title="Split equally among active selected subgroup ({customSelectedUsers.join(' & ')})"
                        >
                          Subgroup ({customSelectedUsers.join('+')})
                        </button>
                      {/if}
                      {#if totalSalary > 0}
                        <button
                          type="button"
                          on:click={() => resetToSalary(split.category)}
                          class="px-2 py-0.5 rounded-md bg-amber-50 dark:bg-neutral-800 hover:bg-amber-100 dark:hover:bg-neutral-700 text-amber-800 dark:text-amber-300 text-[11px] font-medium transition-colors cursor-pointer border border-amber-200 dark:border-transparent"
                          title="Distribute proportionally to monthly salary ratios"
                        >
                          Salary Ratio
                        </button>
                      {/if}
                    </div>
                  {/if}

                  <!-- Reset & Save Buttons -->
                  <div class="flex items-center gap-1.5 ml-auto lg:ml-2">
                    <button
                      id="reset-split-{split.category}"
                      type="button"
                      on:click={() => resetToSalary(split.category)}
                      disabled={isJointCategory || totalSalary === 0}
                      class="px-2.5 py-1 rounded-lg text-xs font-semibold text-neutral-600 dark:text-neutral-300 bg-neutral-100 dark:bg-neutral-800 hover:bg-neutral-200 dark:hover:bg-neutral-700 disabled:opacity-40 transition-colors cursor-pointer border border-neutral-200 dark:border-neutral-700"
                      title="Reset to salary ratio"
                    >
                      Reset
                    </button>
                    <button
                      id="save-split-{split.category}"
                      type="button"
                      on:click={() => save(split.category)}
                      disabled={isJointCategory || saving[split.category] || !sumOk}
                      class="px-3 py-1 rounded-lg text-xs font-semibold text-white bg-indigo-600 hover:bg-indigo-700 active:bg-indigo-800 disabled:opacity-40 transition-all shadow-xs cursor-pointer"
                    >
                      {saving[split.category] ? 'Saving…' : 'Save Agreement'}
                    </button>
                  </div>
                </div>
              </div>

              <!-- Row 2: Visual Distribution Bar & Status / Legend -->
              <div class="space-y-1">
                <div class="h-2 w-full bg-neutral-200 dark:bg-neutral-950 rounded-full overflow-hidden flex shadow-inner">
                  {#each activeUsers as u}
                    {@const pctVal = parseFloat(editValues[split.category]?.[u.name] || '0')}
                    {#if pctVal > 0}
                      <div
                        class="h-full transition-all duration-200"
                        style="width: {pctVal}%; background-color: {u.color}"
                        title="{u.name}: {pctVal}%"
                      ></div>
                    {/if}
                  {/each}
                </div>
                <div class="flex items-center justify-between text-[11px] text-neutral-500 dark:text-neutral-400">
                  <div class="flex items-center gap-3 flex-wrap">
                    {#each activeUsers as u}
                      {@const pctVal = parseFloat(editValues[split.category]?.[u.name] || '0')}
                      <span class="inline-flex items-center gap-1 font-semibold {pctVal > 0 ? '' : 'opacity-40'}">
                        <span class="w-2 h-2 rounded-full" style="background-color: {u.color}"></span>
                        <span style="color: {u.color}">{u.name}:</span>
                        <span class="tabular-nums font-bold text-neutral-800 dark:text-neutral-200">{pctVal}%</span>
                      </span>
                    {/each}
                  </div>

                  <div class="flex items-center gap-2">
                    {#if rowSuccess[split.category]}
                      <span class="text-[11px] text-emerald-600 dark:text-emerald-400 font-semibold">✓ Saved</span>
                    {/if}
                    {#if rowError[split.category]}
                      <span class="text-[11px] text-rose-700 dark:text-red-400 font-medium">{rowError[split.category]}</span>
                    {/if}
                    <span class="font-bold tabular-nums {sumOk ? 'text-emerald-600 dark:text-emerald-400' : 'text-amber-600 dark:text-amber-400'}">
                      Total: {sum}%
                    </span>
                  </div>
                </div>
              </div>

              <!-- Row 3: Compact Interactive Inputs / Slider -->
              {#if $splitInputMode === 'slider' && activeUsers.length === 2}
                {@const sliderVal = Math.round(parseFloat(editValues[split.category]?.[activeUsers[0].name] || '0'))}
                <div class="flex items-center gap-2.5 pt-0.5">
                  <span class="text-xs font-bold tabular-nums min-w-[32px]" style="color: {activeUsers[0].color}">{sliderVal}%</span>
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
                    class="flex-1 h-2 rounded-full cursor-pointer slider-split disabled:opacity-40"
                    style="background: linear-gradient(to right, {activeUsers[0].color} {sliderVal}%, {activeUsers[1].color} {sliderVal}%)"
                  />
                  <span class="text-xs font-bold tabular-nums min-w-[32px] text-right" style="color: {activeUsers[1].color}">{100 - sliderVal}%</span>
                </div>
              {:else if activeUsers.length > 0}
                <!-- Compact multi-user input strip -->
                <div class="flex items-center gap-2 flex-wrap pt-0.5">
                  {#each activeUsers as u}
                    <div class="inline-flex items-center gap-1.5 px-2 py-1 rounded-lg bg-neutral-50 dark:bg-neutral-950/80 border border-neutral-200 dark:border-neutral-800 text-xs">
                      <span class="w-2 h-2 rounded-full shrink-0" style="background-color: {u.color}"></span>
                      <span class="font-semibold text-[11px] truncate max-w-[80px]" style="color: {u.color}">{u.name}</span>
                      <div class="relative w-14">
                        <input
                          id="split-{u.name}-{split.category}"
                          type="number"
                          min="0"
                          max="100"
                          step="1"
                          disabled={isJointCategory}
                          bind:value={editValues[split.category][u.name]}
                          class="w-full bg-white dark:bg-neutral-900 border border-neutral-300 dark:border-neutral-700 rounded px-1.5 py-0.5 text-xs font-bold tabular-nums text-neutral-900 dark:text-neutral-100 disabled:opacity-40 focus:outline-none focus:ring-1 text-center"
                          style="--tw-ring-color: {u.color}"
                        />
                      </div>
                      <span class="text-[10px] text-neutral-400 font-bold">%</span>
                      <button
                        type="button"
                        on:click={() => setSinglePayer(split.category, u.name)}
                        disabled={isJointCategory}
                        class="text-[10px] text-neutral-400 hover:text-indigo-600 dark:hover:text-indigo-400 px-1 py-0.5 rounded hover:bg-neutral-200 dark:hover:bg-neutral-800 transition-colors disabled:opacity-30 cursor-pointer"
                        title="Assign 100% to {u.name}"
                      >
                        100%
                      </button>
                    </div>
                  {/each}
                </div>
              {/if}

              <!-- Row 4: Timeline Accordion Tray (only when expanded) -->
              {#if isTimelineOpen}
                <div class="pt-2.5 border-t border-neutral-200 dark:border-neutral-800/80 space-y-2">
                  <div class="flex items-center justify-between">
                    <span class="text-xs font-bold text-neutral-700 dark:text-neutral-300 flex items-center gap-1.5">
                      <span>📅</span> Split Agreements Timeline
                    </span>
                    <button
                      id="add-override-btn-{split.category}"
                      type="button"
                      on:click={() => openCreateOverrideModal(split.category)}
                      disabled={isJointCategory}
                      class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[11px] font-semibold text-indigo-700 dark:text-indigo-300 bg-indigo-50 dark:bg-indigo-950/60 hover:bg-indigo-100 dark:hover:bg-indigo-900 border border-indigo-200 dark:border-indigo-800/60 transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
                    >
                      <span>+</span>
                      <span>Add Temporary Override</span>
                    </button>
                  </div>

                  <!-- Baseline Info -->
                  <div class="flex items-center justify-between p-2 rounded-lg bg-neutral-100 dark:bg-neutral-900/60 border border-neutral-200 dark:border-neutral-800 text-xs">
                    <div class="flex items-center gap-2">
                      <span class="px-1.5 py-0.5 rounded text-[10px] font-bold bg-neutral-200 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300">
                        Baseline
                      </span>
                      <span class="text-neutral-500 font-mono text-[11px]">Ongoing (Default)</span>
                      {#if !hasActiveOverride}
                        <span class="px-1.5 py-0.2 rounded text-[10px] font-semibold bg-emerald-100 dark:bg-emerald-950 text-emerald-700 dark:text-emerald-300 border border-emerald-200 dark:border-emerald-800">
                          ✓ Active in {$selectedMonth || 'current period'}
                        </span>
                      {/if}
                    </div>
                    <div class="flex items-center gap-2.5">
                      {#each activeUsers as u}
                        {@const baselineAlloc = (baselineAgr?.allocations || []).find(a => a.user_name === u.name)}
                        {@const basePct = baselineAlloc ? Math.round(baselineAlloc.pct) : Math.round(parseFloat(editValues[split.category]?.[u.name] || '0'))}
                        <span class="font-medium text-[11px]" style="color: {u.color}">{u.name}: {basePct}%</span>
                      {/each}
                    </div>
                  </div>

                  <!-- Temporary Overrides List -->
                  {#each agreementsList as agr (agr.id)}
                    {@const isThisOverrideActive = isAgreementActiveForMonth(agr, $selectedMonth)}
                    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-1.5 p-2 rounded-lg {isThisOverrideActive ? 'bg-indigo-50/90 dark:bg-indigo-950/40 border-indigo-300 dark:border-indigo-700' : 'bg-neutral-50 dark:bg-neutral-900/40 border-neutral-200 dark:border-neutral-800/60 opacity-90'} border text-xs">
                      <div class="space-y-0.5">
                        <div class="flex items-center gap-1.5 flex-wrap">
                          <span class="px-1.5 py-0.5 rounded text-[10px] font-bold {isThisOverrideActive ? 'bg-indigo-600 text-white' : 'bg-neutral-200 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300'}">
                            Override
                          </span>
                          <span class="font-mono font-semibold text-neutral-900 dark:text-white text-[11px]">
                            {agr.start_date} → {agr.end_date || 'Ongoing'}
                          </span>
                          {#if isThisOverrideActive}
                            <span class="px-1.5 py-0.2 rounded text-[10px] font-bold bg-amber-100 dark:bg-amber-950 text-amber-800 dark:text-amber-300 border border-amber-300 dark:border-amber-800/60">
                              ⚡ Active in {$selectedMonth}
                            </span>
                          {:else}
                            <span class="px-1.5 py-0.2 rounded text-[10px] font-medium text-neutral-500 bg-neutral-100 dark:bg-neutral-800 border border-neutral-200 dark:border-neutral-700">
                              Inactive in {$selectedMonth}
                            </span>
                          {/if}
                          {#if agr.note}
                            <span class="text-neutral-500 dark:text-neutral-400 italic text-[11px]">
                              ({agr.note})
                            </span>
                          {/if}
                        </div>
                        <div class="flex items-center gap-2.5 flex-wrap pt-0.5">
                          {#each activeUsers as u}
                            {@const userAlloc = (agr.allocations || []).find(a => a.user_name === u.name)}
                            {@const pctVal = userAlloc ? Math.round(userAlloc.pct) : 0}
                            <span class="font-semibold text-[11px]" style="color: {u.color}">
                              {u.name}: {pctVal}%
                            </span>
                          {/each}
                        </div>
                      </div>

                      <div class="flex items-center gap-1 self-end sm:self-center">
                        <button
                          type="button"
                          on:click={() => openEditOverrideModal(agr)}
                          class="p-1 rounded text-neutral-400 hover:text-indigo-600 dark:hover:text-indigo-400 transition-colors cursor-pointer"
                          title="Edit override"
                        >
                          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
                          </svg>
                        </button>
                        <button
                          type="button"
                          on:click={() => handleDeleteOverride(agr.id, split.category)}
                          class="p-1 rounded text-neutral-400 hover:text-rose-600 dark:hover:text-red-400 transition-colors cursor-pointer"
                          title="Delete override"
                        >
                          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                          </svg>
                        </button>
                      </div>
                    </div>
                  {/each}
                </div>
              {/if}
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
                    <div class="inline-flex items-center gap-2 justify-end">
                      <button
                        type="button"
                        on:click={() => {
                          categorySearch = split.category;
                          section = 'agreements';
                        }}
                        class="text-xs text-indigo-600 dark:text-indigo-400 hover:underline font-semibold cursor-pointer"
                      >
                        Edit Split
                      </button>
                      <button
                        id="matrix-rename-{split.category}"
                        type="button"
                        on:click={() => openRenameModal(split.category, 'expense')}
                        disabled={isJoint}
                        class="text-xs text-neutral-500 hover:text-indigo-600 dark:hover:text-indigo-400 p-1 rounded transition-colors cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
                        title="Rename category"
                        aria-label="Rename {split.category}"
                      >
                        ✏️
                      </button>
                      <button
                        id="matrix-delete-{split.category}"
                        type="button"
                        on:click={() => openDeleteModal(split.category, 'expense')}
                        disabled={isJoint}
                        class="text-xs text-neutral-500 hover:text-rose-600 dark:hover:text-red-400 p-1 rounded transition-colors cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
                        title="Delete category"
                        aria-label="Delete {split.category}"
                      >
                        🗑️
                      </button>
                    </div>
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
              <div class="flex items-center gap-2 px-3 py-1.5 rounded-xl bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-700/80 text-xs font-semibold text-neutral-800 dark:text-neutral-200 shadow-sm">
                <span>{c.category}</span>
                <button
                  id="rename-income-cat-{c.category}"
                  type="button"
                  on:click={() => openRenameModal(c.category, 'income')}
                  class="w-5 h-5 rounded-lg flex items-center justify-center text-neutral-400 hover:text-indigo-600 dark:hover:text-indigo-400 hover:bg-indigo-50 dark:hover:bg-indigo-950/40 transition-colors cursor-pointer"
                  title="Rename income category"
                  aria-label="Rename {c.category}"
                >
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
                  </svg>
                </button>
                <button
                  id="delete-income-cat-{c.category}"
                  type="button"
                  on:click={() => openDeleteModal(c.category, 'income')}
                  class="w-5 h-5 rounded-lg flex items-center justify-center text-neutral-400 hover:text-rose-600 dark:hover:text-red-400 hover:bg-rose-50 dark:hover:bg-red-950/40 transition-colors cursor-pointer"
                  title="Remove income category"
                  aria-label="Delete {c.category}"
                >
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- ── Rename Category Modal ─────────────────────────────────────────────── -->
  {#if showRenameModal}
    <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-xs" role="dialog" aria-modal="true" aria-labelledby="rename-modal-title">
      <div class="card w-full max-w-md p-6 space-y-4 shadow-2xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900">
        <div class="flex items-center justify-between">
          <h3 id="rename-modal-title" class="text-base font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span>✏️</span> Rename {renameCategoryType === 'expense' ? 'Expense' : 'Income'} Category
          </h3>
          <button
            type="button"
            on:click={closeRenameModal}
            disabled={renameCategoryLoading}
            class="text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200 text-lg p-1 rounded-lg transition-colors cursor-pointer"
            aria-label="Close"
          >
            ×
          </button>
        </div>

        <p class="text-xs text-neutral-500 dark:text-neutral-400">
          Renaming <strong>{renameCategoryTarget}</strong> will automatically update all existing transactions, split rules, and budget limits.
        </p>

        <div>
          <label for="rename-category-input" class="block text-xs font-semibold uppercase tracking-wider text-neutral-700 dark:text-neutral-300 mb-1.5">
            New Category Name
          </label>
          <input
            id="rename-category-input"
            type="text"
            bind:value={renameCategoryNewName}
            class="input-field uppercase py-2.5 text-sm"
            placeholder="e.g. GROCERIES & FOOD"
            disabled={renameCategoryLoading}
            on:keydown={(e) => e.key === 'Enter' && handleRenameCategory()}
          />
        </div>

        {#if renameCategoryError}
          <div class="text-rose-700 dark:text-red-400 text-xs bg-rose-50 dark:bg-red-950/40 border border-rose-200 dark:border-red-800/60 rounded-xl p-3">
            {renameCategoryError}
          </div>
        {/if}

        <div class="flex items-center justify-end gap-2.5 pt-2">
          <button
            type="button"
            on:click={closeRenameModal}
            disabled={renameCategoryLoading}
            class="btn-secondary py-2 px-4 text-xs"
          >
            Cancel
          </button>
          <button
            id="confirm-rename-btn"
            type="button"
            on:click={handleRenameCategory}
            disabled={renameCategoryLoading || !renameCategoryNewName.trim()}
            class="btn-primary py-2 px-4 text-xs"
          >
            {renameCategoryLoading ? 'Saving…' : 'Save Name'}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- ── Delete Category Confirmation Modal ─────────────────────────────────── -->
  {#if showDeleteModal}
    <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-xs" role="dialog" aria-modal="true" aria-labelledby="delete-modal-title">
      <div class="card w-full max-w-md p-6 space-y-4 shadow-2xl border border-rose-200 dark:border-rose-900/40 bg-white dark:bg-neutral-900">
        <div class="flex items-center justify-between">
          <h3 id="delete-modal-title" class="text-base font-bold text-rose-600 dark:text-rose-400 flex items-center gap-2">
            <span>🗑️</span> Remove {deleteCategoryType === 'expense' ? 'Expense' : 'Income'} Category
          </h3>
          <button
            type="button"
            on:click={closeDeleteModal}
            disabled={deleteCategoryLoading}
            class="text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200 text-lg p-1 rounded-lg transition-colors cursor-pointer"
            aria-label="Close"
          >
            ×
          </button>
        </div>

        <p class="text-xs text-neutral-600 dark:text-neutral-300">
          Are you sure you want to completely remove the category <strong class="text-neutral-900 dark:text-white font-bold">{deleteCategoryTarget}</strong>?
        </p>

        <p class="text-[11px] text-neutral-500 dark:text-neutral-400">
          Note: If any expenses or recurring rules currently reference this category, deletion will be blocked to preserve data integrity.
        </p>

        {#if deleteCategoryError}
          <div class="text-rose-700 dark:text-red-400 text-xs bg-rose-50 dark:bg-red-950/40 border border-rose-200 dark:border-red-800/60 rounded-xl p-3">
            {deleteCategoryError}
          </div>
        {/if}

        <div class="flex items-center justify-end gap-2.5 pt-2">
          <button
            type="button"
            on:click={closeDeleteModal}
            disabled={deleteCategoryLoading}
            class="btn-secondary py-2 px-4 text-xs"
          >
            Cancel
          </button>
          <button
            id="confirm-delete-category-btn"
            type="button"
            on:click={handleConfirmDeleteCategory}
            disabled={deleteCategoryLoading}
            class="px-4 py-2 rounded-xl text-xs font-semibold text-white bg-rose-600 hover:bg-rose-700 active:bg-rose-800 transition-colors shadow-sm cursor-pointer disabled:opacity-50"
          >
            {deleteCategoryLoading ? 'Removing…' : 'Delete Category'}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- ═════════════════════════════════════════════════════════════════════════
       TEMPORARY SPLIT OVERRIDE MODAL
       ═════════════════════════════════════════════════════════════════════════ -->
  {#if showOverrideModal}
    {@const ovSum = overrideSum()}
    {@const ovSumOk = Math.abs(ovSum - 100) < 0.05}
    <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-neutral-900/60 backdrop-blur-xs">
      <div
        class="bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 rounded-2xl max-w-lg w-full p-6 space-y-4 shadow-2xl animate-in fade-in zoom-in-95 duration-150 max-h-[90vh] overflow-y-auto"
        role="dialog"
        aria-modal="true"
        aria-labelledby="override-modal-title"
      >
        <div class="flex items-center justify-between">
          <h3 id="override-modal-title" class="text-sm font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span>📅</span>
            <span>{overrideModalMode === 'create' ? 'Add Temporary Split Override' : 'Edit Split Override'}</span>
          </h3>
          <button
            type="button"
            on:click={closeOverrideModal}
            class="text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200 text-lg leading-none cursor-pointer"
            aria-label="Close"
          >
            ×
          </button>
        </div>

        <div class="space-y-3">
          <div>
            <span class="text-xs font-semibold text-neutral-500">Target Category:</span>
            <div class="mt-1 px-3 py-1.5 rounded-xl bg-neutral-100 dark:bg-neutral-800 font-bold text-sm text-neutral-900 dark:text-white border border-neutral-200 dark:border-neutral-700">
              {overrideCategory}
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label for="override-start-date" class="block text-xs font-semibold text-neutral-700 dark:text-neutral-300 mb-1">
                Start Date *
              </label>
              <input
                id="override-start-date"
                type="date"
                bind:value={overrideStartDate}
                class="input-field text-xs py-2"
                required
              />
            </div>
            <div>
              <label for="override-end-date" class="block text-xs font-semibold text-neutral-700 dark:text-neutral-300 mb-1">
                End Date (Inclusive)
              </label>
              <input
                id="override-end-date"
                type="date"
                bind:value={overrideEndDate}
                class="input-field text-xs py-2"
              />
            </div>
          </div>

          <div>
            <label for="override-note" class="block text-xs font-semibold text-neutral-700 dark:text-neutral-300 mb-1">
              Note / Reason (Optional)
            </label>
            <input
              id="override-note"
              type="text"
              bind:value={overrideNote}
              placeholder="e.g. Summer holiday guests, Parental leave"
              class="input-field text-xs py-2"
              maxlength="512"
            />
          </div>

          <!-- Allocation Inputs -->
          <div class="space-y-2 pt-2 border-t border-neutral-200 dark:border-neutral-800">
            <div class="flex items-center justify-between">
              <span class="text-xs font-semibold text-neutral-700 dark:text-neutral-300">
                Override Allocation Split
              </span>
              <span class="text-xs font-bold tabular-nums {ovSumOk ? 'text-emerald-600 dark:text-emerald-400' : 'text-amber-600 dark:text-amber-400'}">
                Total: {ovSum}%
              </span>
            </div>

            <div class="grid grid-cols-2 sm:grid-cols-3 gap-2.5">
              {#each activeUsers as u}
                <div class="bg-neutral-50 dark:bg-neutral-950 p-2 rounded-xl border border-neutral-200 dark:border-neutral-800 space-y-1">
                  <div class="flex items-center justify-between text-xs">
                    <span class="font-semibold" style="color: {u.color}">{u.name}</span>
                    <button
                      type="button"
                      on:click={() => {
                        for (const other of activeUsers) {
                          overrideAllocations[other.name] = other.name === u.name ? '100' : '0';
                        }
                        overrideAllocations = { ...overrideAllocations };
                      }}
                      class="text-[10px] text-neutral-400 hover:text-indigo-600 transition-colors"
                    >
                      100%
                    </button>
                  </div>
                  <div class="relative">
                    <input
                      id="override-split-{u.name}"
                      type="number"
                      min="0"
                      max="100"
                      step="1"
                      bind:value={overrideAllocations[u.name]}
                      class="w-full bg-white dark:bg-neutral-900 border border-neutral-300 dark:border-neutral-700 rounded-lg px-2 py-1 text-xs font-semibold tabular-nums text-neutral-900 dark:text-neutral-100"
                    />
                    <span class="absolute right-2 top-1/2 -translate-y-1/2 text-xs text-neutral-400 font-bold">%</span>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        </div>

        {#if overrideError}
          <div class="text-rose-700 dark:text-red-400 text-xs bg-rose-50 dark:bg-red-950/40 border border-rose-200 dark:border-red-800/60 rounded-xl p-3">
            {overrideError}
          </div>
        {/if}

        <div class="flex items-center justify-end gap-2.5 pt-2 border-t border-neutral-200 dark:border-neutral-800">
          <button
            type="button"
            on:click={closeOverrideModal}
            disabled={overrideLoading}
            class="btn-secondary py-2 px-4 text-xs"
          >
            Cancel
          </button>
          <button
            id="save-override-btn"
            type="button"
            on:click={handleSaveOverride}
            disabled={overrideLoading || !ovSumOk}
            class="btn-primary py-2 px-4 text-xs"
          >
            {overrideLoading ? 'Saving…' : 'Save Override'}
          </button>
        </div>
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
