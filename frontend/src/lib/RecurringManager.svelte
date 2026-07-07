<script>
  import { recurringExpenses, splits, users } from './stores.js';
  import { createRecurring, deleteRecurring } from './api.js';

  $: activeUsers = $users.filter((u) => u.is_active);

  let form = { name: '', cost_euros: '', who_paid: '', category: '', day_of_month: 1 };
  // Default who_paid to first active user once users load
  $: if (!form.who_paid && activeUsers.length > 0) form.who_paid = activeUsers[0].name;
  let saving = false;
  let error = '';

  async function handleSubmit() {
    error = '';
    const cost_cents = Math.round(parseFloat(form.cost_euros) * 100);
    if (!form.name || isNaN(cost_cents) || cost_cents <= 0 || !form.category) {
      error = 'Fill in all fields with valid values.';
      return;
    }
    saving = true;
    try {
      await createRecurring({
        name: form.name,
        cost_cents,
        who_paid: form.who_paid,
        category: form.category,
        day_of_month: Number(form.day_of_month),
      });
      form = { name: '', cost_euros: '', who_paid: activeUsers[0]?.name ?? '', category: '', day_of_month: 1 };
    } catch (e) {
      error = e.message;
    } finally {
      saving = false;
    }
  }

  async function handleDelete(id) {
    error = '';
    try {
      await deleteRecurring(id);
    } catch (e) {
      error = e.message;
    }
  }

  function ordinal(n) {
    const s = ['th','st','nd','rd'];
    const v = n % 100;
    return n + (s[(v - 20) % 10] || s[v] || s[0]);
  }
</script>

<div class="recurring-manager">
  <h2 class="section-title">Recurring Expenses</h2>
  <p class="section-sub">Templates are automatically inserted as expenses each month on their scheduled day.</p>

  <!-- ── Existing templates ─────────────────────────────── -->
  {#if $recurringExpenses.length === 0}
    <div class="empty-state">No recurring expenses configured yet.</div>
  {:else}
    <div class="table-wrap">
      <table class="rec-table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Amount</th>
            <th>Paid by</th>
            <th>Category</th>
            <th>Day</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each $recurringExpenses as rec (rec.id)}
            <tr>
              <td class="name-cell">{rec.name}</td>
              <td class="amount-cell">€{(rec.cost_cents / 100).toFixed(2)}</td>
              <td>
                <span class="payer-badge" style="background-color:{($users.find(u=>u.name===rec.who_paid)?.color ?? '#6366f1') + '33'}; color:{$users.find(u=>u.name===rec.who_paid)?.color ?? '#a5b4fc'}">
                  {rec.who_paid}
                </span>
              </td>
              <td class="cat-cell">{rec.category}</td>
              <td class="day-cell">{ordinal(rec.day_of_month)}</td>
              <td>
                <button class="btn-delete" on:click={() => handleDelete(rec.id)} title="Remove">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
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

  <!-- ── Add new template ───────────────────────────────── -->
  <div class="add-card">
    <h3 class="add-title">Add Recurring Expense</h3>
    {#if error}
      <div class="error-banner">{error}</div>
    {/if}
    <form class="rec-form" on:submit|preventDefault={handleSubmit}>
      <div class="field-row">
        <div class="field">
          <label for="rec-name">Name</label>
          <input id="rec-name" type="text" bind:value={form.name} placeholder="e.g. Spotify" maxlength="96" />
        </div>
        <div class="field field--sm">
          <label for="rec-amount">Amount (€)</label>
          <input id="rec-amount" type="number" min="0.01" step="0.01" bind:value={form.cost_euros} placeholder="0.00" />
        </div>
      </div>
      <div class="field-row">
        <div class="field">
          <label for="rec-payer">Paid by</label>
          <select id="rec-payer" bind:value={form.who_paid}>
            {#each activeUsers as u (u.name)}
              <option value={u.name}>{u.name}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="rec-cat">Category</label>
          <select id="rec-cat" bind:value={form.category}>
            <option value="">— select —</option>
            {#each $splits as s}
              <option value={s.category}>{s.category}</option>
            {/each}
          </select>
        </div>
        <div class="field field--xs">
          <label for="rec-day">Day of month</label>
          <input id="rec-day" type="number" min="1" max="31" bind:value={form.day_of_month} />
        </div>
      </div>
      <button class="btn-save" type="submit" disabled={saving}>
        {saving ? 'Saving…' : '+ Add Recurring'}
      </button>
    </form>
  </div>
</div>

<style>
  .recurring-manager {
    max-width: 860px;
    margin: 0 auto;
    padding: 0 0 2rem;
  }
  .section-title {
    font-size: 1.4rem;
    font-weight: 700;
    color: #e2e8f0;
    margin-bottom: 0.25rem;
  }
  .section-sub {
    font-size: 0.82rem;
    color: #64748b;
    margin-bottom: 1.5rem;
  }
  .empty-state {
    text-align: center;
    color: #475569;
    padding: 2rem;
    border: 1px dashed #334155;
    border-radius: 10px;
    margin-bottom: 1.5rem;
  }
  .table-wrap {
    overflow-x: auto;
    border-radius: 12px;
    border: 1px solid #1e293b;
    margin-bottom: 1.5rem;
  }
  .rec-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
  }
  .rec-table thead tr {
    background: #0f172a;
  }
  .rec-table th {
    text-align: left;
    padding: 0.65rem 1rem;
    color: #475569;
    font-weight: 600;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .rec-table tbody tr {
    border-top: 1px solid #1e293b;
    transition: background 0.15s;
  }
  .rec-table tbody tr:hover {
    background: rgba(99,102,241,0.05);
  }
  .rec-table td {
    padding: 0.65rem 1rem;
    color: #cbd5e1;
  }
  .name-cell { font-weight: 500; color: #e2e8f0; }
  .amount-cell { font-variant-numeric: tabular-nums; color: #38bdf8; font-weight: 600; }
  .cat-cell { font-size: 0.78rem; color: #94a3b8; }
  .day-cell { color: #94a3b8; }
  .payer-badge {
    display: inline-block;
    padding: 0.15rem 0.55rem;
    border-radius: 999px;
    font-size: 0.72rem;
    font-weight: 600;
  }
  .btn-delete {
    background: transparent;
    border: 1px solid #ef4444;
    color: #ef4444;
    border-radius: 6px;
    padding: 0.3rem 0.4rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    opacity: 0.7;
    transition: opacity 0.15s;
  }
  .btn-delete:hover { opacity: 1; }
  .add-card {
    background: #0f172a;
    border: 1px solid #1e293b;
    border-radius: 14px;
    padding: 1.5rem;
  }
  .add-title {
    font-size: 1rem;
    font-weight: 600;
    color: #e2e8f0;
    margin-bottom: 1rem;
  }
  .error-banner {
    background: rgba(239,68,68,0.1);
    border: 1px solid rgba(239,68,68,0.3);
    color: #fca5a5;
    border-radius: 8px;
    padding: 0.6rem 0.9rem;
    font-size: 0.82rem;
    margin-bottom: 1rem;
  }
  .rec-form { display: flex; flex-direction: column; gap: 1rem; }
  .field-row { display: flex; gap: 0.75rem; flex-wrap: wrap; }
  .field { display: flex; flex-direction: column; gap: 0.3rem; flex: 1; min-width: 140px; }
  .field--sm { max-width: 140px; }
  .field--xs { max-width: 100px; }
  label { font-size: 0.75rem; font-weight: 600; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em; }
  input, select {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 8px;
    color: #e2e8f0;
    padding: 0.5rem 0.75rem;
    font-size: 0.875rem;
    transition: border-color 0.15s;
    font-family: inherit;
  }
  input:focus, select:focus {
    outline: none;
    border-color: #6366f1;
    box-shadow: 0 0 0 3px rgba(99,102,241,0.15);
  }
  .btn-save {
    align-self: flex-start;
    background: linear-gradient(135deg, #6366f1, #8b5cf6);
    color: #fff;
    border: none;
    border-radius: 9px;
    padding: 0.6rem 1.4rem;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s, transform 0.1s;
  }
  .btn-save:hover:not(:disabled) { opacity: 0.9; transform: translateY(-1px); }
  .btn-save:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
