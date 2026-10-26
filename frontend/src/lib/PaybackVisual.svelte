<script>
  /**
   * PaybackVisual.svelte — the Auditor's settlement for the selected month: debts,
   * per-bucket paid / share / net. A sealed month shows its stored result and
   * flags drift when today's data would settle differently.
   */

  import { audit } from './api.js';
  import { money, fmtPct } from './format.js';
  import { selectedMonth, scopedMembers, ledgerVersion, memberById, categoryName, paybackDisplayMode, openDrilldown } from './stores.js';

  let settlement = null;
  let sealed = null;
  let settling = false;
  let settleError = '';

  $: load($selectedMonth, $scopedMembers ?? [], $ledgerVersion);

  async function load(month, members) {
    settleError = '';
    try {
      const view = await audit.settlement(month, members);
      sealed = view.state === 'sealed' ? view : null;
      settlement = sealed ? JSON.parse(view.seal.result_json) : view.value;
    } catch (e) {
      settleError = e.message;
    }
  }

  $: debts = settlement?.debts ?? [];
  $: rows = settlement?.rows ?? [];
  $: allSettled = debts.length === 0;

  const name = (id) => $memberById.get(Number(id))?.name ?? `#${id}`;
  const color = (id) => $memberById.get(Number(id))?.color ?? '#6366f1';
  const initial = (id) => name(id).charAt(0).toUpperCase();
  const bucketLabel = (b) => (b.personal_of == null ? $categoryName(b.category) : `${$categoryName(b.category)} · ${name(b.personal_of)}`);
  const members = (row) => Object.keys(row.net);

  async function lockMonth() {
    settleError = '';
    settling = true;
    try {
      await audit.seal($selectedMonth);
      await load($selectedMonth, $scopedMembers ?? []);
    } catch (e) {
      settleError = e.message;
    } finally {
      settling = false;
    }
  }
</script>

<div class="space-y-6">

  <!-- ── Month Settled Banner ─────────────────────────────────────────────── -->
  {#if sealed}
    <div class="card bg-gradient-to-br from-emerald-50 via-white to-white dark:from-emerald-950/50 dark:via-neutral-900 dark:to-neutral-900 border-emerald-300 dark:border-emerald-700/50 p-5 flex items-center gap-4">
      <div class="w-10 h-10 rounded-full bg-emerald-100 dark:bg-emerald-900 border border-emerald-300 dark:border-emerald-500/40 flex items-center justify-center text-emerald-600 dark:text-emerald-400 text-xl flex-shrink-0">✓</div>
      <div>
        <p class="text-sm font-bold text-emerald-800 dark:text-emerald-300">✔️ Month Settled</p>
        <p class="text-xs text-emerald-600 dark:text-emerald-500 mt-0.5">
          {$selectedMonth} was sealed on {sealed.seal.sealed_at.slice(0, 10)}
          &middot; Transfer: {$money(settlement.transfer_total)}
        </p>
        {#if sealed.drift}
          <p class="text-xs font-semibold text-amber-700 dark:text-amber-400 mt-1">⚠ Current data would settle differently; the sealed result stands.</p>
        {/if}
      </div>
    </div>
  {/if}

  <!-- ── Hero — Debt Transfers ─────────────────────────────────────────────── -->
  {#if allSettled}
    <div class="card bg-gradient-to-br from-emerald-50 via-white to-white dark:from-emerald-950/40 dark:via-neutral-900 dark:to-neutral-900 border-emerald-200 dark:border-emerald-900/60 p-6 flex flex-col items-center text-center shadow-sm">
      <div class="w-12 h-12 rounded-full bg-emerald-100 dark:bg-emerald-950 border border-emerald-300 dark:border-emerald-500/30 flex items-center justify-center text-emerald-600 dark:text-emerald-400 text-xl font-bold mb-3 shadow-inner">✓</div>
      <h3 class="text-base font-semibold text-neutral-900 dark:text-white">All Settled Up!</h3>
      <p class="text-neutral-500 dark:text-neutral-400 text-xs mt-1 max-w-sm">
        All shared expenses for this month are split exactly according to your agreed configurations.
      </p>
    </div>
  {:else}
    <div class="card p-5 sm:p-6">
      <div class="flex items-center justify-between mb-4">
        <div>
          <h3 class="text-sm font-semibold text-neutral-800 dark:text-neutral-200 uppercase tracking-wider">Settlement Transfers</h3>
          <p class="text-xs text-neutral-500 mt-0.5">Who pays whom to reconcile the month</p>
        </div>
        <span class="badge-amber">Action Required</span>
      </div>

      <!-- One row per debt item -->
      <div class="space-y-3">
        {#each debts as debt (`${debt.from}-${debt.to}`)}
          <div class="card-sub flex items-center gap-4 px-4 sm:px-5 py-3.5 sm:py-4">
            <!-- From user -->
            <div class="flex flex-col items-center min-w-[48px]">
              <div
                class="w-10 h-10 rounded-full flex items-center justify-center text-xs font-bold shadow-md flex-none text-white"
                style="background-color:{color(debt.from)}; box-shadow: 0 4px 12px {color(debt.from)}40"
              >{initial(debt.from)}</div>
              <span class="text-[10px] text-neutral-500 dark:text-neutral-400 mt-1 font-medium">{name(debt.from)}</span>
            </div>

            <!-- Arrow + amount -->
            <div class="flex-1 flex flex-col items-center">
              <span class="font-bold tabular-nums text-lg sm:text-xl" style="color: {color(debt.to)}">{$money(debt.amount)}</span>
              <div class="flex items-center text-neutral-400 dark:text-neutral-600 font-bold text-lg select-none leading-none mt-0.5">
                ─────&gt;
              </div>
            </div>

            <!-- To user -->
            <div class="flex flex-col items-center min-w-[48px]">
              <div
                class="w-10 h-10 rounded-full flex items-center justify-center text-xs font-bold shadow-md flex-none text-white"
                style="background-color:{color(debt.to)}; box-shadow: 0 4px 12px {color(debt.to)}40"
              >{initial(debt.to)}</div>
              <span class="text-[10px] text-neutral-500 dark:text-neutral-400 mt-1 font-medium">{name(debt.to)}</span>
            </div>
          </div>
        {/each}
      </div>

      <p class="text-neutral-500 text-xs mt-3">
        Computed and verified by the Auditor ({settlement?.components?.length ?? 0} independent group{settlement?.components?.length === 1 ? '' : 's'}).
      </p>

      <!-- Lock Month button -->
      {#if !sealed}
        <div class="pt-4 border-t border-neutral-200 dark:border-neutral-800/60 mt-4">
          <button
            class="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-xl text-xs font-semibold
                   bg-amber-50 dark:bg-amber-900/40 border border-amber-300 dark:border-amber-700/50 text-amber-800 dark:text-amber-300
                   hover:bg-amber-100 dark:hover:bg-amber-800/50 transition-colors disabled:opacity-50 cursor-pointer shadow-sm"
            on:click={lockMonth}
            disabled={settling}
          >
            <svg class="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 20 20">
              <path fill-rule="evenodd" d="M10 1a4.5 4.5 0 00-4.5 4.5V9H5a2 2 0 00-2 2v6a2 2 0 002 2h10a2 2 0 002-2v-6a2 2 0 00-2-2h-.5V5.5A4.5 4.5 0 0010 1zm3 8V5.5a3 3 0 10-6 0V9h6z" clip-rule="evenodd"/>
            </svg>
            {settling ? 'Locking…' : 'Mark as Settled & Lock Month'}
          </button>
          {#if settleError}<p class="text-xs text-rose-600 dark:text-red-400 mt-1">{settleError}</p>{/if}
        </div>
      {/if}
    </div>
  {/if}

  <!-- ── Category Payback Breakdown ───────────────────────────────────────── -->
  <div class="card p-5 sm:p-6">
    <div class="flex items-center justify-between mb-5">
      <div>
        <h4 class="text-sm font-semibold text-neutral-800 dark:text-neutral-200">Category Adjustments</h4>
        <p class="text-xs text-neutral-500 mt-0.5">Per-category actual spend vs. agreed shares &middot; Click any category to view underlying expenses</p>
      </div>
    </div>

    {#if rows.length === 0}
      <div class="empty-state-box">
        <div class="w-12 h-12 rounded-2xl bg-neutral-100 dark:bg-neutral-800/80 flex items-center justify-center mb-2">
          <svg class="w-6 h-6 text-neutral-400 dark:text-neutral-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
            <path stroke-linecap="round" stroke-linejoin="round"
              d="M12 3v2.25m6.364.386l-1.591 1.591M21 12h-2.25m-.386 6.364l-1.591-1.591M12 18.75V21m-4.773-4.227l-1.591 1.591M5.25 12H3m4.227-4.773L5.636 5.636M15.75 12a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0z" />
          </svg>
        </div>
        <p class="text-neutral-700 dark:text-neutral-300 text-sm font-semibold">No transactions yet.</p>
        <p class="text-neutral-500 text-xs max-w-xs mt-1">
          Log expenses on the Expenses tab — per-category adjustments will appear here once there's spending to compare.
        </p>
      </div>
    {:else}
      {#if $paybackDisplayMode === 'bar'}
        <div class="space-y-5">
          {#each rows as row}
            {@const label = bucketLabel(row.bucket)}
            <div
              class="border-b border-neutral-200 dark:border-neutral-800/60 pb-4 last:border-0 last:pb-0 p-2.5 -mx-2.5 rounded-xl hover:bg-neutral-50 dark:hover:bg-neutral-900/60 transition-colors cursor-pointer group"
              role="button" tabindex="0"
              on:click={() => openDrilldown('category-adjustment', label, { id: row.bucket.category, row })}
              on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); openDrilldown('category-adjustment', label, { id: row.bucket.category, row }); } }}
              aria-label="View overview of expenses for {label}"
            >
              <div class="flex items-baseline justify-between mb-2">
                <span class="text-sm font-bold text-neutral-900 dark:text-neutral-100 group-hover:text-indigo-600 dark:group-hover:text-indigo-400 transition-colors">{label}</span>
                <span class="text-xs text-neutral-500 tabular-nums">{$money(row.total)}</span>
              </div>
              <div class="flex h-6 rounded-lg overflow-hidden bg-neutral-100 dark:bg-neutral-800/40">
                {#each Object.entries(row.paid_pct) as [m, pct]}
                  {#if pct > 0}
                    <div
                      class="h-full flex items-center justify-center text-[10px] font-bold tabular-nums text-white/90"
                      style="width: {pct}%; background-color: {color(m)};"
                      title="{name(m)}: {$money(row.paid[m])}"
                    >
                      {#if pct > 15}{$money(row.paid[m])}{/if}
                    </div>
                  {/if}
                {/each}
              </div>
              <div class="flex flex-wrap gap-x-4 gap-y-1 mt-2">
                {#each members(row) as m}
                  {@const net = row.net[m]}
                  <span class="text-[11px] tabular-nums">
                    <span class="font-semibold" style="color: {color(m)}">{name(m)}</span>
                    <span class="text-neutral-500 ml-1">{$money(row.paid[m] ?? 0)}</span>
                    <span class="ml-1 font-semibold {net > 0 ? 'text-emerald-600 dark:text-emerald-400' : net < 0 ? 'text-rose-600 dark:text-red-400' : 'text-neutral-400 dark:text-neutral-600'}">
                      {net > 0 ? `+${$money(net)}` : net < 0 ? $money(net) : '='}
                    </span>
                  </span>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <div class="space-y-6">
          {#each rows as row}
            {@const label = bucketLabel(row.bucket)}
            <div
              class="border-b border-neutral-200 dark:border-neutral-800/60 pb-5 last:border-0 last:pb-0 p-3 -mx-3 rounded-2xl hover:bg-neutral-50 dark:hover:bg-neutral-900/60 transition-colors cursor-pointer group"
              role="button" tabindex="0"
              on:click={() => openDrilldown('category-adjustment', label, { id: row.bucket.category, row })}
              on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); openDrilldown('category-adjustment', label, { id: row.bucket.category, row }); } }}
              aria-label="View overview of expenses for {label}"
            >
              <div class="flex items-center gap-2 mb-3">
                <span class="text-sm font-bold text-neutral-900 dark:text-neutral-100 group-hover:text-indigo-600 dark:group-hover:text-indigo-400 transition-colors">{label}</span>
                <span class="text-xs text-neutral-500 ml-1">Total: {$money(row.total)}</span>
              </div>
              <div class="grid gap-2" style="grid-template-columns: repeat(auto-fill, minmax(140px, 1fr))">
                {#each members(row) as m}
                  {@const net = row.net[m]}
                  <div class="card-sub p-3">
                    <div class="flex items-center gap-2 mb-1.5">
                      <div class="w-5 h-5 rounded-full flex items-center justify-center text-[9px] font-bold text-white" style="background-color:{color(m)}">{initial(m)}</div>
                      <span class="text-xs font-semibold text-neutral-800 dark:text-neutral-200">{name(m)}</span>
                    </div>
                    <p class="text-xs text-neutral-500">Paid: <span class="text-neutral-900 dark:text-neutral-200 font-semibold tabular-nums">{$money(row.paid[m] ?? 0)}</span></p>
                    <p class="text-xs text-neutral-500">Share: <span class="font-semibold tabular-nums" style="color:{color(m)}">{fmtPct(row.share_pct[m] ?? 0)}</span></p>
                    <p class="text-[11px] mt-1 font-semibold tabular-nums {net > 0 ? 'text-emerald-600 dark:text-emerald-400' : net < 0 ? 'text-rose-600 dark:text-red-400' : 'text-neutral-500'}">
                      {net > 0 ? `+${$money(net)} owed back` : net < 0 ? `${$money(net)} owes` : 'settled'}
                    </p>
                  </div>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </div>

</div>
