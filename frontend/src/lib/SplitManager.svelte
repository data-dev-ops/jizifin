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

  import { onMount } from 'svelte';
  import { updateSplit, fetchLatestSalaries, createIncome, fetchIncomeByPerson } from './api.js';
  import { splits, selectedMonth, users, mobileSplitsEditable } from './stores.js';

  /** True when viewport width < 768 px (Tailwind's md breakpoint). */
  let isMobile = false;
  onMount(() => {
    const mq = window.matchMedia('(max-width: 767px)');
    isMobile = mq.matches;
    mq.addEventListener('change', (e) => { isMobile = e.matches; });
  });

  /** Editing splits is locked when on mobile AND the user hasn't enabled it in Settings. */
  $: splitsLocked = isMobile && !$mobileSplitsEditable;

  /** Categories where the payer always bears 100% — percentages are irrelevant. */
  const PERSONAL_PAY = new Set(['PERSONAL COST', 'GIFT', 'LEISURE']);

  $: activeUsers    = $users.filter((u) => u.is_active);
  $: variableSplits = $splits.filter((s) => !PERSONAL_PAY.has(s.category));
  $: personalSplits = $splits.filter((s) =>  PERSONAL_PAY.has(s.category));

  // ── Salary inputs ─────────────────────────────────────────────────────────
  /** { [userName]: euroAmount } */
  let salaryValues  = {};
  let salaryLoading = true;
  let salarySaving  = false;
  let salarySuccess = false;
  let salaryError   = null;

  $: {
    // Initialise missing entries for newly visible active users
    for (const u of activeUsers) {
      if (!(u.name in salaryValues)) salaryValues[u.name] = 0;
    }
  }

  $: totalSalary = activeUsers.reduce((sum, u) => sum + (Number(salaryValues[u.name]) || 0), 0);

  /** Compute salary-implied percentage for a given user. */
  function salaryPct(userName) {
    const s = Number(salaryValues[userName]) || 0;
    return totalSalary > 0 ? (s / totalSalary) * 100 : 100 / (activeUsers.length || 1);
  }

  onMount(async () => {
    try {
      const latest = await fetchLatestSalaries();
      const fresh = { ...salaryValues };
      for (const row of latest) {
        fresh[row.who] = row.amount_cents / 100;
      }
      salaryValues = fresh;
    } catch (_) { /* non-fatal */ } finally {
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
    const entry = {};
    for (const alloc of (split.allocations ?? [])) {
      entry[alloc.user_name] = String(alloc.pct);
    }
    // Ensure every active user has an entry (default 0 if missing)
    for (const u of activeUsers) {
      if (!(u.name in entry)) entry[u.name] = '0';
    }
    editValues[split.category] = entry;
  }

  $: {
    for (const s of variableSplits) initEditValues(s);
  }

  /** Sum of the currently-entered percentages for a category. */
  function rowSum(category) {
    const vals = editValues[category] ?? {};
    return parseFloat(
      Object.values(vals).reduce((acc, v) => acc + (parseFloat(v) || 0), 0).toFixed(4)
    );
  }

  /** Reset all percentages for a category to the salary-implied ratio. */
  function resetToSalary(category) {
    if (!editValues[category]) return;
    const fresh = {};
    const n = activeUsers.length;
    activeUsers.forEach((u, i) => {
      const pct = i === n - 1
        ? parseFloat((100 - activeUsers.slice(0, -1).reduce((s, uu) => s + salaryPct(uu.name), 0)).toFixed(4))
        : parseFloat(salaryPct(u.name).toFixed(4));
      fresh[u.name] = String(pct);
    });
    editValues[category] = fresh;
    editValues = { ...editValues };
  }

  async function save(category) {
    rowError[category]   = null;
    rowSuccess[category] = false;
    const vals = editValues[category] ?? {};
    const allocations = activeUsers.map((u) => ({
      user_name: u.name,
      pct:       parseFloat(parseFloat(vals[u.name] ?? '0').toFixed(4)),
    }));
    const total = parseFloat(allocations.reduce((s, a) => s + a.pct, 0).toFixed(4));
    if (Math.abs(total - 100.0) > 0.01) {
      rowError[category] = `Percentages must sum to 100 (currently ${total.toFixed(2)}).`;
      return;
    }
    saving[category] = true;
    try {
      await updateSplit(category, { allocations });
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

  <!-- ── Salary header ─────────────────────────────────────────────────────── -->
  <div class="bg-neutral-800/60 border border-neutral-700/60 rounded-xl p-4">
    <p class="text-xs font-semibold text-neutral-400 uppercase tracking-wider mb-3">Monthly Salaries</p>

    {#if salaryLoading}
      <p class="text-xs text-neutral-500">Loading…</p>
    {:else}
      <div class="flex flex-wrap gap-4 items-end">
        {#each activeUsers as u (u.name)}
          <div class="flex items-center gap-2 min-w-0">
            <div
              class="w-6 h-6 rounded-full flex items-center justify-center text-[10px] font-bold shrink-0"
              style="background-color: {u.color}"
            >{u.name.charAt(0).toUpperCase()}</div>
            <span class="text-sm font-medium whitespace-nowrap" style="color: {u.color}">{u.name}</span>
            <div class="relative">
              <span class="absolute left-2.5 top-1/2 -translate-y-1/2 text-xs text-neutral-500 pointer-events-none">€</span>
              <input
                id="salary-{u.name}"
                type="number" min="0" step="0.01"
                bind:value={salaryValues[u.name]}
                class="w-28 bg-neutral-900 border border-neutral-700 rounded-lg pl-6 pr-3 py-1.5 text-sm
                       font-semibold tabular-nums text-neutral-200
                       focus:outline-none focus:ring-1 transition-colors"
                style="--tw-ring-color: {u.color}"
              />
            </div>
          </div>
        {/each}

        <!-- Salary-implied ratio preview -->
        {#if totalSalary > 0 && activeUsers.length >= 2}
          <div class="flex items-center self-center gap-1 text-[11px] text-neutral-500">
            {#each activeUsers as u, i}
              <span class="font-semibold" style="color: {u.color}">{salaryPct(u.name).toFixed(1)}%</span>
              {#if i < activeUsers.length - 1}<span>/</span>{/if}
            {/each}
            <span class="ml-1">ratio</span>
          </div>
        {/if}

        <!-- Save Income button -->
        <div class="ml-auto flex flex-col items-end gap-1">
          <button
            id="save-income"
            on:click={saveSalaries}
            disabled={salarySaving}
            class="px-4 py-1.5 rounded-lg text-xs font-semibold
                   bg-emerald-700 hover:bg-emerald-600 disabled:opacity-40
                   transition-colors active:scale-95 whitespace-nowrap"
          >
            {salarySaving ? 'Saving…' : 'Save Income'}
          </button>
          {#if salarySuccess}<span class="text-[10px] text-emerald-400">Saved ✓</span>{/if}
          {#if salaryError}<span class="text-[10px] text-red-400">{salaryError}</span>{/if}
        </div>
      </div>
    {/if}
  </div>

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

    {#each variableSplits as split (split.category)}
      {#if editValues[split.category]}
        {@const sum = rowSum(split.category)}
        {@const sumOk = Math.abs(sum - 100.0) <= 0.01}

        <div class="group px-1 py-3 rounded-xl hover:bg-neutral-800/50 transition-colors">
          {#if splitsLocked}
            <!-- ── Read-only mobile row ── -->
            <div class="flex items-center justify-between gap-3 flex-wrap">
              <span class="inline-flex items-center px-2.5 py-1 rounded-lg bg-neutral-800 border border-neutral-700 text-xs text-neutral-300 font-medium">
                {split.category}
              </span>
              <div class="flex items-center gap-2 flex-wrap">
                {#each activeUsers as u}
                  <span
                    class="inline-flex items-center gap-1 px-2.5 py-1 rounded-lg text-xs font-semibold tabular-nums"
                    style="background-color: {u.color}18; color: {u.color}; border: 1px solid {u.color}40"
                  >
                    {u.name.charAt(0)}: {parseFloat(editValues[split.category][u.name] ?? '0').toFixed(1)}%
                  </span>
                {/each}
              </div>
            </div>
          {:else}
            <!-- ── Editable row (desktop always, mobile when unlocked) ── -->
            <div class="grid gap-3 items-center" style="grid-template-columns: minmax(80px,1fr) {activeUsers.map(() => 'minmax(70px,1fr)').join(' ')} auto">

            <!-- Category badge -->
            <div>
              <span class="inline-flex items-center px-2.5 py-1 rounded-lg bg-neutral-800 border border-neutral-700 text-xs text-neutral-300 font-medium truncate max-w-full">
                {split.category}
              </span>
            </div>

            <!-- One input per active user -->
            {#each activeUsers as u}
              <div class="flex items-center gap-1">
                <input
                  id="split-{u.name}-{split.category}"
                  type="number" min="0" max="100" step="0.1"
                  bind:value={editValues[split.category][u.name]}
                  class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-2 py-2 text-sm
                         font-semibold tabular-nums text-neutral-200
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
                  disabled={totalSalary === 0}
                  title={totalSalary === 0 ? 'Enter salaries above to enable reset' : 'Reset to salary ratio'}
                  class="px-2.5 py-1.5 rounded-lg text-xs font-semibold
                         bg-neutral-700 hover:bg-neutral-600 disabled:opacity-30 disabled:cursor-not-allowed
                         transition-colors active:scale-95"
                >Reset</button>
                <button
                  id="save-split-{split.category}"
                  on:click={() => save(split.category)}
                  disabled={saving[split.category] || !sumOk}
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
            {#if !sumOk && !splitsLocked}
              <p class="text-[10px] text-amber-400 mt-0.5 text-right">sum = {sum.toFixed(2)}%</p>
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
        {#each personalSplits as split (split.category)}
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
