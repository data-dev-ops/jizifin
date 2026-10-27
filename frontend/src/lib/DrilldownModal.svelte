<script>
  /**
   * DrilldownModal.svelte — detail dialog for a dashboard card. Totals and
   * breakdowns come from `/views/month` focused on the clicked item; the list is
   * one server page of matching entries, searchable as plaintext.
   */

  import { audit, entryApi, views } from './api.js';
  import { money, fmtDate, fmtPct, monthBounds } from './format.js';
  import {
    drilldownTarget, closeDrilldown, selectedMonth, scopedMembers, memberById, categoryName,
    tagById, projectById, jobs, privacyShield, ledgerVersion,
  } from './stores.js';

  /** Which `/views/month` focus and breakdown each drilldown type uses. */
  const FOCUS = {
    category: ['category', 'by_member'],
    budget: ['category', 'by_member'],
    'category-adjustment': ['category', 'by_member'],
    payer: ['member', 'by_category'],
    project: ['project', 'by_member'],
    'joint-spent': ['account', 'by_category'],
    'monthly-total': [null, 'by_category'],
  };

  let searchQuery = '';
  let summary = null;
  let entries = [];
  let income = [];
  let progress = null;
  let loadError = null;

  $: target = $drilldownTarget;
  $: isOpen = !!target;
  $: type = target?.type ?? '';
  $: title = target?.title ?? '';
  $: data = target?.data ?? {};
  $: [focusKey, breakdownKey] = FOCUS[type] ?? [null, null];
  $: if (isOpen) load(type, data.id, $selectedMonth, $scopedMembers ?? [], $ledgerVersion);
  $: if (!isOpen) searchQuery = '';

  async function load(kind, id, month, members) {
    loadError = null;
    try {
      if (kind === 'income') {
        income = Object.values((await audit.income(month)).value).filter((i) => !members.length || members.includes(i.member));
        return;
      }
      const focus = focusKey ? { [focusKey]: id } : {};
      const [view, page] = await Promise.all([
        views.month(month, { members, ...focus }),
        entryApi.page({ kind: 'expense', ...monthBounds(month), ...focus }),
      ]);
      summary = view;
      entries = page.entries;
      progress = kind === 'project' ? (await views.projects()).find((p) => p.project === id) : null;
    } catch (err) {
      loadError = err.message;
    }
  }

  $: breakdown = (summary?.[breakdownKey] ?? []).map((b) => ({
    ...b,
    label: breakdownKey === 'by_member' ? $memberById.get(b.key)?.name ?? `#${b.key}` : $categoryName(b.key),
    color: breakdownKey === 'by_member' ? $memberById.get(b.key)?.color ?? '#6366f1' : '#6366f1',
  }));

  $: needle = searchQuery.trim().toLowerCase();
  $: filteredExpenses = needle ? entries.filter(matches) : entries;

  function matches(e) {
    return [e.name, $memberById.get(e.member)?.name, $categoryName(e.category), e.date].some((v) => (v || '').toLowerCase().includes(needle));
  }

  const memberColor = (id) => $memberById.get(Number(id))?.color ?? '#6366f1';
  const memberLabel = (id) => $memberById.get(Number(id))?.name ?? `#${id}`;
  const initial = (text) => (text ?? '?').charAt(0).toUpperCase();

  $: scopedLabel = ($scopedMembers ?? []).map(memberLabel).join(', ');
  $: settlementRow = type === 'category-adjustment' ? data.row : null;
  $: budgetRow = type === 'budget' ? data.row : null;

  function handleKeyDown(e) {
    if (e.key === 'Escape' && isOpen) {
      e.stopPropagation();
      closeDrilldown();
    }
  }
</script>

<svelte:window on:keydown={handleKeyDown} />

{#if isOpen}
  <!-- Modal Overlay Container -->
  <div class="fixed inset-0 z-50 !m-0 w-full h-full min-h-screen flex items-center justify-center p-3 sm:p-4 md:p-6 overflow-hidden">
    <!-- Backdrop button for click-outside dismissal -->
    <button
      type="button"
      class="fixed inset-0 bg-neutral-900/60 dark:bg-black/75 backdrop-blur-sm transition-opacity duration-200 cursor-default w-full h-full border-0 p-0 m-0"
      on:click={closeDrilldown}
      aria-label="Close dialog backdrop"
      tabindex="-1"
    ></button>

    <!-- Modal Card Surface with Concentric Radius and Animation -->
    <div
            role="dialog"
      aria-modal="true"
      aria-labelledby="drilldown-modal-title"
      tabindex="-1"
      class="drilldown-animate relative z-10 w-full max-w-2xl max-h-[90vh] flex flex-col bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 shadow-2xl rounded-2xl overflow-hidden"
    >
      <!-- ── Header ───────────────────────────────────────────────────────── -->
      <div class="px-5 py-4 sm:px-6 border-b border-neutral-200 dark:border-neutral-800 flex items-center justify-between gap-3 bg-neutral-50/70 dark:bg-neutral-950/40">
        <div class="flex items-center gap-3 min-w-0">
          <div class="w-10 h-10 rounded-xl bg-indigo-50 dark:bg-indigo-950/70 border border-indigo-200 dark:border-indigo-800/60 flex items-center justify-center text-lg flex-shrink-0">
            {#if type === 'category' || type === 'category-adjustment'}
              🏷️
            {:else if type === 'payer'}
              👤
            {:else if type === 'budget'}
              🎯
            {:else if type === 'project'}
              🚀
            {:else if type === 'joint-spent'}
              🏦
            {:else if type === 'income'}
              💰
            {:else}
              📊
            {/if}
          </div>

          <div class="min-w-0">
            <div class="flex items-center gap-2 flex-wrap">
              <h2 id="drilldown-modal-title" class="text-base sm:text-lg font-bold text-neutral-900 dark:text-white truncate">
                {title}
              </h2>
              {#if type === 'category-adjustment'}
                <span class="badge-amber text-[10px]">Settlement</span>
              {:else if type === 'budget'}
                <span class="badge-indigo text-[10px]">Budget Cap</span>
              {:else if type === 'joint-spent'}
                <span class="badge-indigo text-[10px]">Joint Account</span>
              {/if}
              {#if scopedLabel}
                <span class="inline-flex items-center gap-1 text-[10px] font-semibold px-2 py-0.5 rounded-full bg-indigo-50 dark:bg-indigo-950/80 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800/60" title="Filtered to dashboard scope">
                  <span>Scope:</span>
                  <span class="font-bold">{scopedLabel}</span>
                </span>
              {/if}
            </div>
            <p class="text-xs text-neutral-500 dark:text-neutral-400 truncate mt-0.5">
              {#if type === 'category'}
                Category expense distribution & contributing transactions &middot; {$selectedMonth}
              {:else if type === 'category-adjustment'}
                Reconciliation breakdown & contributing expenses &middot; {$selectedMonth}
              {:else if type === 'payer'}
                Expenses paid by {title} & category spread &middot; {$selectedMonth}
              {:else if type === 'budget'}
                Budget limit progress & itemized spend &middot; {$selectedMonth}
              {:else if type === 'project'}
                Savings milestone contributions & expenses &middot; {$selectedMonth}
              {:else if type === 'joint-spent'}
                Joint account expenses overview &middot; {$selectedMonth}
              {:else if type === 'income'}
                Household income streams & earnings &middot; {$selectedMonth}
              {:else}
                Monthly overview & itemized ledger &middot; {$selectedMonth}
              {/if}
              {#if scopedLabel} &middot; Scoped to {scopedLabel}{/if}
            </p>
          </div>
        </div>

        <!-- Accessible Close Button (Min 44x44px Touch Target) -->
        <button
          type="button"
          on:click={closeDrilldown}
          class="w-9 h-9 rounded-xl flex items-center justify-center text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 hover:bg-neutral-100 dark:hover:bg-neutral-800 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500 cursor-pointer flex-shrink-0"
          aria-label="Close drill-down details"
          title="Close (Esc)"
        >
          <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- ── Scrollable Body ───────────────────────────────────────────────── -->
      <div class="p-5 sm:p-6 overflow-y-auto space-y-6 flex-1">
        {#if loadError}
          <p class="text-xs text-rose-600 dark:text-red-400">{loadError}</p>
        {/if}

        {#if type !== 'income' && summary}
          <div class="grid grid-cols-2 sm:grid-cols-3 gap-3">
            <div class="card-sub p-3.5 space-y-1">
              <p class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wider">Total Spent</p>
              <p class="text-xl font-bold text-neutral-900 dark:text-white tabular-nums"><span class:privacy-masked={$privacyShield}>{$money(summary.total, true)}</span></p>
            </div>
            <div class="card-sub p-3.5 space-y-1">
              <p class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wider">Transactions</p>
              <p class="text-xl font-bold text-neutral-900 dark:text-white tabular-nums">{summary.count}</p>
            </div>
            <div class="card-sub p-3.5 space-y-1 col-span-2 sm:col-span-1">
              <p class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wider">Avg Transaction</p>
              <p class="text-xl font-bold text-neutral-900 dark:text-white tabular-nums"><span class:privacy-masked={$privacyShield}>{summary.average == null ? '—' : $money(summary.average)}</span></p>
            </div>
          </div>
        {/if}

        {#if budgetRow}
          <div class="card-sub p-4 space-y-2">
            <div class="flex justify-between items-baseline text-xs">
              <span class="font-semibold text-neutral-500">Budget used</span>
              <span class="font-bold text-indigo-600 dark:text-indigo-400 tabular-nums">{fmtPct(budgetRow.pct_used)}</span>
            </div>
            <div class="w-full h-2 bg-neutral-200 dark:bg-neutral-800 rounded-full overflow-hidden">
              <div class="h-full rounded-full bg-indigo-500" style="width: {Math.min(budgetRow.pct_used, 100)}%"></div>
            </div>
            <p class="text-xs text-neutral-600 dark:text-neutral-400">{$money(budgetRow.spent)} of {$money(budgetRow.limit)} · remaining {$money(budgetRow.remaining)}</p>
          </div>
        {/if}

        {#if progress}
          <div class="card-sub p-4 space-y-2">
            <div class="flex justify-between items-baseline text-xs">
              <span class="font-semibold text-neutral-500">Milestone Progress</span>
              <span class="font-bold text-indigo-600 dark:text-indigo-400 tabular-nums">{fmtPct(progress.progress_pct)}</span>
            </div>
            <div class="w-full h-2 bg-neutral-200 dark:bg-neutral-800 rounded-full overflow-hidden">
              <div class="h-full rounded-full bg-indigo-500" style="width: {progress.progress_pct ?? 0}%"></div>
            </div>
            <p class="text-xs text-neutral-600 dark:text-neutral-400 pt-1">
              Spent: <strong>{$money(progress.spent)}</strong> of target <strong>{$money(progress.target)}</strong>
            </p>
          </div>
        {/if}

        {#if settlementRow}
          <div class="space-y-3">
            <h3 class="text-xs font-bold uppercase tracking-wider text-neutral-700 dark:text-neutral-300">Settlement Allocation & Net Balance</h3>
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
              {#each Object.keys(settlementRow.net) as m}
                {@const net = settlementRow.net[m]}
                <div class="card-sub p-3 space-y-2 border-l-4" style="border-left-color: {memberColor(m)}">
                  <div class="flex items-center justify-between">
                    <span class="text-xs font-bold text-neutral-900 dark:text-white">{memberLabel(m)}</span>
                    <span class="text-xs font-semibold px-2 py-0.5 rounded-full" style="background-color: {memberColor(m)}15; color: {memberColor(m)}">
                      Agreed Share: {fmtPct(settlementRow.share_pct[m] ?? 0)}
                    </span>
                  </div>
                  <div class="flex items-center justify-between text-xs pt-1 border-t border-neutral-200 dark:border-neutral-800">
                    <span class="text-neutral-500">Paid: <strong class="text-neutral-800 dark:text-neutral-200 font-semibold tabular-nums">{$money(settlementRow.paid[m] ?? 0)}</strong></span>
                    <span class="font-bold tabular-nums {net > 0 ? 'text-emerald-600 dark:text-emerald-400' : net < 0 ? 'text-rose-600 dark:text-red-400' : 'text-neutral-500'}">
                      {net > 0 ? `+${$money(net)} owed back` : net < 0 ? `owes ${$money(Math.abs(net))}` : 'settled'}
                    </span>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        {#if breakdown.length > 0 && !settlementRow}
          <div class="space-y-3">
            <h3 class="text-xs font-bold uppercase tracking-wider text-neutral-700 dark:text-neutral-300">
              {breakdownKey === 'by_member' ? 'Spending per Member' : 'Spending by Category'}
            </h3>
            {#if breakdownKey === 'by_member'}
              <div class="flex h-4 rounded-lg overflow-hidden bg-neutral-100 dark:bg-neutral-800/60 p-0.5 border border-neutral-200 dark:border-neutral-800">
                {#each breakdown as b (b.key)}
                  <div class="h-full rounded-sm" style="width: {b.share_pct ?? 0}%; background-color: {b.color}; min-width: 4px;" title="{b.label}: {$money(b.total)}"></div>
                {/each}
              </div>
            {/if}
            <div class="space-y-2">
              {#each breakdown as b (b.key)}
                <div class="p-2.5 rounded-xl bg-neutral-50 dark:bg-neutral-950/70 border border-neutral-200 dark:border-neutral-800 space-y-1.5 border-l-4" style="border-left-color: {b.color}">
                  <div class="flex items-center justify-between text-xs">
                    <span class="font-semibold text-neutral-800 dark:text-neutral-200">{b.label} ({b.count})</span>
                    <span class="font-bold text-neutral-900 dark:text-white tabular-nums"><span class:privacy-masked={$privacyShield}>{$money(b.total)}</span> ({fmtPct(b.share_pct)})</span>
                  </div>
                  <div class="w-full h-1.5 bg-neutral-200 dark:bg-neutral-800 rounded-full overflow-hidden">
                    <div class="h-full rounded-full" style="width: {b.share_pct ?? 0}%; background-color: {b.color}"></div>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        {#if type === 'income'}
          <div class="space-y-3">
            <h3 class="text-xs font-bold uppercase tracking-wider text-neutral-700 dark:text-neutral-300">Household Member Income Streams</h3>
            {#each income as i (i.member)}
              {@const memberJobs = $jobs.filter((j) => j.member === i.member)}
              <div class="card-sub p-3.5 space-y-2 border-l-4" style="border-left-color: {memberColor(i.member)}">
                <div class="flex items-center justify-between">
                  <span class="text-xs font-bold text-neutral-900 dark:text-white">{memberLabel(i.member)}</span>
                  <span class="text-xs font-bold tabular-nums text-neutral-900 dark:text-white"><span class:privacy-masked={$privacyShield}>{$money(i.total)}</span></span>
                </div>
                <p class="text-[11px] text-neutral-500">
                  Base {$money(i.base)}{i.has_override ? ' (override)' : ''} · one-off {$money(i.one_off)}
                </p>
                {#each memberJobs as j (j.id)}
                  <div class="flex items-center justify-between bg-white/70 dark:bg-neutral-900/70 px-2.5 py-1.5 rounded-lg text-xs text-neutral-600 dark:text-neutral-400">
                    <span class="font-medium">{j.name} ({j.frequency})</span>
                    <span class="font-bold tabular-nums text-neutral-900 dark:text-white">{$money(j.amount)}</span>
                  </div>
                {/each}
              </div>
            {/each}
          </div>
        {/if}

        {#if type !== 'income'}
          <div class="space-y-3 pt-2 border-t border-neutral-200 dark:border-neutral-800">
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2.5">
              <h3 class="text-xs font-bold uppercase tracking-wider text-neutral-700 dark:text-neutral-300">Itemized Expenses ({filteredExpenses.length})</h3>
              <input type="text" bind:value={searchQuery} placeholder="Filter loaded expenses…" class="input-field py-1.5 px-3 text-xs w-full sm:w-56" />
            </div>
            {#if filteredExpenses.length > 0}
              <div class="space-y-2 max-h-72 overflow-y-auto pr-1">
                {#each filteredExpenses as exp (exp.id)}
                  <div class="card-sub p-3 flex items-center justify-between gap-3">
                    <div class="min-w-0 flex-1 space-y-1">
                      <div class="flex items-center gap-2 flex-wrap">
                        <span class="text-[11px] font-mono text-neutral-400 tabular-nums">{fmtDate(exp.date)}</span>
                        <span class="text-xs font-semibold text-neutral-900 dark:text-white truncate">{exp.name}</span>
                        {#if exp.account != null}<span class="badge-indigo text-[9px] py-0 px-1.5">Joint</span>{/if}
                        {#if $tagById.get(exp.tag)}
                          {@const tag = $tagById.get(exp.tag)}
                          <span class="text-[9px] px-1.5 py-0.5 rounded font-medium border" style="background-color: {tag.color}15; color: {tag.color}; border-color: {tag.color}40">#{tag.name}</span>
                        {/if}
                        {#if $projectById.get(exp.project)}
                          <span class="text-[9px] px-1.5 py-0.5 rounded font-medium bg-neutral-200 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300">📁 {$projectById.get(exp.project).name}</span>
                        {/if}
                      </div>
                      <div class="flex items-center gap-2 text-[11px] text-neutral-500">
                        <span class="font-medium" style="color: {memberColor(exp.member)}">{memberLabel(exp.member)}</span>
                        <span>&middot;</span>
                        <span class="text-neutral-400">{$categoryName(exp.category)}</span>
                        {#if exp.overrides.length > 0}<span class="text-amber-600 dark:text-amber-400 font-medium">· Custom split</span>{/if}
                      </div>
                    </div>
                    <span class="text-sm font-bold text-neutral-900 dark:text-white tabular-nums flex-none pl-2"><span class:privacy-masked={$privacyShield}>{$money(exp.amount)}</span></span>
                  </div>
                {/each}
              </div>
            {:else}
              <div class="empty-state-box py-6">
                <p class="text-xs font-semibold text-neutral-700 dark:text-neutral-300">{searchQuery ? 'No matching expenses found' : 'No expenses logged for this item yet'}</p>
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <!-- ── Footer ───────────────────────────────────────────────────────── -->
      <div class="px-5 py-3 sm:px-6 border-t border-neutral-200 dark:border-neutral-800 flex items-center justify-between bg-neutral-50/50 dark:bg-neutral-950/30 text-xs text-neutral-500">
        <span>Press <kbd class="px-1.5 py-0.5 rounded bg-neutral-200 dark:bg-neutral-800 text-[10px] font-mono text-neutral-700 dark:text-neutral-300">Esc</kbd> to close</span>
        <button
          type="button"
          on:click={closeDrilldown}
          class="btn-secondary py-1.5 px-3.5 text-xs cursor-pointer"
        >
          Done
        </button>
      </div>

    </div>
  </div>
{/if}

<style>
  @keyframes drilldownIn {
    from {
      opacity: 0;
      transform: scale(0.96) translateY(6px);
    }
    to {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }

  .drilldown-animate {
    animation: drilldownIn 200ms cubic-bezier(0.23, 1, 0.32, 1) both;
  }

  @media (prefers-reduced-motion: reduce) {
    .drilldown-animate {
      animation: none !important;
    }
  }
</style>
