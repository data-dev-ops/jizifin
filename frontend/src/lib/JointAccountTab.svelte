<script>
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import {
    jointAccount,
    jointCategories,
    jointDeposits,
    jointExpectedCosts,
    jointCorrections,
    jointDashboard,
    jointMonthlyDeposits,
    splits,
    users,
    selectedMonth,
    currencySymbol,
  } from './stores.js';
  import {
    createJointAccount,
    updateJointAccount,
    deleteJointAccount,
    fetchJointAccount,
    addJointCategory,
    removeJointCategory,
    setJointDeposits,
    setJointExpectedCosts,
    createJointCorrection,
    deleteJointCorrection,
    fetchJointDashboard,
    fetchJointCategories,
    fetchJointDeposits,
    fetchJointMonthlyDeposits,
    updateJointMonthlyDeposit,
    fetchJointExpectedCosts,
    fetchJointCorrections,
    settleJointAccount,
    fetchLatestSalaries,
    fetchIncomeByPerson,
    enc,
  } from './api.js';

  // ── reactive ──────────────────────────────────────────────────────────────
  $: ja          = $jointAccount;
  $: cats        = $jointCategories;    // [{enc, plain}]
  $: deposits    = $jointDeposits;      // [{user_name, amount_cents, day_of_month}]
  $: expected    = $jointExpectedCosts; // [{category, expected_cents}]
  $: corrections = $jointCorrections;  // [{id, amount_cents, correction_date, note}]
  $: dash        = $jointDashboard;
  $: activeUsers = $users.filter((u) => u.is_active);
  $: allSplits   = $splits;             // [{category, allocations}]
  $: sym         = $currencySymbol;
  $: month       = $selectedMonth;

  // ── local state ───────────────────────────────────────────────────────────
  let section = 'overview';  // overview | categories | deposits | expected | corrections | settle
  let errorMsg = '';
  let successMsg = '';

  // Setup form
  let setupName = '';
  let setupBalance = '0';
  let setupMargin = '10';
  let setupMode = 'even';

  // Correction form
  let corrAmount = '';
  let corrDate = '';
  let corrNote = '';
  let corrIsNeg = false;

  // Settle
  let settleMode = 'direct_pay'; // direct_pay | adjust_deposits
  let settleResult = null;
  let settling = false;

  // Deposit editing
  let editDeposits = [];
  let editingDeposits = false;

  // Expected cost editing
  let editExpected = [];
  let editingExpected = false;

  const PERSONAL_CATEGORIES = new Set(['PERSONAL COST', 'LEISURE', 'GIFT', 'PERSONAL']);

  // ── lifecycle ─────────────────────────────────────────────────────────────
  onMount(async () => {
    corrDate = new Date().toISOString().slice(0, 10);
    if ($jointAccount) {
      await fetchJointDashboard(month);
      await fetchJointMonthlyDeposits(month);
    }
  });

  $: if ($jointAccount && month) {
    fetchJointDashboard(month);
    fetchJointMonthlyDeposits(month);
  }

  let editingPaidUser = null;
  let customPaidAmount = '';

  async function handleToggleDepositPaid(uName, isPaid, customCents = null) {
    errorMsg = '';
    try {
      const existingLog = ($jointMonthlyDeposits || []).find((d) => d.user_name === uName);
      const sched = deposits.find((d) => d.user_name === uName);
      const scheduledCents = sched ? sched.amount_cents : 0;

      let actualCents = customCents !== null ? customCents : (existingLog && existingLog.actual_cents > 0 ? existingLog.actual_cents : scheduledCents);
      if (isPaid && actualCents <= 0) actualCents = scheduledCents;

      await updateJointMonthlyDeposit({
        month: $selectedMonth,
        user_name: uName,
        actual_cents: isPaid ? actualCents : 0,
        is_paid: isPaid,
        paid_date: new Date().toISOString().slice(0, 10),
      });

      editingPaidUser = null;
      customPaidAmount = '';
      await fetchJointDashboard($selectedMonth);
      await fetchJointMonthlyDeposits($selectedMonth);
      flash(true, isPaid ? `Deposit marked paid for ${uName}.` : `Deposit unmarked for ${uName}.`);
    } catch (e) {
      errorMsg = e.message;
    }
  }

  // Refresh dashboard when month changes
  $: if (ja && month) {
    fetchJointDashboard(month).catch(() => {});
  }

  // Seed editDeposits when entering section
  function enterDeposits() {
    editDeposits = activeUsers.map((u) => {
      const existing = deposits.find((d) => d.user_name === u.name);
      return {
        user_name: u.name,
        amount_cents: existing ? existing.amount_cents : 0,
        day_of_month: existing ? existing.day_of_month : 1,
      };
    });
    editingDeposits = false;
    section = 'deposits';
  }

  let costEstimationMode = 'categories'; // gross | categories
  let grossTotalEuros = '';

  $: sumCategoryExpectedCents = editExpected.reduce((s, e) => s + (e.expected_cents || 0), 0);

  function enterExpected() {
    editExpected = cats.map((c) => {
      const ex = expected.find((e) => e.category === c.plain);
      return {
        category: c.plain,
        expected_cents: ex ? ex.expected_cents : 0,
      };
    });
    costEstimationMode = (ja && ja.expected_total_cents !== null && ja.expected_total_cents !== undefined) ? 'gross' : 'categories';
    grossTotalEuros = (ja && ja.expected_total_cents !== null && ja.expected_total_cents !== undefined) ? (ja.expected_total_cents / 100).toFixed(2) : '';
    editingExpected = false;
    section = 'expected';
  }

  // ── helpers ───────────────────────────────────────────────────────────────
  const fmt = (cents) => `${sym}${(cents / 100).toFixed(2)}`;
  const pct = (actual, target) => target > 0 ? Math.min(100, Math.round(actual / target * 100)) : 0;

  function flash(ok, msg) {
    errorMsg = '';
    successMsg = '';
    if (ok) successMsg = msg;
    else errorMsg = msg;
    setTimeout(() => { successMsg = ''; errorMsg = ''; }, 3500);
  }

  // ── handlers ─────────────────────────────────────────────────────────────

  async function handleSetup() {
    errorMsg = '';
    try {
      await createJointAccount({
        name: setupName.trim(),
        balance_cents: Math.round(parseFloat(setupBalance) * 100) || 0,
        safety_margin_pct: parseInt(setupMargin) || 10,
        deposit_split_mode: setupMode,
      });
      // Enable default non-personal categories
      await enableDefaultCategories();
      await fetchJointDashboard(month);
      flash(true, 'Joint account created with default categories enabled.');
    } catch (e) {
      errorMsg = e.message;
    }
  }

  async function enableDefaultCategories() {
    for (const s of allSplits) {
      const isPersonal = PERSONAL_CATEGORIES.has(s.category.toUpperCase());
      const isAssigned = cats.some((c) => c.plain === s.category);
      if (!isPersonal && !isAssigned) {
        const encCat = await enc(s.category);
        await addJointCategory(encCat).catch(() => {});
      }
    }
    await fetchJointCategories();
  }

  async function handleDelete() {
    if (!confirm('Delete joint account and all its config?')) return;
    try {
      await deleteJointAccount();
      flash(true, 'Joint account removed.');
    } catch (e) {
      errorMsg = e.message;
    }
  }

  async function handleUpdateSettings() {
    errorMsg = '';
    try {
      await updateJointAccount({
        name: setupName.trim() || undefined,
        balance_cents: setupBalance !== '' ? Math.round(parseFloat(setupBalance) * 100) : undefined,
        safety_margin_pct: setupMargin !== '' ? parseInt(setupMargin) : undefined,
        deposit_split_mode: setupMode || undefined,
      });
      flash(true, 'Settings saved.');
    } catch (e) {
      errorMsg = e.message;
    }
  }

  async function handleToggleCategory(splitRow) {
    const existing = cats.find((c) => c.plain === splitRow.category);
    try {
      if (existing) {
        await removeJointCategory(existing.enc);
      } else {
        const encCat = await enc(splitRow.category);
        await addJointCategory(encCat);
      }
      await fetchJointExpectedCosts();
      enterExpected();
    } catch (e) {
      errorMsg = e.message;
    }
  }

  async function handleProposeDeposits() {
    errorMsg = '';
    try {
      const totalExpected = expected.reduce((sum, e) => sum + (e.expected_cents || 0), 0);
      const targetCents = dash?.target_deposit_cents || (totalExpected > 0
        ? Math.round(totalExpected * (1 + (ja?.safety_margin_pct || 10) / 100))
        : 0);

      if (targetCents === 0) {
        flash(false, 'Please set expected monthly costs first before proposing deposit amounts.');
        return;
      }

      let salaryRows = [];
      try {
        salaryRows = await fetchLatestSalaries();
      } catch {
        salaryRows = await fetchIncomeByPerson(month);
      }

      const salariesDict = {};
      for (const r of salaryRows || []) {
        const uName = r.who || r.user_name;
        const val = r.amount_cents || r.salary_cents || r.total_cents || 0;
        if (uName) salariesDict[uName] = val;
      }

      const totalHouseholdSalary = activeUsers.reduce((s, u) => s + (salariesDict[u.name] || 0), 0);

      editDeposits = activeUsers.map((u) => {
        const existing = deposits.find((d) => d.user_name === u.name);
        const userSalary = salariesDict[u.name] || 0;
        let proposedCents = 0;
        if (totalHouseholdSalary > 0) {
          proposedCents = Math.round(targetCents * (userSalary / totalHouseholdSalary));
        } else {
          proposedCents = Math.round(targetCents / activeUsers.length);
        }
        return {
          user_name: u.name,
          amount_cents: proposedCents,
          day_of_month: existing ? existing.day_of_month : 1,
        };
      });

      flash(true, 'Proposed deposit amounts calculated based on expected costs and salary ratios.');
    } catch (e) {
      errorMsg = e.message;
    }
  }

  let roundInterval = 10;

  function handleRoundOffDeposits() {
    const step = parseInt(roundInterval) || 10;
    editDeposits = editDeposits.map((dep) => {
      const currentEuros = dep.amount_cents / 100;
      if (currentEuros <= 0) return dep;
      const roundedEuros = Math.ceil(currentEuros / step) * step;
      return {
        ...dep,
        amount_cents: Math.round(roundedEuros * 100),
      };
    });
    flash(true, `Deposit amounts rounded up to the nearest €${step}.`);
  }

  async function handleSaveDeposits() {
    errorMsg = '';
    try {
      await setJointDeposits(editDeposits);
      editingDeposits = false;
      flash(true, 'Deposits saved.');
    } catch (e) {
      errorMsg = e.message;
    }
  }

  async function handleSaveExpected() {
    errorMsg = '';
    try {
      if (costEstimationMode === 'gross') {
        const grossCents = Math.round(parseFloat(grossTotalEuros) * 100) || 0;
        await updateJointAccount({ expected_total_cents: grossCents });
      } else {
        await setJointExpectedCosts(editExpected.filter((e) => e.expected_cents > 0));
        await updateJointAccount({ expected_total_cents: null });
      }
      editingExpected = false;
      flash(true, 'Expected costs saved.');
      await fetchJointDashboard(month);
    } catch (e) {
      errorMsg = e.message;
    }
  }

  async function handleAddCorrection() {
    errorMsg = '';
    if (!corrAmount || !corrDate) { errorMsg = 'Amount and date required.'; return; }
    try {
      const cents = Math.round(parseFloat(corrAmount) * 100) * (corrIsNeg ? -1 : 1);
      await createJointCorrection({ amount_cents: cents, correction_date: corrDate, note: corrNote.trim() || null });
      corrAmount = '';
      corrNote = '';
      flash(true, 'Correction logged.');
      await fetchJointDashboard(month);
    } catch (e) {
      errorMsg = e.message;
    }
  }

  async function handleDeleteCorrection(id) {
    if (!confirm('Delete this correction?')) return;
    try {
      await deleteJointCorrection(id);
      await fetchJointDashboard(month);
      flash(true, 'Correction deleted.');
    } catch (e) {
      errorMsg = e.message;
    }
  }

  async function handleSettle() {
    settling = true;
    errorMsg = '';
    settleResult = null;
    try {
      settleResult = await settleJointAccount({ mode: settleMode, month });
      if (settleMode === 'adjust_deposits') {
        await fetchJointDeposits();
      }
      flash(true, 'Settled.');
    } catch (e) {
      errorMsg = e.message;
    } finally {
      settling = false;
    }
  }

  $: if (ja) {
    setupName    = ja.name;
    setupBalance = (ja.balance_cents / 100).toFixed(2);
    setupMargin  = String(ja.safety_margin_pct);
    setupMode    = ja.deposit_split_mode;
  }
</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-xl sm:text-2xl font-bold text-white flex items-center gap-2.5">
        <span>🏦</span> Joint Account
      </h1>
      <p class="text-neutral-400 text-sm mt-1">Shared household funds, monthly deposit schedules, and settlements</p>
    </div>
  </div>

  {#if errorMsg}
    <div class="p-3.5 bg-red-950/60 border border-red-800/80 rounded-xl text-red-300 text-xs flex items-center justify-between gap-2 animate-fadeIn">
      <span>{errorMsg}</span>
      <button on:click={() => (errorMsg = '')} class="text-red-400 hover:text-red-200 text-sm font-bold flex-none px-1">×</button>
    </div>
  {/if}
  {#if successMsg}
    <div class="p-3.5 bg-emerald-950/60 border border-emerald-800/80 rounded-xl text-emerald-300 text-xs flex items-center justify-between gap-2 animate-fadeIn">
      <span>{successMsg}</span>
      <button on:click={() => (successMsg = '')} class="text-emerald-400 hover:text-emerald-200 text-sm font-bold flex-none px-1">×</button>
    </div>
  {/if}

  <!-- ── No account setup card ───────────────────────────────────────────── -->
  {#if !ja}
    <div class="card p-6 sm:p-8 text-center max-w-xl mx-auto space-y-5">
      <div class="w-14 h-14 rounded-2xl bg-indigo-500/10 border border-indigo-500/30 flex items-center justify-center text-indigo-400 text-2xl mx-auto">
        🏦
      </div>
      <div>
        <h3 class="text-lg font-bold text-white">Set Up Household Joint Account</h3>
        <p class="text-xs text-neutral-400 mt-1 max-w-md mx-auto">
          A joint account lets you track shared expenses outside of personal paybacks. Default non-personal categories will be linked automatically.
        </p>
      </div>

      <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-left">
        <div>
          <label for="ja-setup-name" class="block text-xs font-medium text-neutral-400 mb-1">Account Name</label>
          <input
            id="ja-setup-name"
            type="text"
            bind:value={setupName}
            placeholder="e.g. Household Fund"
            class="input-field"
          />
        </div>
        <div>
          <label for="ja-setup-balance" class="block text-xs font-medium text-neutral-400 mb-1">Initial Balance (€)</label>
          <input
            id="ja-setup-balance"
            type="number"
            step="0.01"
            bind:value={setupBalance}
            class="input-field"
          />
        </div>
        <div>
          <label for="ja-setup-margin" class="block text-xs font-medium text-neutral-400 mb-1">Safety Margin %</label>
          <input
            id="ja-setup-margin"
            type="number"
            min="0"
            max="100"
            bind:value={setupMargin}
            class="input-field"
          />
        </div>
        <div>
          <label for="ja-setup-mode" class="block text-xs font-medium text-neutral-400 mb-1">Deposit Split Mode</label>
          <select
            id="ja-setup-mode"
            bind:value={setupMode}
            class="select-field"
          >
            <option value="even">Even split</option>
            <option value="salary">Proportional to salary</option>
            <option value="manual">Manual</option>
          </select>
        </div>
      </div>

      <button
        id="ja-create-btn"
        on:click={handleSetup}
        disabled={!setupName.trim()}
        class="btn-primary w-full py-2.5"
      >
        Create Joint Account
      </button>
    </div>

  {:else}
    <!-- Sub-navigation tabs -->
    <nav class="flex flex-wrap gap-2 border-b border-neutral-800 pb-3">
      {#each [
        ['overview', '📊 Overview'],
        ['categories', '🏷️ Categories'],
        ['deposits', '💰 Deposits'],
        ['expected', '📋 Expected'],
        ['corrections', '✏️ Corrections'],
        ['settle', '⚖️ Settle'],
      ] as [id, label]}
        <button
          id="ja-nav-{id}"
          on:click={() => {
            if (id === 'deposits') { enterDeposits(); }
            else if (id === 'expected') { enterExpected(); }
            else { section = id; }
          }}
          class="px-4 py-2 rounded-xl text-xs font-semibold transition-all duration-200 cursor-pointer
                 {section === id
                   ? 'bg-indigo-600 text-white shadow-md shadow-indigo-900/40'
                   : 'bg-neutral-900 text-neutral-400 hover:text-neutral-200 border border-neutral-800'}"
        >
          {label}
        </button>
      {/each}
    </nav>

    <!-- OVERVIEW SECTION -->
    {#if section === 'overview'}
      <div class="space-y-6">
        <!-- Stat Cards Grid -->
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          <!-- Balance -->
          <div class="card border-indigo-500/30 bg-indigo-500/5 p-4 sm:p-5">
            <p class="text-xs font-medium text-neutral-400 uppercase tracking-wider">Current Balance</p>
            <p class="text-2xl sm:text-3xl font-bold text-white mt-1 tabular-nums">{fmt(ja.balance_cents)}</p>
            <p class="text-xs text-indigo-400 mt-1 font-medium">{ja.name}</p>
          </div>

          {#if dash}
            <!-- Spent this month -->
            <div class="bg-neutral-900 border border-neutral-800 rounded-2xl p-4 sm:p-5">
              <p class="text-xs font-medium text-neutral-400 uppercase tracking-wider">Spent ({dash.month})</p>
              <p class="text-2xl sm:text-3xl font-bold text-white mt-1 tabular-nums">{fmt(dash.actual_total_cents)}</p>
              <p class="text-xs text-neutral-500 mt-1">of {fmt(dash.expected_total_cents)} expected</p>
              <div class="w-full h-1.5 bg-neutral-800 rounded-full mt-3 overflow-hidden">
                <div class="h-full bg-indigo-500 rounded-full transition-all duration-500" style="width: {pct(dash.actual_total_cents, dash.expected_total_cents)}%"></div>
              </div>
            </div>

            <!-- Target deposit -->
            <div class="bg-neutral-900 border border-neutral-800 rounded-2xl p-4 sm:p-5">
              <p class="text-xs font-medium text-neutral-400 uppercase tracking-wider">Target Deposit / Mo</p>
              <p class="text-2xl sm:text-3xl font-bold text-white mt-1 tabular-nums">{fmt(dash.target_deposit_cents)}</p>
              <p class="text-xs text-neutral-500 mt-1">+{dash.safety_margin_pct}% safety margin</p>
            </div>

            <!-- Deposits received -->
            <div class="bg-neutral-900 border border-neutral-800 rounded-2xl p-4 sm:p-5">
              <p class="text-xs font-medium text-neutral-400 uppercase tracking-wider">Deposits Received</p>
              <p class="text-2xl sm:text-3xl font-bold text-white mt-1 tabular-nums">{fmt(dash.total_deposits_cents)}</p>
              <p class="text-xs mt-1 font-semibold {dash.total_deposits_cents >= dash.target_deposit_cents ? 'text-emerald-400' : 'text-amber-400'}">
                {dash.total_deposits_cents >= dash.target_deposit_cents ? '✓ Target met' : '⚠ Below target'}
              </p>
            </div>
          {/if}
        </div>

        <!-- Category breakdown -->
        {#if dash && dash.categories.length > 0}
          <div class="bg-neutral-900 border border-neutral-800 rounded-2xl p-5 sm:p-6 space-y-4">
            <h3 class="text-sm font-semibold text-neutral-200">Category Spending Progression — {dash.month}</h3>
            <div class="space-y-3">
              {#each dash.categories as row}
                {@const used = pct(row.actual_cents, row.expected_cents)}
                <div class="space-y-1">
                  <div class="flex justify-between text-xs font-medium">
                    <span class="text-neutral-300">{row.category}</span>
                    <span class="text-neutral-400 tabular-nums">{fmt(row.actual_cents)} / {row.expected_cents > 0 ? fmt(row.expected_cents) : '—'} ({used}%)</span>
                  </div>
                  <div class="w-full h-2 bg-neutral-800 rounded-full overflow-hidden">
                    <div
                      class="h-full rounded-full transition-all duration-500 {used >= 100 ? 'bg-red-500' : 'bg-indigo-500'}"
                      style="width: {Math.min(100, used)}%"
                    ></div>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Quick settings -->
        <div class="bg-neutral-900 border border-neutral-800 rounded-2xl p-5 sm:p-6 space-y-4">
          <h3 class="text-sm font-semibold text-neutral-200">Joint Account Settings</h3>
          <div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-3">
            <div>
              <label for="ja-edit-name" class="block text-xs font-medium text-neutral-400 mb-1">Account Name</label>
              <input id="ja-edit-name" type="text" bind:value={setupName} class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100" />
            </div>
            <div>
              <label for="ja-edit-balance" class="block text-xs font-medium text-neutral-400 mb-1">Current Balance (€)</label>
              <input id="ja-edit-balance" type="number" step="0.01" bind:value={setupBalance} class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100" />
            </div>
            <div>
              <label for="ja-edit-margin" class="block text-xs font-medium text-neutral-400 mb-1">Safety Margin %</label>
              <input id="ja-edit-margin" type="number" min="0" max="100" bind:value={setupMargin} class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100" />
            </div>
            <div>
              <label for="ja-edit-mode" class="block text-xs font-medium text-neutral-400 mb-1">Deposit Split Mode</label>
              <select id="ja-edit-mode" bind:value={setupMode} class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100">
                <option value="even">Even split</option>
                <option value="salary">Proportional to salary</option>
                <option value="manual">Manual</option>
              </select>
            </div>
          </div>
          <div class="flex gap-3 pt-2">
            <button id="ja-save-settings-btn" on:click={handleUpdateSettings} class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 text-white font-semibold text-xs rounded-xl shadow-md transition-colors">
              Save Settings
            </button>
            <button id="ja-delete-btn" on:click={handleDelete} class="px-4 py-2 bg-red-950/60 hover:bg-red-900/80 text-red-300 border border-red-800/80 font-semibold text-xs rounded-xl transition-colors">
              Delete Account
            </button>
          </div>
        </div>
      </div>
    {/if}

    <!-- CATEGORIES SECTION (Multi-column grid layout) -->
    {#if section === 'categories'}
      <div class="bg-neutral-900 border border-neutral-800 rounded-2xl p-5 sm:p-6 space-y-5">
        <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-800 pb-4">
          <div>
            <h3 class="text-base font-bold text-white">Joint Account Categories</h3>
            <p class="text-xs text-neutral-400 mt-0.5">
              Select categories paid directly from joint funds. Expenses in these categories are excluded from personal paybacks.
            </p>
          </div>
          <button
            on:click={enableDefaultCategories}
            class="px-3.5 py-2 bg-neutral-800 hover:bg-neutral-700 text-neutral-200 font-semibold text-xs rounded-xl border border-neutral-700 transition-colors flex-none"
          >
            ✓ Enable Non-Personal Defaults
          </button>
        </div>

        <!-- Multi-column category grid -->
        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-3">
          {#each allSplits as s}
            {@const active = cats.some((c) => c.plain === s.category)}
            {@const isPersonal = PERSONAL_CATEGORIES.has(s.category.toUpperCase())}
            <label
              for="ja-cat-{s.category}"
              class="flex items-center gap-3 p-3 rounded-xl border cursor-pointer select-none transition-all duration-150
                     {active
                       ? 'bg-indigo-950/60 border-indigo-500/80 text-white shadow-sm shadow-indigo-900/30'
                       : 'bg-neutral-950/80 border-neutral-800 text-neutral-400 hover:border-neutral-700 hover:text-neutral-200'}"
            >
              <input
                id="ja-cat-{s.category}"
                type="checkbox"
                checked={active}
                on:change={() => handleToggleCategory(s)}
                class="sr-only"
              />
              <div
                class="w-5 h-5 rounded-md border flex items-center justify-center flex-none transition-colors
                       {active ? 'bg-indigo-600 border-indigo-500 text-white' : 'bg-neutral-800 border-neutral-700'}"
              >
                {#if active}
                  <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
                  </svg>
                {/if}
              </div>
              <div class="min-w-0">
                <p class="text-xs font-semibold truncate">{s.category}</p>
                <p class="text-[10px] text-neutral-500 truncate">{isPersonal ? 'Personal Category' : 'Shared Category'}</p>
              </div>
            </label>
          {/each}
        </div>
      </div>
    {/if}

    <!-- DEPOSITS SECTION -->
    {#if section === 'deposits'}
      <div class="bg-neutral-900 border border-neutral-800 rounded-2xl p-5 sm:p-6 space-y-5">
        <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-800 pb-4">
          <div>
            <h3 class="text-base font-bold text-white">Monthly Deposit Schedule</h3>
            <p class="text-xs text-neutral-400 mt-0.5">Configure monthly contribution amounts and deposit days per member.</p>
          </div>
          <div class="flex flex-wrap items-center gap-2">
            <button
              id="ja-propose-deposits-btn"
              on:click={handleProposeDeposits}
              class="px-3.5 py-2 bg-indigo-950/60 hover:bg-indigo-900/80 text-indigo-300 font-semibold text-xs rounded-xl border border-indigo-700/60 transition-colors flex items-center gap-1.5 flex-none"
            >
              💡 Propose Amounts
            </button>
            <div class="flex items-center gap-1.5 bg-neutral-950/80 px-2.5 py-1.5 rounded-xl border border-neutral-800">
              <label for="ja-round-step" class="text-xs font-medium text-neutral-400">Interval €</label>
              <input
                id="ja-round-step"
                type="number"
                min="1"
                step="1"
                bind:value={roundInterval}
                class="w-14 bg-neutral-800 border border-neutral-700 rounded-lg px-2 py-1 text-xs text-neutral-100 font-bold tabular-nums focus:outline-none focus:border-indigo-500"
              />
            </div>
            <button
              id="ja-round-deposits-btn"
              on:click={handleRoundOffDeposits}
              class="px-3.5 py-2 bg-neutral-800 hover:bg-neutral-700 text-neutral-200 font-semibold text-xs rounded-xl border border-neutral-700 transition-colors flex items-center gap-1.5 flex-none"
            >
              ⬆️ Round Up
            </button>
          </div>
        </div>

        <div class="space-y-3 max-w-xl">
          {#each editDeposits as dep, i}
            <div class="flex items-center gap-4 p-4 rounded-xl bg-neutral-950/80 border border-neutral-800">
              <span class="w-24 font-bold text-sm text-neutral-200 flex-none">{dep.user_name}</span>
              <div class="flex-1">
                <label for="ja-dep-amount-{i}" class="block text-[11px] font-medium text-neutral-400 mb-1">Monthly Deposit (€)</label>
                <input
                  id="ja-dep-amount-{i}"
                  type="number"
                  step="0.01"
                  min="0"
                  value={(dep.amount_cents / 100).toFixed(2)}
                  on:input={(e) => (editDeposits[i].amount_cents = Math.round(parseFloat(e.target.value) * 100) || 0)}
                  on:change={(e) => (editDeposits[i].amount_cents = Math.round(parseFloat(e.target.value) * 100) || 0)}
                  class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100 focus:outline-none focus:border-indigo-500"
                />
              </div>
              <div class="w-28 flex-none">
                <label for="ja-dep-day-{i}" class="block text-[11px] font-medium text-neutral-400 mb-1">Day of Month</label>
                <input
                  id="ja-dep-day-{i}"
                  type="number"
                  min="1"
                  max="31"
                  bind:value={editDeposits[i].day_of_month}
                  class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100 focus:outline-none focus:border-indigo-500"
                />
              </div>
            </div>
          {/each}
        </div>

        <button
          id="ja-save-deposits-btn"
          on:click={handleSaveDeposits}
          class="px-4 py-2.5 bg-indigo-600 hover:bg-indigo-500 text-white font-semibold text-xs rounded-xl shadow-lg shadow-indigo-600/30 transition-all"
        >
          Save Deposit Schedule
        </button>

        <!-- ── Monthly Deposit Execution Log ── -->
        <div class="mt-8 pt-6 border-t border-neutral-800 space-y-4">
          <div>
            <h4 class="text-sm font-bold text-white flex items-center gap-2">
              <span>📅</span> Monthly Deposit Execution ({$selectedMonth})
            </h4>
            <p class="text-xs text-neutral-400 mt-0.5">
              Track whether members have paid their monthly deposits for {$selectedMonth}. If paid amounts differ from scheduled amounts, customize the exact paid figure below.
            </p>
          </div>

          <div class="grid grid-cols-1 gap-3 max-w-xl">
            {#each activeUsers as u}
              {@const log = ($jointMonthlyDeposits || []).find((d) => d.user_name === u.name)}
              {@const sched = deposits.find((d) => d.user_name === u.name)}
              {@const schedCents = sched ? sched.amount_cents : 0}
              {@const isPaid = log ? log.is_paid : false}
              {@const actualCents = log && isPaid ? log.actual_cents : schedCents}
              {@const isDiverted = isPaid && actualCents !== schedCents}
              {@const dueDay = sched ? sched.day_of_month : 1}
              {@const isEditingThis = editingPaidUser === u.name}

              <div class="p-4 rounded-xl bg-neutral-950 border border-neutral-800 space-y-3">
                <div class="flex items-center justify-between gap-3 flex-wrap">
                  <div class="flex items-center gap-2">
                    <div
                      class="w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold text-white shrink-0"
                      style="background-color: {u.color}"
                    >
                      {u.name.charAt(0).toUpperCase()}
                    </div>
                    <span class="font-bold text-sm text-neutral-200">{u.name}</span>
                  </div>

                  <!-- Status badge -->
                  <div class="flex items-center gap-2">
                    {#if isPaid}
                      {#if isDiverted}
                        <span class="px-2.5 py-1 rounded-lg text-xs font-semibold bg-amber-950/60 border border-amber-800/80 text-amber-300">
                          ✓ Paid €{(actualCents / 100).toFixed(2)} (Scheduled €{(schedCents / 100).toFixed(2)})
                        </span>
                      {:else}
                        <span class="px-2.5 py-1 rounded-lg text-xs font-semibold bg-emerald-950/60 border border-emerald-800/80 text-emerald-300">
                          ✓ Paid €{(actualCents / 100).toFixed(2)}
                        </span>
                      {/if}
                    {:else}
                      {#if log && log.status === 'pending'}
                        <span class="px-2.5 py-1 rounded-lg text-xs font-semibold bg-indigo-950/60 border border-indigo-800/80 text-indigo-300">
                          ⏳ Pending (Due Day {dueDay})
                        </span>
                      {:else}
                        <span class="px-2.5 py-1 rounded-lg text-xs font-semibold bg-amber-950/60 border border-amber-800/80 text-amber-300">
                          ⚠ Unpaid / Due Day {dueDay}
                        </span>
                      {/if}
                    {/if}
                  </div>
                </div>

                <!-- Action buttons / Edit inputs -->
                {#if isEditingThis}
                  <div class="flex items-center gap-2 pt-2 border-t border-neutral-900">
                    <div class="flex-1">
                      <label for="custom-paid-{u.name}" class="block text-[10px] font-medium text-neutral-400 mb-1">Actual Amount Paid (€)</label>
                      <input
                        id="custom-paid-{u.name}"
                        type="number"
                        step="0.01"
                        bind:value={customPaidAmount}
                        placeholder={(schedCents / 100).toFixed(2)}
                        class="w-full bg-neutral-900 border border-neutral-700 rounded-lg px-2.5 py-1.5 text-xs text-neutral-100 font-semibold tabular-nums focus:outline-none focus:border-indigo-500"
                      />
                    </div>
                    <button
                      id="save-custom-paid-{u.name}"
                      type="button"
                      on:click={() => handleToggleDepositPaid(u.name, true, Math.round(parseFloat(customPaidAmount || '0') * 100))}
                      class="px-3 py-2 bg-emerald-600 hover:bg-emerald-500 text-white font-semibold text-xs rounded-lg transition-colors self-end"
                    >
                      Confirm
                    </button>
                    <button
                      type="button"
                      on:click={() => { editingPaidUser = null; customPaidAmount = ''; }}
                      class="px-3 py-2 bg-neutral-800 hover:bg-neutral-700 text-neutral-300 font-semibold text-xs rounded-lg transition-colors self-end"
                    >
                      Cancel
                    </button>
                  </div>
                {:else}
                  <div class="flex items-center gap-2 pt-1">
                    {#if !isPaid}
                      <button
                        id="btn-mark-paid-{u.name}"
                        type="button"
                        on:click={() => handleToggleDepositPaid(u.name, true, schedCents)}
                        class="px-3 py-1.5 bg-emerald-950/80 hover:bg-emerald-900 border border-emerald-700/60 text-emerald-300 font-semibold text-xs rounded-lg transition-colors"
                      >
                        ✓ Mark Paid (€{(schedCents / 100).toFixed(2)})
                      </button>
                      <button
                        id="btn-custom-paid-{u.name}"
                        type="button"
                        on:click={() => { editingPaidUser = u.name; customPaidAmount = (schedCents / 100).toFixed(2); }}
                        class="px-3 py-1.5 bg-neutral-900 hover:bg-neutral-800 border border-neutral-700 text-neutral-300 font-semibold text-xs rounded-lg transition-colors"
                      >
                        Enter Custom Amount…
                      </button>
                    {:else}
                      <button
                        id="btn-edit-paid-{u.name}"
                        type="button"
                        on:click={() => { editingPaidUser = u.name; customPaidAmount = (actualCents / 100).toFixed(2); }}
                        class="px-3 py-1.5 bg-neutral-900 hover:bg-neutral-800 border border-neutral-700 text-neutral-300 font-semibold text-xs rounded-lg transition-colors"
                      >
                        Edit Paid Amount
                      </button>
                      <button
                        id="btn-unmark-paid-{u.name}"
                        type="button"
                        on:click={() => handleToggleDepositPaid(u.name, false, 0)}
                        class="px-3 py-1.5 bg-neutral-900 hover:bg-red-950/60 border border-neutral-700 hover:border-red-800 text-neutral-400 hover:text-red-300 font-semibold text-xs rounded-lg transition-colors"
                      >
                        Unmark
                      </button>
                    {/if}
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      </div>
    {/if}

    <!-- EXPECTED COSTS SECTION -->
    {#if section === 'expected'}
      <div class="bg-neutral-900 border border-neutral-800 rounded-2xl p-5 sm:p-6 space-y-6">
        <div>
          <h3 class="text-base font-bold text-white">Expected Monthly Costs</h3>
          <p class="text-xs text-neutral-400 mt-0.5">Configure expected monthly cost targets either as a single gross total or by category.</p>
        </div>

        <!-- Mode Selection Subsections -->
        <div class="space-y-4 max-w-2xl">
          <!-- Gross Cost Estimation Option -->
          <label class="flex items-start gap-3 p-4 rounded-xl border cursor-pointer transition-all
                        {costEstimationMode === 'gross' ? 'bg-indigo-950/60 border-indigo-500/80 text-white shadow-sm' : 'bg-neutral-950/80 border-neutral-800 text-neutral-400'}">
            <input
              id="radio-estimation-gross"
              type="radio"
              name="cost-estimation-mode"
              value="gross"
              checked={costEstimationMode === 'gross'}
              on:change={() => (costEstimationMode = 'gross')}
              class="mt-0.5 accent-indigo-600"
            />
            <div class="flex-1 space-y-2">
              <span class="font-bold text-sm text-neutral-100 block">Expected Gross Cost Estimation</span>
              <p class="text-xs text-neutral-400">Use a single overall gross estimate for the entire joint account.</p>
              {#if costEstimationMode === 'gross'}
                <div class="pt-2">
                  <label for="ja-gross-total" class="block text-xs font-medium text-neutral-300 mb-1">Expected Gross Total (€)</label>
                  <input
                    id="ja-gross-total"
                    type="number"
                    step="0.01"
                    min="0"
                    bind:value={grossTotalEuros}
                    placeholder="e.g. 3000.00"
                    class="w-full sm:w-64 bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100 focus:outline-none focus:border-indigo-500"
                  />
                </div>
              {/if}
            </div>
          </label>

          <hr class="border-neutral-800 my-2" />

          <!-- Category Specific Estimation Option -->
          <label class="flex items-start gap-3 p-4 rounded-xl border cursor-pointer transition-all
                        {costEstimationMode === 'categories' ? 'bg-indigo-950/60 border-indigo-500/80 text-white shadow-sm' : 'bg-neutral-950/80 border-neutral-800 text-neutral-400'}">
            <input
              id="radio-estimation-categories"
              type="radio"
              name="cost-estimation-mode"
              value="categories"
              checked={costEstimationMode === 'categories'}
              on:change={() => (costEstimationMode = 'categories')}
              class="mt-0.5 accent-indigo-600"
            />
            <div class="flex-1 space-y-2">
              <div class="flex items-center justify-between flex-wrap gap-2">
                <span class="font-bold text-sm text-neutral-100 block">Category Specific Cost Estimations</span>
                {#if costEstimationMode === 'categories'}
                  <span class="text-xs font-semibold text-indigo-400 bg-indigo-950/80 px-2.5 py-1 rounded-lg border border-indigo-800/60">
                    Estimated monthly total cost = {fmt(sumCategoryExpectedCents)}
                  </span>
                {/if}
              </div>
              <p class="text-xs text-neutral-400">Set individual monthly budget targets per joint category.</p>

              {#if costEstimationMode === 'categories'}
                <div class="pt-2">
                  {#if cats.length === 0}
                    <p class="text-xs text-neutral-500">No joint account categories assigned yet. Go to Categories sub-tab first.</p>
                  {:else}
                    <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-1">
                      {#each editExpected as ex, i}
                        <div class="p-3 rounded-xl bg-neutral-900 border border-neutral-800 space-y-1">
                          <span class="block text-xs font-semibold text-neutral-300">{ex.category}</span>
                          <label for="ja-exp-{i}" class="block text-[11px] text-neutral-500">Expected Cost (€)</label>
                          <input
                            id="ja-exp-{i}"
                            type="number"
                            step="0.01"
                            min="0"
                            value={(ex.expected_cents / 100).toFixed(2)}
                            on:change={(e) => (editExpected[i].expected_cents = Math.round(parseFloat(e.target.value) * 100) || 0)}
                            class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-1.5 text-sm text-neutral-100 focus:outline-none focus:border-indigo-500"
                          />
                        </div>
                      {/each}
                    </div>
                  {/if}
                </div>
              {/if}
            </div>
          </label>
        </div>

        <button
          id="ja-save-expected-btn"
          on:click={handleSaveExpected}
          class="px-4 py-2.5 bg-indigo-600 hover:bg-indigo-500 text-white font-semibold text-xs rounded-xl shadow-lg shadow-indigo-600/30 transition-all"
        >
          Save Expected Costs
        </button>
      </div>
    {/if}

    <!-- CORRECTIONS SECTION (Multi-column grid layout) -->
    {#if section === 'corrections'}
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <!-- Log Correction Card -->
        <div class="bg-neutral-900 border border-neutral-800 rounded-2xl p-5 sm:p-6 space-y-4">
          <h3 class="text-base font-bold text-white">Log Balance Correction</h3>
          
          <div class="space-y-3">
            <div>
              <span class="block text-xs font-medium text-neutral-400 mb-1.5">Correction Type</span>
              <div class="grid grid-cols-2 gap-2">
                <button
                  id="ja-corr-topup"
                  type="button"
                  on:click={() => (corrIsNeg = false)}
                  class="py-2 rounded-xl text-xs font-semibold border transition-all
                         {!corrIsNeg ? 'bg-emerald-950/80 border-emerald-500 text-emerald-300 shadow-sm' : 'bg-neutral-800 border-neutral-700 text-neutral-400 hover:text-neutral-200'}"
                >
                  + Deposit / Top-up
                </button>
                <button
                  id="ja-corr-withdraw"
                  type="button"
                  on:click={() => (corrIsNeg = true)}
                  class="py-2 rounded-xl text-xs font-semibold border transition-all
                         {corrIsNeg ? 'bg-red-950/80 border-red-500 text-red-300 shadow-sm' : 'bg-neutral-800 border-neutral-700 text-neutral-400 hover:text-neutral-200'}"
                >
                  − Withdrawal
                </button>
              </div>
            </div>

            <div>
              <label for="ja-corr-amount" class="block text-xs font-medium text-neutral-400 mb-1">Amount (€)</label>
              <input id="ja-corr-amount" type="number" step="0.01" min="0" bind:value={corrAmount} class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100" />
            </div>

            <div>
              <label for="ja-corr-date" class="block text-xs font-medium text-neutral-400 mb-1">Date</label>
              <input id="ja-corr-date" type="date" bind:value={corrDate} class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100" />
            </div>

            <div>
              <label for="ja-corr-note" class="block text-xs font-medium text-neutral-400 mb-1">Note (optional)</label>
              <input id="ja-corr-note" type="text" bind:value={corrNote} maxlength="512" placeholder="e.g. Monthly top-up" class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100" />
            </div>
          </div>

          <button
            id="ja-add-corr-btn"
            on:click={handleAddCorrection}
            class="w-full py-2.5 bg-indigo-600 hover:bg-indigo-500 text-white font-semibold text-xs rounded-xl shadow-lg shadow-indigo-600/30 transition-all"
          >
            Log Correction
          </button>
        </div>

        <!-- History Log Card -->
        <div class="bg-neutral-900 border border-neutral-800 rounded-2xl p-5 sm:p-6 space-y-4">
          <h3 class="text-base font-bold text-white">Correction Log History</h3>

          {#if corrections.length === 0}
            <p class="text-xs text-neutral-500">No balance corrections logged yet.</p>
          {:else}
            <div class="space-y-2 max-h-96 overflow-y-auto pr-1">
              {#each corrections as c}
                <div class="flex items-center justify-between p-3 rounded-xl bg-neutral-950/80 border border-neutral-800 text-xs">
                  <div>
                    <div class="font-bold tabular-nums {c.amount_cents >= 0 ? 'text-emerald-400' : 'text-red-400'}">
                      {c.amount_cents >= 0 ? '+' : ''}{fmt(c.amount_cents)}
                    </div>
                    <div class="text-neutral-400 mt-0.5">{c.note ?? '—'} ({c.correction_date})</div>
                  </div>
                  <button
                    id="ja-del-corr-{c.id}"
                    on:click={() => handleDeleteCorrection(c.id)}
                    class="p-1 text-neutral-500 hover:text-red-400 transition-colors"
                  >
                    ×
                  </button>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- SETTLE SECTION (Selectable checkmark cards instead of select dropdown) -->
    {#if section === 'settle'}
      <div class="bg-neutral-900 border border-neutral-800 rounded-2xl p-5 sm:p-6 space-y-6 max-w-2xl">
        <div>
          <h3 class="text-base font-bold text-white">Settle Joint Account ({month})</h3>
          <p class="text-xs text-neutral-400 mt-0.5">
            Compare actual spending against deposits collected for the month and choose a settlement method.
          </p>
        </div>

        <!-- Selectable Radio Cards for Settlement Modes -->
        <div class="space-y-3">
          <span class="block text-xs font-semibold text-neutral-400 uppercase tracking-wider">Select Settlement Method</span>
          
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <!-- Direct Pay Card -->
            <button
              type="button"
              id="settle-option-direct-pay"
              on:click={() => (settleMode = 'direct_pay')}
              class="text-left p-4 rounded-xl border transition-all relative flex flex-col justify-between space-y-2 cursor-pointer
                     {settleMode === 'direct_pay'
                       ? 'bg-indigo-950/60 border-indigo-500 text-white shadow-lg shadow-indigo-900/30'
                       : 'bg-neutral-950/80 border-neutral-800 text-neutral-400 hover:border-neutral-700 hover:text-neutral-200'}"
            >
              <div class="flex items-center justify-between">
                <span class="font-bold text-sm text-neutral-100 flex items-center gap-2">
                  💵 Direct Payment
                </span>
                <div class="w-5 h-5 rounded-full border flex items-center justify-center {settleMode === 'direct_pay' ? 'bg-indigo-600 border-indigo-500 text-white' : 'border-neutral-700'}">
                  {#if settleMode === 'direct_pay'}
                    <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
                    </svg>
                  {/if}
                </div>
              </div>
              <p class="text-xs text-neutral-400 leading-relaxed">
                Calculate the difference between deposits and actual spending, showing exact amounts for users to transfer directly.
              </p>
            </button>

            <!-- Adjust Deposits Card -->
            <button
              type="button"
              id="settle-option-adjust-deposits"
              on:click={() => (settleMode = 'adjust_deposits')}
              class="text-left p-4 rounded-xl border transition-all relative flex flex-col justify-between space-y-2 cursor-pointer
                     {settleMode === 'adjust_deposits'
                       ? 'bg-indigo-950/60 border-indigo-500 text-white shadow-lg shadow-indigo-900/30'
                       : 'bg-neutral-950/80 border-neutral-800 text-neutral-400 hover:border-neutral-700 hover:text-neutral-200'}"
            >
              <div class="flex items-center justify-between">
                <span class="font-bold text-sm text-neutral-100 flex items-center gap-2">
                  🔄 Adjust Future Deposits
                </span>
                <div class="w-5 h-5 rounded-full border flex items-center justify-center {settleMode === 'adjust_deposits' ? 'bg-indigo-600 border-indigo-500 text-white' : 'border-neutral-700'}">
                  {#if settleMode === 'adjust_deposits'}
                    <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
                    </svg>
                  {/if}
                </div>
              </div>
              <p class="text-xs text-neutral-400 leading-relaxed">
                Automatically recalculates per-user monthly deposit amounts to absorb any surplus or deficit over next month.
              </p>
            </button>
          </div>
        </div>

        <button
          id="ja-settle-btn"
          on:click={handleSettle}
          disabled={settling}
          class="w-full py-2.5 bg-indigo-600 hover:bg-indigo-500 text-white font-semibold text-xs rounded-xl shadow-lg shadow-indigo-600/30 transition-all disabled:opacity-50"
        >
          {settling ? 'Settling…' : 'Execute Settlement'}
        </button>

        {#if settleResult}
          <div class="p-4 rounded-xl bg-neutral-950/80 border border-neutral-800 space-y-2 animate-fadeIn">
            <h4 class="text-xs font-bold uppercase tracking-wider text-neutral-300">{settleResult.message ?? settleResult.mode}</h4>
            <p class="text-sm">
              Difference: <strong class={settleResult.difference_cents >= 0 ? 'text-emerald-400' : 'text-red-400'}>
                {settleResult.difference_cents >= 0 ? '+' : ''}{fmt(settleResult.difference_cents)}
              </strong>
            </p>
            {#if settleMode === 'direct_pay'}
              <p class="text-xs text-neutral-400">
                {settleResult.difference_cents < 0
                  ? `Deficit: Users need to top up ${fmt(-settleResult.difference_cents)} into the account.`
                  : `Surplus: ${fmt(settleResult.difference_cents)} remains in joint account.`}
              </p>
            {:else if settleResult.adjustments}
              <div class="space-y-1 pt-2">
                {#each settleResult.adjustments as adj}
                  <div class="flex justify-between text-xs py-1 border-b border-neutral-800">
                    <span class="text-neutral-300">{adj.user_name}</span>
                    <span class="font-medium">{fmt(adj.old_cents)} → <span class="text-emerald-400">{fmt(adj.new_cents)}</span></span>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  {/if}
</div>
