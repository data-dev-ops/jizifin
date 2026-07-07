<script>
  import { onMount } from 'svelte';
  import { budgets, splits, selectedMonth } from './stores.js';
  import { upsertBudget, deleteBudget, fetchBudgetAnalytics } from './api.js';

  let budgetStatus = []; // BudgetStatusRow[]
  let editMap = {};      // category+month → pending cents input
  let saving = {};
  let error = '';
  let newForm = { category: '', month: 'ALL', limit_euros: '' };
  let addError = '';
  let addSaving = false;

  $: monthStr = $selectedMonth;

  async function loadStatus() {
    try {
      budgetStatus = await fetchBudgetAnalytics(monthStr);
    } catch (e) {
      error = e.message;
    }
  }

  onMount(loadStatus);
  $: monthStr, loadStatus();

  function statusColor(pct) {
    if (pct >= 90) return 'red';
    if (pct >= 70) return 'yellow';
    return 'green';
  }

  function editKey(cat, month) { return `${cat}::${month}`; }

  function startEdit(b) {
    editMap[editKey(b.category, b.month)] = (b.limit_cents / 100).toFixed(2);
  }

  async function saveEdit(b) {
    const key = editKey(b.category, b.month);
    const limit_cents = Math.round(parseFloat(editMap[key]) * 100);
    if (isNaN(limit_cents) || limit_cents < 0) return;
    saving[key] = true;
    try {
      await upsertBudget({ category: b.category, month: b.month, limit_cents });
      delete editMap[key];
      editMap = editMap;
      await loadStatus();
    } catch (e) {
      error = e.message;
    } finally {
      saving[key] = false;
    }
  }

  async function handleDelete(b) {
    try {
      await deleteBudget(b.category, b.month);
      await loadStatus();
    } catch (e) {
      error = e.message;
    }
  }

  async function handleAdd() {
    addError = '';
    const limit_cents = Math.round(parseFloat(newForm.limit_euros) * 100);
    if (!newForm.category || isNaN(limit_cents) || limit_cents < 0) {
      addError = 'Choose a category and enter a valid amount.';
      return;
    }
    addSaving = true;
    try {
      await upsertBudget({ category: newForm.category, month: newForm.month, limit_cents });
      newForm = { category: '', month: 'ALL', limit_euros: '' };
      await loadStatus();
    } catch (e) {
      addError = e.message;
    } finally {
      addSaving = false;
    }
  }
</script>

<div class="budget-manager">
  <h2 class="section-title">Budget Manager</h2>
  <p class="section-sub">
    Set monthly spending limits per category. Use <code>ALL</code> for a standing default;
    a YYYY-MM entry overrides it for that specific month.
  </p>

  {#if error}
    <div class="error-banner">{error}</div>
  {/if}

  <!-- ── Status bars for selected month ─────────────────── -->
  <div class="status-section">
    <h3 class="sub-head">Spending vs. Limits — {monthStr}</h3>
    {#if budgetStatus.length === 0}
      <div class="empty-state">No budget data for this month.</div>
    {:else}
      <div class="status-grid">
        {#each budgetStatus as row}
          {@const color = statusColor(row.pct_used)}
          {@const barPct = Math.min(row.pct_used, 100)}
          <div class="status-card">
            <div class="card-header">
              <span class="cat-name">{row.category}</span>
              <span class="amounts">
                €{(row.actual_cents / 100).toFixed(0)}
                {#if row.limit_cents > 0}
                  / €{(row.limit_cents / 100).toFixed(0)}
                {:else}
                  <span class="no-limit">(no limit)</span>
                {/if}
              </span>
            </div>
            {#if row.limit_cents > 0}
              <div class="progress-track">
                <div
                  class="progress-bar {color}"
                  style="width:{barPct}%"
                ></div>
              </div>
              <div class="pct-label {color}">{row.pct_used.toFixed(1)}%</div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- ── Configured budget rows ─────────────────────────── -->
  <div class="config-section">
    <h3 class="sub-head">Configured Limits</h3>
    {#if $budgets.length === 0}
      <div class="empty-state">No limits configured.</div>
    {:else}
      <div class="table-wrap">
        <table class="budget-table">
          <thead>
            <tr>
              <th>Category</th>
              <th>Month</th>
              <th>Limit (€)</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {#each $budgets as b}
              {@const key = editKey(b.category, b.month)}
              <tr>
                <td class="cat-cell">{b.category}</td>
                <td class="month-cell">{b.month}</td>
                <td class="limit-cell">
                  {#if key in editMap}
                    <input
                      class="inline-edit"
                      type="number"
                      min="0"
                      step="0.01"
                      bind:value={editMap[key]}
                      on:keydown={(e) => { if (e.key === 'Enter') saveEdit(b); if (e.key === 'Escape') { delete editMap[key]; editMap = editMap; } }}
                    />
                    <button class="btn-confirm" on:click={() => saveEdit(b)} disabled={saving[key]}>✓</button>
                  {:else}
                    <button class="limit-btn" on:click={() => startEdit(b)}>
                      €{(b.limit_cents / 100).toFixed(2)}
                    </button>
                  {/if}
                </td>
                <td>
                  <button class="btn-delete" on:click={() => handleDelete(b)} title="Remove">
                    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                      <polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14H6L5 6"/><path d="M10 11v6M14 11v6"/>
                    </svg>
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}

    <!-- Add new budget row -->
    <div class="add-card">
      <h4 class="add-title">Add / Update Limit</h4>
      {#if addError}
        <div class="error-banner">{addError}</div>
      {/if}
      <div class="add-form">
        <div class="field">
          <label for="bcat">Category</label>
          <select id="bcat" bind:value={newForm.category}>
            <option value="">— select —</option>
            {#each $splits as s}
              <option value={s.category}>{s.category}</option>
            {/each}
          </select>
        </div>
        <div class="field field--sm">
          <label for="bmonth">Month</label>
          <input id="bmonth" type="text" bind:value={newForm.month} placeholder="ALL or YYYY-MM" />
        </div>
        <div class="field field--sm">
          <label for="blimit">Limit (€)</label>
          <input id="blimit" type="number" min="0" step="0.01" bind:value={newForm.limit_euros} placeholder="0.00" />
        </div>
        <button class="btn-save" on:click={handleAdd} disabled={addSaving}>
          {addSaving ? '…' : 'Save'}
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .budget-manager { max-width: 860px; margin: 0 auto; padding-bottom: 2rem; }
  .section-title { font-size: 1.4rem; font-weight: 700; color: #e2e8f0; margin-bottom: 0.25rem; }
  .section-sub { font-size: 0.82rem; color: #64748b; margin-bottom: 1.5rem; }
  code { background: #1e293b; border-radius: 4px; padding: 0.1rem 0.4rem; font-size: 0.8rem; color: #7dd3fc; }
  .sub-head { font-size: 0.95rem; font-weight: 600; color: #94a3b8; margin: 0 0 0.8rem; }
  .status-section { margin-bottom: 2rem; }
  .config-section {}
  .empty-state {
    text-align: center; color: #475569; padding: 1.5rem;
    border: 1px dashed #334155; border-radius: 10px; margin-bottom: 1rem; font-size: 0.85rem;
  }
  .error-banner {
    background: rgba(239,68,68,0.1); border: 1px solid rgba(239,68,68,0.3);
    color: #fca5a5; border-radius: 8px; padding: 0.6rem 0.9rem;
    font-size: 0.82rem; margin-bottom: 1rem;
  }
  /* Status grid */
  .status-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 0.75rem; }
  .status-card {
    background: #0f172a; border: 1px solid #1e293b; border-radius: 12px;
    padding: 0.9rem 1rem;
  }
  .card-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem; }
  .cat-name { font-size: 0.78rem; font-weight: 600; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; }
  .amounts { font-size: 0.82rem; color: #e2e8f0; font-variant-numeric: tabular-nums; }
  .no-limit { color: #475569; font-size: 0.75rem; }
  .progress-track { background: #1e293b; border-radius: 999px; height: 6px; overflow: hidden; }
  .progress-bar { height: 100%; border-radius: 999px; transition: width 0.5s ease; }
  .progress-bar.green { background: linear-gradient(90deg,#22c55e,#4ade80); }
  .progress-bar.yellow { background: linear-gradient(90deg,#eab308,#facc15); }
  .progress-bar.red { background: linear-gradient(90deg,#ef4444,#f87171); }
  .pct-label { font-size: 0.72rem; font-weight: 700; margin-top: 0.3rem; text-align: right; }
  .pct-label.green { color: #4ade80; }
  .pct-label.yellow { color: #facc15; }
  .pct-label.red { color: #f87171; }
  /* Table */
  .table-wrap { overflow-x: auto; border-radius: 12px; border: 1px solid #1e293b; margin-bottom: 1rem; }
  .budget-table { width: 100%; border-collapse: collapse; font-size: 0.875rem; }
  .budget-table thead tr { background: #0f172a; }
  .budget-table th { text-align: left; padding: 0.6rem 1rem; color: #475569; font-weight: 600; font-size: 0.73rem; text-transform: uppercase; letter-spacing: 0.06em; }
  .budget-table tbody tr { border-top: 1px solid #1e293b; transition: background 0.15s; }
  .budget-table tbody tr:hover { background: rgba(99,102,241,0.04); }
  .budget-table td { padding: 0.55rem 1rem; color: #cbd5e1; }
  .cat-cell { font-weight: 500; color: #e2e8f0; }
  .month-cell { color: #64748b; font-size: 0.8rem; }
  .limit-cell { font-variant-numeric: tabular-nums; }
  .limit-btn {
    background: transparent; border: 1px solid #334155; color: #38bdf8; border-radius: 6px;
    padding: 0.2rem 0.55rem; font-size: 0.82rem; cursor: pointer; transition: border-color 0.15s;
  }
  .limit-btn:hover { border-color: #6366f1; }
  .inline-edit {
    background: #1e293b; border: 1px solid #6366f1; border-radius: 6px; color: #e2e8f0;
    padding: 0.2rem 0.5rem; font-size: 0.82rem; width: 90px; font-family: inherit;
  }
  .btn-confirm {
    background: #22c55e; border: none; color: #fff; border-radius: 5px;
    padding: 0.2rem 0.4rem; font-size: 0.8rem; cursor: pointer; margin-left: 0.3rem;
  }
  .btn-delete {
    background: transparent; border: 1px solid #ef4444; color: #ef4444;
    border-radius: 6px; padding: 0.28rem 0.38rem; cursor: pointer; display: flex;
    align-items: center; opacity: 0.7; transition: opacity 0.15s;
  }
  .btn-delete:hover { opacity: 1; }
  /* Add form */
  .add-card { background: #0f172a; border: 1px solid #1e293b; border-radius: 14px; padding: 1.2rem; }
  .add-title { font-size: 0.9rem; font-weight: 600; color: #e2e8f0; margin-bottom: 0.8rem; }
  .add-form { display: flex; gap: 0.75rem; flex-wrap: wrap; align-items: flex-end; }
  .field { display: flex; flex-direction: column; gap: 0.3rem; flex: 1; min-width: 130px; }
  .field--sm { max-width: 150px; }
  label { font-size: 0.73rem; font-weight: 600; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em; }
  input, select {
    background: #1e293b; border: 1px solid #334155; border-radius: 8px;
    color: #e2e8f0; padding: 0.48rem 0.7rem; font-size: 0.875rem; font-family: inherit;
    transition: border-color 0.15s;
  }
  input:focus, select:focus { outline: none; border-color: #6366f1; box-shadow: 0 0 0 3px rgba(99,102,241,0.15); }
  .btn-save {
    background: linear-gradient(135deg, #6366f1, #8b5cf6); color: #fff; border: none;
    border-radius: 9px; padding: 0.52rem 1.2rem; font-size: 0.875rem; font-weight: 600;
    cursor: pointer; transition: opacity 0.15s; align-self: flex-end;
  }
  .btn-save:hover:not(:disabled) { opacity: 0.9; }
  .btn-save:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
