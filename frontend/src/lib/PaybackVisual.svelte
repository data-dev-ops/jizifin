<script>
  /**
   * PaybackVisual.svelte
   *
   * Displays payback (settlement) calculations for any number of household members.
   * Renders a hero section listing all debt transfers (from_user → to_user, amount),
   * and a category-level breakdown table showing per-user paid amounts and net balances.
   */

  import { paybacks, settlements, selectedMonth, users } from './stores.js';
  import { createSettlement, fetchSettlements } from './api.js';

  let settling = false;
  let settleError = '';

  $: currentMonth      = $selectedMonth;
  $: isSettled         = $settlements.some((s) => s.month === currentMonth);
  $: settlementRecord  = $settlements.find((s) => s.month === currentMonth);
  $: debts             = $paybacks.debts ?? [];
  $: allSettled        = debts.length === 0;

  function userColor(name) {
    return $users.find((u) => u.name === name)?.color ?? '#6366f1';
  }

  function initial(name) {
    return (name ?? '').charAt(0).toUpperCase();
  }

  function fmt(n) {
    return `€${Number(n).toFixed(2)}`;
  }

  /** Total net transfer amount across all debt items. */
  $: totalTransfer = debts.reduce((s, d) => s + d.amount, 0);

  async function lockMonth() {
    settleError = '';
    settling = true;
    try {
      const netCents = Math.round(totalTransfer * 100);
      await createSettlement({ month: currentMonth, net_balance_transferred_cents: netCents });
      await fetchSettlements();
    } catch (e) {
      settleError = e.message;
    } finally {
      settling = false;
    }
  }
</script>

<div class="space-y-6">

  <!-- ── Month Settled Banner ─────────────────────────────────────────────── -->
  {#if isSettled}
    <div class="bg-gradient-to-br from-emerald-950/50 via-neutral-900 to-neutral-900 border border-emerald-700/50 rounded-2xl p-5 flex items-center gap-4">
      <div class="w-10 h-10 rounded-full bg-emerald-900 border border-emerald-500/40 flex items-center justify-center text-emerald-400 text-xl flex-shrink-0">✓</div>
      <div>
        <p class="text-sm font-bold text-emerald-300">✔️ Month Settled</p>
        <p class="text-xs text-emerald-600 mt-0.5">
          {currentMonth} was locked on {settlementRecord?.settled_at?.slice(0, 10) ?? ''}
          &middot; Transfer: {fmt((settlementRecord?.net_balance_transferred_cents ?? 0) / 100)}
        </p>
      </div>
    </div>
  {/if}

  <!-- ── Hero — Debt Transfers ─────────────────────────────────────────────── -->
  {#if allSettled}
    <div class="bg-gradient-to-br from-emerald-950/40 via-neutral-900 to-neutral-900 border border-emerald-900/60 rounded-2xl p-6 flex flex-col items-center text-center shadow-lg shadow-emerald-950/10">
      <div class="w-12 h-12 rounded-full bg-emerald-950 border border-emerald-500/30 flex items-center justify-center text-emerald-400 text-xl font-bold mb-3 shadow-inner">✓</div>
      <h3 class="text-base font-semibold text-white">All Settled Up!</h3>
      <p class="text-neutral-400 text-xs mt-1 max-w-sm">
        All shared expenses for this month are split exactly according to your agreed configurations.
      </p>
    </div>
  {:else}
    <div class="bg-neutral-900 border border-neutral-800 rounded-2xl p-6 shadow-xl">
      <h3 class="text-sm font-semibold text-neutral-400 uppercase tracking-wider mb-4">Settlement Transfers</h3>

      <!-- One row per debt item -->
      <div class="space-y-3">
        {#each debts as debt (debt.from_user + debt.to_user)}
          <div class="flex items-center gap-4 bg-neutral-950/40 border border-neutral-800/80 rounded-xl px-5 py-4">
            <!-- From user -->
            <div class="flex flex-col items-center min-w-[48px]">
              <div
                class="w-10 h-10 rounded-full flex items-center justify-center text-xs font-bold shadow-md flex-none"
                style="background-color:{userColor(debt.from_user)}; box-shadow: 0 4px 12px {userColor(debt.from_user)}40"
              >{initial(debt.from_user)}</div>
              <span class="text-[10px] text-neutral-400 mt-1 font-medium">{debt.from_user}</span>
            </div>

            <!-- Arrow + amount -->
            <div class="flex-1 flex flex-col items-center">
              <span class="font-bold tabular-nums text-lg" style="color: {userColor(debt.to_user)}">{fmt(debt.amount)}</span>
              <div class="flex items-center text-neutral-600 font-bold text-lg select-none leading-none mt-0.5">
                ─────&gt;
              </div>
            </div>

            <!-- To user -->
            <div class="flex flex-col items-center min-w-[48px]">
              <div
                class="w-10 h-10 rounded-full flex items-center justify-center text-xs font-bold shadow-md flex-none"
                style="background-color:{userColor(debt.to_user)}; box-shadow: 0 4px 12px {userColor(debt.to_user)}40"
              >{initial(debt.to_user)}</div>
              <span class="text-[10px] text-neutral-400 mt-1 font-medium">{debt.to_user}</span>
            </div>
          </div>
        {/each}
      </div>

      <p class="text-neutral-500 text-xs mt-3">
        Calculated by comparing actual spending against split percentages for all active categories.
      </p>

      <!-- Lock Month button -->
      {#if !isSettled}
        <div class="pt-4 border-t border-neutral-800/60 mt-4">
          <button
            class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold
                   bg-amber-900/40 border border-amber-700/50 text-amber-300
                   hover:bg-amber-800/50 transition-colors disabled:opacity-50"
            on:click={lockMonth}
            disabled={settling}
          >
            <svg class="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 20 20">
              <path fill-rule="evenodd" d="M10 1a4.5 4.5 0 00-4.5 4.5V9H5a2 2 0 00-2 2v6a2 2 0 002 2h10a2 2 0 002-2v-6a2 2 0 00-2-2h-.5V5.5A4.5 4.5 0 0010 1zm3 8V5.5a3 3 0 10-6 0V9h6z" clip-rule="evenodd"/>
            </svg>
            {settling ? 'Locking…' : 'Mark as Settled & Lock Month'}
          </button>
          {#if settleError}<p class="text-xs text-red-400 mt-1">{settleError}</p>{/if}
        </div>
      {/if}
    </div>
  {/if}

  <!-- ── Category Payback Breakdown ───────────────────────────────────────── -->
  <div class="bg-neutral-900 border border-neutral-800 rounded-2xl p-6">
    <div class="flex items-center justify-between mb-5">
      <div>
        <h4 class="text-sm font-semibold text-neutral-200">Category Adjustments</h4>
        <p class="text-xs text-neutral-500 mt-0.5">Per-category actual spend vs. agreed shares</p>
      </div>
    </div>

    {#if $paybacks.rows.length === 0}
      <div class="flex flex-col items-center justify-center py-10 text-center">
        <p class="text-neutral-500 text-sm">No transactions logged for this month.</p>
      </div>
    {:else}
      <div class="space-y-6">
        {#each $paybacks.rows as row}
          {@const catUsers = Object.keys(row.per_user_paid ?? {})}

          <div class="border-b border-neutral-800/60 pb-5 last:border-0 last:pb-0">
            <!-- Row header -->
            <div class="flex flex-wrap items-center justify-between gap-3 mb-3">
              <div>
                <span class="text-sm font-bold text-neutral-100">{row.category}</span>
                <span class="text-xs text-neutral-500 ml-2">Total: {fmt(row.total_amount)}</span>
              </div>
            </div>

            <!-- Per-user spending grid -->
            <div class="grid gap-2" style="grid-template-columns: repeat(auto-fill, minmax(140px, 1fr))">
              {#each catUsers as userName}
                {@const color = userColor(userName)}
                {@const paid  = row.per_user_paid[userName] ?? 0}
                {@const net   = row.net_per_user[userName]  ?? 0}
                {@const pct   = row.per_user_share_pct[userName] ?? 0}

                <div class="bg-neutral-950/60 border border-neutral-800 rounded-xl p-3">
                  <div class="flex items-center gap-2 mb-1.5">
                    <div class="w-5 h-5 rounded-full flex items-center justify-center text-[9px] font-bold"
                         style="background-color:{color}">{initial(userName)}</div>
                    <span class="text-xs font-semibold text-neutral-200">{userName}</span>
                  </div>
                  <p class="text-xs text-neutral-500">Paid: <span class="text-neutral-200 font-semibold tabular-nums">{fmt(paid)}</span></p>
                  <p class="text-xs text-neutral-500">Share: <span class="font-semibold tabular-nums" style="color:{color}">{pct.toFixed(1)}%</span></p>
                  <p class="text-[11px] mt-1 font-semibold tabular-nums {net > 0.005 ? 'text-emerald-400' : net < -0.005 ? 'text-red-400' : 'text-neutral-500'}">
                    {net > 0.005 ? `+${fmt(net)} owed back` : net < -0.005 ? `${fmt(net)} owes` : 'settled'}
                  </p>
                </div>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>

</div>
