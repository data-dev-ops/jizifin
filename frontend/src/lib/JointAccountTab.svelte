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
    fetchJointExpectedCosts,
    fetchJointCorrections,
    settleJointAccount,
    enc,
  } from './api.js';

  // ── reactive ──────────────────────────────────────────────────────────────
  $: ja         = $jointAccount;
  $: cats       = $jointCategories;    // [{enc, plain}]
  $: deposits   = $jointDeposits;      // [{user_name, amount_cents, day_of_month}]
  $: expected   = $jointExpectedCosts; // [{category, expected_cents}]
  $: corrections = $jointCorrections;  // [{id, amount_cents, correction_date, note}]
  $: dash       = $jointDashboard;
  $: activeUsers = $users.filter((u) => u.is_active);
  $: allSplits  = $splits;             // [{category, allocations}]
  $: sym        = $currencySymbol;
  $: month      = $selectedMonth;

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
  let settleMode = 'direct_pay';
  let settleResult = null;
  let settling = false;

  // Deposit editing
  let editDeposits = [];
  let editingDeposits = false;

  // Expected cost editing
  let editExpected = [];
  let editingExpected = false;

  // ── lifecycle ─────────────────────────────────────────────────────────────
  onMount(async () => {
    corrDate = new Date().toISOString().slice(0, 10);
    if ($jointAccount) {
      await fetchJointDashboard(month);
    }
  });

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

  function enterExpected() {
    editExpected = cats.map((c) => {
      const ex = expected.find((e) => e.category === c.plain);
      return {
        category: c.plain,
        expected_cents: ex ? ex.expected_cents : 0,
      };
    });
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
      await fetchJointDashboard(month);
      flash(true, 'Joint account created.');
    } catch (e) {
      errorMsg = e.message;
    }
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
    // splitRow: { category (plain), allocations }
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
      await setJointExpectedCosts(editExpected.filter((e) => e.expected_cents > 0));
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

  // init setup form from existing account
  $: if (ja) {
    setupName    = ja.name;
    setupBalance = (ja.balance_cents / 100).toFixed(2);
    setupMargin  = String(ja.safety_margin_pct);
    setupMode    = ja.deposit_split_mode;
  }
</script>

<!-- ── wrapper ──────────────────────────────────────────────────────────────── -->
<div class="ja-tab">
  <h2 class="section-title">🏦 Joint Account</h2>

  {#if errorMsg}
    <div class="alert alert-error">{errorMsg}</div>
  {/if}
  {#if successMsg}
    <div class="alert alert-success">{successMsg}</div>
  {/if}

  <!-- ── No account: setup prompt ─────────────────────────────────────────── -->
  {#if !ja}
    <div class="card setup-card">
      <h3>Set up your joint account</h3>
      <p class="muted">A joint account lets you track shared expenses outside of personal paybacks.</p>
      <div class="form-grid">
        <label>
          Account name
          <input id="ja-setup-name" type="text" bind:value={setupName} placeholder="e.g. Household" />
        </label>
        <label>
          Current balance (€)
          <input id="ja-setup-balance" type="number" step="0.01" bind:value={setupBalance} />
        </label>
        <label>
          Safety margin %
          <input id="ja-setup-margin" type="number" min="0" max="100" bind:value={setupMargin} />
        </label>
        <label>
          Deposit split mode
          <select id="ja-setup-mode" bind:value={setupMode}>
            <option value="even">Even split</option>
            <option value="salary">Proportional to salary</option>
            <option value="manual">Manual</option>
          </select>
        </label>
      </div>
      <button id="ja-create-btn" class="btn btn-primary" on:click={handleSetup} disabled={!setupName.trim()}>
        Create Joint Account
      </button>
    </div>

  {:else}
    <!-- ── Sub-nav ──────────────────────────────────────────────────────────── -->
    <nav class="sub-nav">
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
          class="sub-nav-btn"
          class:active={section === id}
          on:click={() => {
            if (id === 'deposits') { enterDeposits(); }
            else if (id === 'expected') { enterExpected(); }
            else { section = id; }
          }}
        >
          {label}
        </button>
      {/each}
    </nav>

    <!-- ── OVERVIEW ─────────────────────────────────────────────────────────── -->
    {#if section === 'overview'}
      <div class="overview-grid">
        <!-- Balance card -->
        <div class="stat-card balance-card">
          <div class="stat-label">Current balance</div>
          <div class="stat-value">{fmt(ja.balance_cents)}</div>
          <div class="stat-sub">{ja.name}</div>
        </div>

        {#if dash}
          <!-- Month progress -->
          <div class="stat-card">
            <div class="stat-label">Spent this month</div>
            <div class="stat-value">{fmt(dash.actual_total_cents)}</div>
            <div class="stat-sub">of {fmt(dash.expected_total_cents)} expected</div>
            <div class="progress-bar-wrap">
              <div class="progress-bar" style="width:{pct(dash.actual_total_cents, dash.expected_total_cents)}%"></div>
            </div>
          </div>

          <!-- Deposit target -->
          <div class="stat-card">
            <div class="stat-label">Target deposit / month</div>
            <div class="stat-value">{fmt(dash.target_deposit_cents)}</div>
            <div class="stat-sub">+{dash.safety_margin_pct}% safety margin</div>
          </div>

          <!-- Deposits received -->
          <div class="stat-card">
            <div class="stat-label">Deposits this month</div>
            <div class="stat-value">{fmt(dash.total_deposits_cents)}</div>
            <div class="stat-sub {dash.total_deposits_cents >= dash.target_deposit_cents ? 'text-green' : 'text-amber'}">
              {dash.total_deposits_cents >= dash.target_deposit_cents ? '✓ On track' : '⚠ Below target'}
            </div>
          </div>

          <!-- Per-category progress chart ─────────────────────────────────── -->
          {#if dash.categories.length > 0}
            <div class="card cat-progress-card">
              <h4>Category breakdown — {dash.month}</h4>
              {#each dash.categories as row}
                {@const used = pct(row.actual_cents, row.expected_cents)}
                <div class="cat-row">
                  <div class="cat-name">{row.category}</div>
                  <div class="cat-bar-wrap">
                    <div
                      class="cat-bar"
                      class:over={used >= 100}
                      style="width:{Math.min(100, used)}%"
                    ></div>
                  </div>
                  <div class="cat-amounts">
                    {fmt(row.actual_cents)} / {row.expected_cents > 0 ? fmt(row.expected_cents) : '—'}
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        {/if}
      </div>

      <!-- Quick settings ──────────────────────────────────────────────────── -->
      <div class="card settings-card">
        <h4>Settings</h4>
        <div class="form-grid">
          <label>
            Account name
            <input id="ja-edit-name" type="text" bind:value={setupName} />
          </label>
          <label>
            Current balance (€)
            <input id="ja-edit-balance" type="number" step="0.01" bind:value={setupBalance} />
          </label>
          <label>
            Safety margin %
            <input id="ja-edit-margin" type="number" min="0" max="100" bind:value={setupMargin} />
          </label>
          <label>
            Deposit split mode
            <select id="ja-edit-mode" bind:value={setupMode}>
              <option value="even">Even split</option>
              <option value="salary">Proportional to salary</option>
              <option value="manual">Manual</option>
            </select>
          </label>
        </div>
        <div class="row-btns">
          <button id="ja-save-settings-btn" class="btn btn-primary" on:click={handleUpdateSettings}>
            Save settings
          </button>
          <button id="ja-delete-btn" class="btn btn-danger" on:click={handleDelete}>
            Delete account
          </button>
        </div>
      </div>
    {/if}

    <!-- ── CATEGORIES ────────────────────────────────────────────────────────── -->
    {#if section === 'categories'}
      <div class="card">
        <h4>Joint account categories</h4>
        <p class="muted">Expenses in these categories are paid from the joint account and excluded from personal paybacks.</p>
        <div class="cat-list">
          {#each allSplits as s}
            {@const active = cats.some((c) => c.plain === s.category)}
            <label class="cat-toggle" for="ja-cat-{s.category}">
              <input
                id="ja-cat-{s.category}"
                type="checkbox"
                checked={active}
                on:change={() => handleToggleCategory(s)}
              />
              <span class="cat-chip" class:active>{s.category}</span>
            </label>
          {/each}
        </div>
      </div>
    {/if}

    <!-- ── DEPOSITS ──────────────────────────────────────────────────────────── -->
    {#if section === 'deposits'}
      <div class="card">
        <h4>Monthly deposit schedule</h4>
        <p class="muted">How much each person deposits and on which day of the month.</p>
        {#each editDeposits as dep, i}
          <div class="deposit-row">
            <span class="dep-name">{dep.user_name}</span>
            <label>
              Amount (€)
              <input
                id="ja-dep-amount-{i}"
                type="number"
                step="0.01"
                min="0"
                value={(dep.amount_cents / 100).toFixed(2)}
                on:change={(e) => (editDeposits[i].amount_cents = Math.round(parseFloat(e.target.value) * 100) || 0)}
              />
            </label>
            <label>
              Day of month
              <input
                id="ja-dep-day-{i}"
                type="number"
                min="1"
                max="31"
                bind:value={editDeposits[i].day_of_month}
              />
            </label>
          </div>
        {/each}
        <button id="ja-save-deposits-btn" class="btn btn-primary" on:click={handleSaveDeposits}>
          Save deposit schedule
        </button>
      </div>
    {/if}

    <!-- ── EXPECTED COSTS ─────────────────────────────────────────────────────── -->
    {#if section === 'expected'}
      <div class="card">
        <h4>Expected monthly costs per category</h4>
        <p class="muted">Used to compute the monthly deposit target and the progress chart.</p>
        {#if cats.length === 0}
          <p class="muted">No categories assigned yet. Go to Categories first.</p>
        {:else}
          {#each editExpected as ex, i}
            <div class="deposit-row">
              <span class="dep-name">{ex.category}</span>
              <label>
                Expected (€)
                <input
                  id="ja-exp-{i}"
                  type="number"
                  step="0.01"
                  min="0"
                  value={(ex.expected_cents / 100).toFixed(2)}
                  on:change={(e) => (editExpected[i].expected_cents = Math.round(parseFloat(e.target.value) * 100) || 0)}
                />
              </label>
            </div>
          {/each}
          <button id="ja-save-expected-btn" class="btn btn-primary" on:click={handleSaveExpected}>
            Save expected costs
          </button>
        {/if}
      </div>
    {/if}

    <!-- ── CORRECTIONS ────────────────────────────────────────────────────────── -->
    {#if section === 'corrections'}
      <div class="card">
        <h4>Log a balance correction</h4>
        <div class="form-grid">
          <label>
            Type
            <div class="toggle-row">
              <button
                id="ja-corr-topup"
                class="btn btn-sm"
                class:active={!corrIsNeg}
                on:click={() => (corrIsNeg = false)}
              >+ Top-up</button>
              <button
                id="ja-corr-withdraw"
                class="btn btn-sm"
                class:active={corrIsNeg}
                on:click={() => (corrIsNeg = true)}
              >− Withdrawal</button>
            </div>
          </label>
          <label>
            Amount (€)
            <input id="ja-corr-amount" type="number" step="0.01" min="0" bind:value={corrAmount} />
          </label>
          <label>
            Date
            <input id="ja-corr-date" type="date" bind:value={corrDate} />
          </label>
          <label>
            Note (optional)
            <input id="ja-corr-note" type="text" bind:value={corrNote} maxlength="512" />
          </label>
        </div>
        <button id="ja-add-corr-btn" class="btn btn-primary" on:click={handleAddCorrection}>
          Log correction
        </button>
      </div>

      <!-- History ───────────────────────────────────────────────────────────── -->
      <div class="card">
        <h4>Correction history</h4>
        {#if corrections.length === 0}
          <p class="muted">No corrections logged.</p>
        {:else}
          <table class="ja-table">
            <thead>
              <tr><th>Date</th><th>Amount</th><th>Note</th><th></th></tr>
            </thead>
            <tbody>
              {#each corrections as c}
                <tr>
                  <td>{c.correction_date}</td>
                  <td class={c.amount_cents >= 0 ? 'text-green' : 'text-red'}>
                    {c.amount_cents >= 0 ? '+' : ''}{fmt(c.amount_cents)}
                  </td>
                  <td>{c.note ?? '—'}</td>
                  <td>
                    <button
                      id="ja-del-corr-{c.id}"
                      class="btn btn-xs btn-danger"
                      on:click={() => handleDeleteCorrection(c.id)}
                    >×</button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>
    {/if}

    <!-- ── SETTLE ─────────────────────────────────────────────────────────────── -->
    {#if section === 'settle'}
      <div class="card">
        <h4>Settle balance for {month}</h4>
        <p class="muted">Compare deposits collected with actual spending and resolve any difference.</p>
        <div class="form-grid">
          <label>
            Settlement mode
            <select id="ja-settle-mode" bind:value={settleMode}>
              <option value="direct_pay">Direct payment — show difference to transfer</option>
              <option value="adjust_deposits">Adjust deposits — modify next month's amounts</option>
            </select>
          </label>
        </div>
        <button id="ja-settle-btn" class="btn btn-primary" on:click={handleSettle} disabled={settling}>
          {settling ? 'Settling…' : 'Settle'}
        </button>

        {#if settleResult}
          <div class="settle-result card">
            <h5>{settleResult.message ?? settleResult.mode}</h5>
            <p>
              Difference: <strong class={settleResult.difference_cents >= 0 ? 'text-green' : 'text-red'}>
                {settleResult.difference_cents >= 0 ? '+' : ''}{fmt(settleResult.difference_cents)}
              </strong>
            </p>
            {#if settleMode === 'direct_pay'}
              {#if settleResult.difference_cents < 0}
                <p>Deficit: users need to pay <strong>{fmt(-settleResult.difference_cents)}</strong> into the account.</p>
              {:else}
                <p>Surplus: <strong>{fmt(settleResult.difference_cents)}</strong> remains in the account — no action needed.</p>
              {/if}
            {:else if settleResult.adjustments}
              <table class="ja-table">
                <thead><tr><th>User</th><th>Old deposit</th><th>New deposit</th></tr></thead>
                <tbody>
                  {#each settleResult.adjustments as adj}
                    <tr>
                      <td>{adj.user_name}</td>
                      <td>{fmt(adj.old_cents)}</td>
                      <td class={adj.new_cents > adj.old_cents ? 'text-red' : 'text-green'}>{fmt(adj.new_cents)}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  {/if}
</div>

<style>
  .ja-tab {
    max-width: 900px;
    margin: 0 auto;
    padding: 1.5rem 1rem;
  }

  .section-title {
    font-size: 1.4rem;
    font-weight: 700;
    margin-bottom: 1.25rem;
    color: var(--color-text-primary, #e2e8f0);
  }

  .alert {
    padding: 0.75rem 1rem;
    border-radius: 0.5rem;
    margin-bottom: 1rem;
    font-size: 0.9rem;
  }
  .alert-error   { background: rgba(239,68,68,0.15); color: #f87171; border: 1px solid rgba(239,68,68,0.3); }
  .alert-success { background: rgba(52,211,153,0.12); color: #6ee7b7; border: 1px solid rgba(52,211,153,0.25); }

  .card {
    background: var(--color-surface, #1e293b);
    border: 1px solid var(--color-border, rgba(255,255,255,0.07));
    border-radius: 0.75rem;
    padding: 1.25rem;
    margin-bottom: 1rem;
  }

  .setup-card { text-align: center; }
  .setup-card h3 { font-size: 1.15rem; font-weight: 700; margin-bottom: 0.5rem; }

  .muted { color: var(--color-text-muted, #94a3b8); font-size: 0.875rem; margin-bottom: 0.75rem; }

  .form-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 0.75rem;
    margin-bottom: 1rem;
  }
  label { display: flex; flex-direction: column; gap: 0.3rem; font-size: 0.85rem; color: var(--color-text-muted, #94a3b8); }
  input, select {
    background: var(--color-input-bg, #0f172a);
    border: 1px solid var(--color-border, rgba(255,255,255,0.1));
    border-radius: 0.4rem;
    color: var(--color-text-primary, #e2e8f0);
    padding: 0.45rem 0.65rem;
    font-size: 0.9rem;
    width: 100%;
  }

  .btn {
    padding: 0.5rem 1.1rem;
    border-radius: 0.4rem;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    border: none;
    transition: opacity 0.15s;
  }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-primary { background: #6366f1; color: #fff; }
  .btn-primary:hover:not(:disabled) { background: #4f46e5; }
  .btn-danger  { background: rgba(239,68,68,0.2); color: #f87171; border: 1px solid rgba(239,68,68,0.3); }
  .btn-danger:hover  { background: rgba(239,68,68,0.35); }
  .btn-sm  { padding: 0.3rem 0.75rem; font-size: 0.8rem; }
  .btn-xs  { padding: 0.15rem 0.5rem; font-size: 0.75rem; }

  /* Sub-nav */
  .sub-nav {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-bottom: 1.25rem;
  }
  .sub-nav-btn {
    padding: 0.4rem 0.85rem;
    border-radius: 2rem;
    border: 1px solid var(--color-border, rgba(255,255,255,0.1));
    background: transparent;
    color: var(--color-text-muted, #94a3b8);
    font-size: 0.82rem;
    cursor: pointer;
    transition: all 0.15s;
  }
  .sub-nav-btn.active,
  .sub-nav-btn:hover { background: #6366f1; color: #fff; border-color: #6366f1; }

  /* Overview grid */
  .overview-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 0.75rem;
    margin-bottom: 1rem;
  }
  .stat-card {
    background: var(--color-surface, #1e293b);
    border: 1px solid var(--color-border, rgba(255,255,255,0.07));
    border-radius: 0.75rem;
    padding: 1rem 1.1rem;
  }
  .balance-card { border-color: rgba(99,102,241,0.4); background: rgba(99,102,241,0.08); }
  .stat-label { font-size: 0.78rem; color: var(--color-text-muted, #94a3b8); margin-bottom: 0.25rem; }
  .stat-value { font-size: 1.5rem; font-weight: 700; color: var(--color-text-primary, #e2e8f0); }
  .stat-sub   { font-size: 0.8rem; color: var(--color-text-muted, #64748b); margin-top: 0.15rem; }

  .progress-bar-wrap {
    height: 6px;
    background: rgba(255,255,255,0.08);
    border-radius: 3px;
    margin-top: 0.6rem;
    overflow: hidden;
  }
  .progress-bar {
    height: 100%;
    background: #6366f1;
    border-radius: 3px;
    transition: width 0.4s;
  }

  /* Category progress chart */
  .cat-progress-card { grid-column: 1 / -1; }
  .cat-progress-card h4 { font-size: 0.95rem; font-weight: 600; margin-bottom: 0.75rem; }
  .cat-row {
    display: grid;
    grid-template-columns: 160px 1fr 120px;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.55rem;
  }
  .cat-name { font-size: 0.82rem; color: var(--color-text-muted, #94a3b8); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .cat-bar-wrap { height: 8px; background: rgba(255,255,255,0.06); border-radius: 4px; overflow: hidden; }
  .cat-bar { height: 100%; background: #6366f1; border-radius: 4px; transition: width 0.4s; }
  .cat-bar.over { background: #ef4444; }
  .cat-amounts { font-size: 0.78rem; color: var(--color-text-muted, #94a3b8); text-align: right; }

  /* Settings card */
  .settings-card h4 { font-size: 0.95rem; font-weight: 600; margin-bottom: 0.75rem; }
  .row-btns { display: flex; gap: 0.5rem; flex-wrap: wrap; }

  /* Category toggles */
  .cat-list { display: flex; flex-wrap: wrap; gap: 0.5rem; margin-top: 0.75rem; }
  .cat-toggle { display: flex; align-items: center; gap: 0.35rem; cursor: pointer; }
  .cat-toggle input { width: auto; accent-color: #6366f1; }
  .cat-chip {
    padding: 0.3rem 0.75rem;
    border-radius: 2rem;
    background: rgba(255,255,255,0.05);
    border: 1px solid rgba(255,255,255,0.1);
    font-size: 0.82rem;
    color: var(--color-text-muted, #94a3b8);
    transition: all 0.15s;
  }
  .cat-chip.active { background: rgba(99,102,241,0.2); border-color: #6366f1; color: #a5b4fc; }

  /* Deposit rows */
  .deposit-row {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 0.75rem;
    flex-wrap: wrap;
  }
  .dep-name { min-width: 100px; font-weight: 600; font-size: 0.9rem; }

  /* Toggle row */
  .toggle-row { display: flex; gap: 0.4rem; margin-top: 0.2rem; }
  .btn.active { background: #6366f1; color: #fff; }

  /* Table */
  .ja-table { width: 100%; border-collapse: collapse; font-size: 0.875rem; }
  .ja-table th { color: var(--color-text-muted, #94a3b8); text-align: left; padding: 0.4rem 0.5rem; border-bottom: 1px solid rgba(255,255,255,0.07); }
  .ja-table td { padding: 0.45rem 0.5rem; border-bottom: 1px solid rgba(255,255,255,0.04); }

  .text-green { color: #6ee7b7; }
  .text-amber { color: #fcd34d; }
  .text-red   { color: #f87171; }

  .settle-result { margin-top: 1rem; }
  .settle-result h5 { font-weight: 600; margin-bottom: 0.5rem; }
</style>
