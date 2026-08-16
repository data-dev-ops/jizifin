<script>
  import { recurringExpenses, splits, users, currencySymbol, selectedMonth } from './stores.js';
  import { createRecurring, updateRecurring, deleteRecurring } from './api.js';

  $: activeUsers = $users.filter((u) => u.is_active);

  const FREQUENCIES = [
    { value: 'monthly', label: '🗓️ Monthly', desc: 'Once per month on a set day' },
    { value: 'weekly', label: '⚡ Weekly', desc: 'Every 7 days from start date' },
    { value: 'biweekly', label: '⚡ Biweekly', desc: 'Every 14 days (2 weeks)' },
    { value: '4-weekly', label: '🔄 4-Weekly', desc: 'Every 28 days (4 weeks)' },
    { value: 'quarterly', label: '📅 Quarterly', desc: 'Every 3 months' },
    { value: 'annual', label: '🌟 Annual', desc: 'Once per year' },
  ];

  function todayIso() {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
  }

  let form = {
    name: '',
    cost_euros: '',
    who_paid: '',
    category: '',
    frequency: 'monthly',
    day_of_month: 1,
    start_date: todayIso(),
    end_date: '',
    is_joint: false,
    is_active: true,
  };

  // Default who_paid to first active user once users load
  $: if (!form.who_paid && activeUsers.length > 0) form.who_paid = activeUsers[0].name;

  let saving = false;
  let error = '';

  // Per-row delete confirmation state
  let confirmDeleteId = null;
  let deletingId = null;

  // Edit modal state
  let editingItem = null;
  let editForm = {
    id: null,
    name: '',
    cost_euros: '',
    who_paid: '',
    category: '',
    frequency: 'monthly',
    day_of_month: 1,
    start_date: '',
    end_date: '',
    is_joint: false,
    is_active: true,
  };
  let editSaving = false;
  let editError = '';

  function parseDateYmd(str) {
    if (!str) return null;
    const parts = str.split('-').map(Number);
    if (parts.length < 3 || isNaN(parts[0])) return null;
    return new Date(parts[0], parts[1] - 1, parts[2]);
  }

  // Calculate occurrences of a recurring template in a target month
  function getOccurrencesInMonth(rec, targetMonth) {
    if (rec.is_active === false) return [];
    if (!targetMonth) return [];
    const [y, m] = targetMonth.split('-').map(Number);
    const daysInMonth = new Date(y, m, 0).getDate();
    const mStart = new Date(y, m - 1, 1);
    const mEnd = new Date(y, m - 1, daysInMonth, 23, 59, 59, 999);

    const startD = parseDateYmd(rec.start_date || '2026-01-01');
    if (!startD) return [];
    let endD = null;
    if (rec.end_date) {
      const parsedEnd = parseDateYmd(rec.end_date);
      if (parsedEnd) {
        parsedEnd.setHours(23, 59, 59, 999);
        endD = parsedEnd;
      }
    }

    if (startD > mEnd) return [];
    if (endD && endD < mStart) return [];

    const freq = rec.frequency || 'monthly';
    const dates = [];

    if (freq === 'weekly' || freq === 'biweekly' || freq === '4-weekly') {
      const intervalDays = freq === 'weekly' ? 7 : (freq === 'biweekly' ? 14 : 28);
      let curr = new Date(startD);
      if (curr < mStart) {
        const diffDays = Math.floor((mStart.getTime() - curr.getTime()) / (1000 * 60 * 60 * 24));
        const steps = Math.ceil(diffDays / intervalDays);
        curr.setDate(curr.getDate() + steps * intervalDays);
      }
      while (curr <= mEnd) {
        if (endD && curr > endD) break;
        if (curr >= startD) {
          const yr = curr.getFullYear();
          const mo = String(curr.getMonth() + 1).padStart(2, '0');
          const dy = String(curr.getDate()).padStart(2, '0');
          dates.push(`${yr}-${mo}-${dy}`);
        }
        curr.setDate(curr.getDate() + intervalDays);
      }
    } else if (freq === 'monthly') {
      const dom = (rec.day_of_month && rec.day_of_month >= 1 && rec.day_of_month <= 31) ? rec.day_of_month : startD.getDate();
      const execDay = Math.min(dom, daysInMonth);
      const dt = new Date(y, m - 1, execDay);
      if (dt >= startD && (!endD || dt <= endD)) {
        dates.push(`${y}-${String(m).padStart(2, '0')}-${String(execDay).padStart(2, '0')}`);
      }
    } else if (freq === 'quarterly') {
      const startMonthIdx = startD.getFullYear() * 12 + startD.getMonth();
      const targetMonthIdx = y * 12 + (m - 1);
      const diff = targetMonthIdx - startMonthIdx;
      if (diff >= 0 && diff % 3 === 0) {
        const dom = (rec.day_of_month && rec.day_of_month >= 1 && rec.day_of_month <= 31) ? rec.day_of_month : startD.getDate();
        const execDay = Math.min(dom, daysInMonth);
        const dt = new Date(y, m - 1, execDay);
        if (dt >= startD && (!endD || dt <= endD)) {
          dates.push(`${y}-${String(m).padStart(2, '0')}-${String(execDay).padStart(2, '0')}`);
        }
      }
    } else if (freq === 'annual') {
      if ((m - 1) === startD.getMonth() && y >= startD.getFullYear()) {
        const dom = (rec.day_of_month && rec.day_of_month >= 1 && rec.day_of_month <= 31) ? rec.day_of_month : startD.getDate();
        const execDay = Math.min(dom, daysInMonth);
        const dt = new Date(y, m - 1, execDay);
        if (dt >= startD && (!endD || dt <= endD)) {
          dates.push(`${y}-${String(m).padStart(2, '0')}-${String(execDay).padStart(2, '0')}`);
        }
      }
    }

    return dates.sort();
  }

  // Reactive items with occurrences and cost in the selected month
  $: enrichedExpenses = $recurringExpenses.map((rec) => {
    const dates = getOccurrencesInMonth(rec, $selectedMonth);
    const occurrences = dates.length;
    const monthCostCents = occurrences * rec.cost_cents;
    return {
      ...rec,
      datesInMonth: dates,
      occurrencesInMonth: occurrences,
      monthCostCents,
    };
  });

  // Aggregates for current month
  $: totalMonthCommitmentCents = enrichedExpenses.reduce((sum, r) => sum + r.monthCostCents, 0);
  $: activeCommitmentsCount = $recurringExpenses.filter((r) => r.is_active !== false).length;

  // Category breakdown for current month
  $: categoryBreakdown = (() => {
    const map = {};
    for (const r of enrichedExpenses) {
      if (r.occurrencesInMonth > 0) {
        if (!map[r.category]) {
          map[r.category] = { category: r.category, totalCents: 0, count: 0, items: [] };
        }
        map[r.category].totalCents += r.monthCostCents;
        map[r.category].count += r.occurrencesInMonth;
        map[r.category].items.push(r);
      }
    }
    return Object.values(map).sort((a, b) => b.totalCents - a.totalCents);
  })();

  $: monthLabel = (() => {
    if (!$selectedMonth) return '';
    const [y, m] = $selectedMonth.split('-');
    return new Date(Number(y), Number(m) - 1, 1).toLocaleDateString('en-GB', { month: 'long', year: 'numeric' });
  })();

  function shiftMonth(delta) {
    const [y, m] = $selectedMonth.split('-').map(Number);
    const d = new Date(y, m - 1 + delta, 1);
    selectedMonth.set(`${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}`);
  }

  async function handleSubmit() {
    error = '';
    const cost_cents = Math.round(parseFloat(form.cost_euros) * 100);
    if (!form.name || isNaN(cost_cents) || cost_cents <= 0 || !form.category) {
      error = 'Fill in all fields with valid values.';
      return;
    }
    if (form.end_date && form.start_date && form.end_date < form.start_date) {
      error = 'End date cannot be earlier than start date.';
      return;
    }

    saving = true;
    try {
      await createRecurring({
        name: form.name,
        cost_cents,
        who_paid: form.who_paid,
        category: form.category,
        frequency: form.frequency,
        day_of_month: Number(form.day_of_month) || 1,
        start_date: form.start_date || todayIso(),
        end_date: form.end_date || null,
        is_joint: form.is_joint,
        is_active: form.is_active,
      });
      form = {
        name: '',
        cost_euros: '',
        who_paid: activeUsers[0]?.name ?? '',
        category: '',
        frequency: 'monthly',
        day_of_month: 1,
        start_date: todayIso(),
        end_date: '',
        is_joint: false,
        is_active: true,
      };
    } catch (e) {
      error = e.message;
    } finally {
      saving = false;
    }
  }

  function startEdit(item) {
    editingItem = item;
    editError = '';
    editForm = {
      id: item.id,
      name: item.name,
      cost_euros: (item.cost_cents / 100).toFixed(2),
      who_paid: item.who_paid,
      category: item.category,
      frequency: item.frequency || 'monthly',
      day_of_month: item.day_of_month || 1,
      start_date: item.start_date || '2026-01-01',
      end_date: item.end_date || '',
      is_joint: Boolean(item.is_joint),
      is_active: item.is_active !== false,
    };
  }

  function cancelEdit() {
    editingItem = null;
    editError = '';
  }

  async function handleEditSubmit() {
    editError = '';
    const cost_cents = Math.round(parseFloat(editForm.cost_euros) * 100);
    if (!editForm.name || isNaN(cost_cents) || cost_cents <= 0 || !editForm.category) {
      editError = 'Fill in all fields with valid values.';
      return;
    }
    if (editForm.end_date && editForm.start_date && editForm.end_date < editForm.start_date) {
      editError = 'End date cannot be earlier than start date.';
      return;
    }

    editSaving = true;
    try {
      await updateRecurring(editForm.id, {
        name: editForm.name,
        cost_cents,
        who_paid: editForm.who_paid,
        category: editForm.category,
        frequency: editForm.frequency,
        day_of_month: Number(editForm.day_of_month) || 1,
        start_date: editForm.start_date,
        end_date: editForm.end_date || null,
        is_joint: editForm.is_joint,
        is_active: editForm.is_active,
      });
      editingItem = null;
    } catch (e) {
      editError = e.message;
    } finally {
      editSaving = false;
    }
  }

  async function toggleActive(rec) {
    try {
      await updateRecurring(rec.id, { is_active: !rec.is_active });
    } catch (e) {
      console.error('Failed to toggle active status:', e);
    }
  }

  function requestDelete(id) {
    confirmDeleteId = id;
  }

  function cancelDelete() {
    confirmDeleteId = null;
  }

  async function confirmDelete(id) {
    error = '';
    deletingId = id;
    try {
      await deleteRecurring(id);
      confirmDeleteId = null;
    } catch (e) {
      error = e.message;
    } finally {
      deletingId = null;
    }
  }

  function ordinal(n) {
    if (!n) return '';
    const s = ['th', 'st', 'nd', 'rd'];
    const v = n % 100;
    return n + (s[(v - 20) % 10] || s[v] || s[0]);
  }

  function formatShortDate(dateStr) {
    if (!dateStr) return '';
    const [y, m, d] = dateStr.split('-').map(Number);
    const date = new Date(y, m - 1, d);
    return date.toLocaleDateString('en-GB', { day: 'numeric', month: 'short' });
  }

  function getFrequencyBadge(freq) {
    switch (freq) {
      case 'weekly':
        return { label: 'Weekly', icon: '⚡', color: 'bg-emerald-950/80 text-emerald-300 border-emerald-800/60' };
      case 'biweekly':
        return { label: 'Biweekly', icon: '⚡', color: 'bg-teal-950/80 text-teal-300 border-teal-800/60' };
      case '4-weekly':
        return { label: '4-Weekly', icon: '🔄', color: 'bg-cyan-950/80 text-cyan-300 border-cyan-800/60' };
      case 'quarterly':
        return { label: 'Quarterly', icon: '📅', color: 'bg-purple-950/80 text-purple-300 border-purple-800/60' };
      case 'annual':
        return { label: 'Annual', icon: '🌟', color: 'bg-amber-950/80 text-amber-300 border-amber-800/60' };
      case 'monthly':
      default:
        return { label: 'Monthly', icon: '🗓️', color: 'bg-sky-950/80 text-sky-300 border-sky-800/60' };
    }
  }
</script>

<div class="space-y-6">
  <!-- ── Header & Month Context ──────────────────────────────────────── -->
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
    <div>
      <h2 class="text-lg font-bold text-neutral-100 flex items-center gap-2">
        <span>Recurring Commitments</span>
        <span class="text-xs font-semibold px-2.5 py-0.5 rounded-full bg-neutral-800 text-neutral-300 border border-neutral-700">
          {activeCommitmentsCount} Active
        </span>
      </h2>
      <p class="text-xs text-neutral-400 mt-0.5">
        Manage automated subscriptions, fixed bills, and flexible frequency schedules.
      </p>
    </div>

    <!-- Month Switcher for projection preview -->
    <div class="inline-flex items-center gap-2 bg-neutral-900 border border-neutral-800 rounded-xl px-3 py-1.5 self-start sm:self-auto shadow-sm">
      <span class="text-xs text-neutral-400 font-medium">Viewing for:</span>
      <button
        type="button"
        on:click={() => shiftMonth(-1)}
        class="w-6 h-6 flex items-center justify-center rounded text-neutral-400 hover:text-white hover:bg-neutral-800 transition-colors text-sm cursor-pointer"
        aria-label="Previous month"
      >‹</button>
      <span class="text-xs font-bold text-indigo-400 tabular-nums">{monthLabel}</span>
      <button
        type="button"
        on:click={() => shiftMonth(1)}
        class="w-6 h-6 flex items-center justify-center rounded text-neutral-400 hover:text-white hover:bg-neutral-800 transition-colors text-sm cursor-pointer"
        aria-label="Next month"
      >›</button>
    </div>
  </div>

  <!-- ── Summary KPI Cards ────────────────────────────────────────────── -->
  <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
    <!-- Month Total Commitment -->
    <div class="card-sub p-4 flex flex-col justify-between relative overflow-hidden group">
      <div class="flex items-center justify-between mb-2">
        <span class="text-xs font-medium text-neutral-400">Total in {monthLabel}</span>
        <span class="text-xs px-2 py-0.5 rounded bg-indigo-950/80 text-indigo-300 font-semibold border border-indigo-800/50">
          Committed
        </span>
      </div>
      <div class="flex items-baseline gap-2">
        <span class="text-2xl font-extrabold text-neutral-100 tabular-nums">
          {$currencySymbol}{(totalMonthCommitmentCents / 100).toFixed(2)}
        </span>
      </div>
      <p class="text-[11px] text-neutral-500 mt-2">
        Sum of all scheduled payments due in {monthLabel}
      </p>
    </div>

    <!-- Active Schedules Count -->
    <div class="card-sub p-4 flex flex-col justify-between">
      <div class="flex items-center justify-between mb-2">
        <span class="text-xs font-medium text-neutral-400">Active Schedules</span>
        <span class="text-xs px-2 py-0.5 rounded bg-emerald-950/80 text-emerald-300 font-semibold border border-emerald-800/50">
          Status
        </span>
      </div>
      <div class="flex items-baseline gap-2">
        <span class="text-2xl font-extrabold text-neutral-100 tabular-nums">
          {activeCommitmentsCount}
        </span>
        <span class="text-xs text-neutral-400 font-medium">/ {$recurringExpenses.length} templates</span>
      </div>
      <p class="text-[11px] text-neutral-500 mt-2">
        Templates active and participating in automatic generation
      </p>
    </div>

    <!-- Unique Categories -->
    <div class="card-sub p-4 flex flex-col justify-between">
      <div class="flex items-center justify-between mb-2">
        <span class="text-xs font-medium text-neutral-400">Categories Impacted</span>
        <span class="text-xs px-2 py-0.5 rounded bg-amber-950/80 text-amber-300 font-semibold border border-amber-800/50">
          Breakdown
        </span>
      </div>
      <div class="flex items-baseline gap-2">
        <span class="text-2xl font-extrabold text-neutral-100 tabular-nums">
          {categoryBreakdown.length}
        </span>
        <span class="text-xs text-neutral-400 font-medium">categories this month</span>
      </div>
      <p class="text-[11px] text-neutral-500 mt-2">
        Expense categories with recurring payments in {monthLabel}
      </p>
    </div>
  </div>

  <!-- ── Per-Category Breakdown Chips & Details ────────────────────────── -->
  {#if categoryBreakdown.length > 0}
    <div class="card-sub p-4">
      <div class="flex items-center justify-between mb-3">
        <h3 class="text-xs font-bold uppercase tracking-wider text-neutral-400">
          Category Spending in {monthLabel}
        </h3>
        <span class="text-xs text-neutral-500">
          {$currencySymbol}{(totalMonthCommitmentCents / 100).toFixed(2)} total
        </span>
      </div>
      <div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
        {#each categoryBreakdown as cat}
          {@const pct = totalMonthCommitmentCents > 0 ? Math.round((cat.totalCents / totalMonthCommitmentCents) * 100) : 0}
          <div class="p-3 rounded-xl bg-neutral-900/80 border border-neutral-800 flex flex-col justify-between">
            <div class="flex items-start justify-between gap-1 mb-1">
              <span class="text-xs font-semibold text-neutral-200 truncate">{cat.category}</span>
              <span class="text-[10px] font-bold text-neutral-400 tabular-nums bg-neutral-800 px-1.5 py-0.5 rounded">
                {cat.count}×
              </span>
            </div>
            <div class="flex items-baseline justify-between mt-1">
              <span class="text-sm font-bold text-sky-400 tabular-nums">
                {$currencySymbol}{(cat.totalCents / 100).toFixed(2)}
              </span>
              <span class="text-[10px] text-neutral-500 font-medium">{pct}%</span>
            </div>
            <div class="mt-2 h-1 bg-neutral-800 rounded-full overflow-hidden">
              <div class="h-full bg-gradient-to-r from-sky-500 to-indigo-500 rounded-full" style="width: {pct}%"></div>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- ── Existing Templates Table ────────────────────────────────────── -->
  {#if $recurringExpenses.length === 0}
    <div class="text-center text-neutral-500 text-sm py-12 border border-dashed border-neutral-800 rounded-2xl">
      <p class="text-base font-semibold text-neutral-300 mb-1">No recurring expenses configured yet</p>
      <p class="text-xs text-neutral-500 max-w-sm mx-auto">
        Add subscriptions, utilities, and fixed commitments below to automate your monthly budget tracking.
      </p>
    </div>
  {:else}
    <div class="overflow-x-auto rounded-2xl border border-neutral-800 bg-neutral-900/40">
      <table class="w-full text-sm border-collapse">
        <thead>
          <tr class="bg-neutral-950/80 border-b border-neutral-800">
            <th class="text-left text-xs font-semibold text-neutral-400 uppercase tracking-wider px-4 py-3">Name</th>
            <th class="text-left text-xs font-semibold text-neutral-400 uppercase tracking-wider px-4 py-3">Amount</th>
            <th class="text-left text-xs font-semibold text-neutral-400 uppercase tracking-wider px-4 py-3">Paid by</th>
            <th class="text-left text-xs font-semibold text-neutral-400 uppercase tracking-wider px-4 py-3">Category</th>
            <th class="text-left text-xs font-semibold text-neutral-400 uppercase tracking-wider px-4 py-3">Frequency & Day</th>
            <th class="text-left text-xs font-semibold text-neutral-400 uppercase tracking-wider px-4 py-3">Due in {monthLabel}</th>
            <th class="px-4 py-3 text-right"></th>
          </tr>
        </thead>
        <tbody class="divide-y divide-neutral-800/60">
          {#each enrichedExpenses as rec (rec.id)}
            {@const payerColor = ($users.find((u) => u.name === rec.who_paid)?.color ?? '#6366f1')}
            {@const badge = getFrequencyBadge(rec.frequency)}
            {@const isPaused = rec.is_active === false}
            <tr class="hover:bg-neutral-800/30 transition-colors group {isPaused ? 'opacity-50' : ''}">
              <!-- Name & Active status -->
              <td class="px-4 py-3.5">
                <div class="flex items-center gap-2">
                  <span class="font-medium text-neutral-100">{rec.name}</span>
                  {#if isPaused}
                    <span class="px-1.5 py-0.5 rounded text-[10px] font-bold uppercase tracking-wider bg-neutral-800 text-neutral-400 border border-neutral-700">
                      Paused
                    </span>
                  {/if}
                </div>
              </td>

              <!-- Cost -->
              <td class="px-4 py-3.5 font-semibold tabular-nums text-sky-400">
                {$currencySymbol}{(rec.cost_cents / 100).toFixed(2)}
              </td>

              <!-- Paid by -->
              <td class="px-4 py-3.5">
                {#if rec.is_joint}
                  <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-semibold bg-indigo-950/80 text-indigo-300 border border-indigo-800/60">
                    🏦 Joint Account
                  </span>
                {:else}
                  <span
                    class="inline-block px-2 py-0.5 rounded-full text-xs font-semibold"
                    style="background-color: {payerColor}22; color: {payerColor}; border: 1px solid {payerColor}44;"
                  >
                    {rec.who_paid}
                  </span>
                {/if}
              </td>

              <!-- Category -->
              <td class="px-4 py-3.5 text-xs text-neutral-400">
                {rec.category}
              </td>

              <!-- Frequency & Day (matches old test looking for dayOrdinal) -->
              <td class="px-4 py-3.5 text-neutral-400 text-xs">
                <div class="flex items-center gap-1.5">
                  <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-semibold border {badge.color} w-fit">
                    <span>{badge.icon}</span>
                    <span>{badge.label}</span>
                  </span>
                  {#if rec.frequency === 'monthly' && rec.day_of_month}
                    <span class="text-neutral-300 font-medium">{ordinal(rec.day_of_month)}</span>
                  {/if}
                </div>
              </td>

              <!-- Occurrences and Impact in current month -->
              <td class="px-4 py-3.5">
                {#if isPaused}
                  <span class="text-xs text-neutral-500 italic">Paused</span>
                {:else if rec.occurrencesInMonth === 0}
                  <span class="text-xs text-neutral-500">None in {monthLabel}</span>
                {:else}
                  <div class="flex flex-col gap-0.5">
                    <span class="text-xs font-bold text-neutral-100 tabular-nums">
                      {rec.occurrencesInMonth}× ({$currencySymbol}{(rec.monthCostCents / 100).toFixed(2)})
                    </span>
                    <span class="text-[10px] text-neutral-400">
                      {rec.datesInMonth.map(formatShortDate).join(', ')}
                    </span>
                  </div>
                {/if}
              </td>

              <!-- Actions: Edit, Pause/Resume, Delete -->
              <td class="px-4 py-3.5 text-right whitespace-nowrap">
                {#if confirmDeleteId === rec.id}
                  <span class="inline-flex items-center gap-1.5">
                    <span class="text-xs text-neutral-400">Remove?</span>
                    <button
                      type="button"
                      on:click={() => confirmDelete(rec.id)}
                      disabled={deletingId === rec.id}
                      class="px-2 py-0.5 rounded text-xs font-semibold bg-red-600 hover:bg-red-500 disabled:opacity-40 transition-colors text-white cursor-pointer"
                    >
                      {deletingId === rec.id ? '…' : 'Yes'}
                    </button>
                    <button
                      type="button"
                      on:click={cancelDelete}
                      class="px-2 py-0.5 rounded text-xs font-semibold bg-neutral-700 hover:bg-neutral-600 transition-colors text-white cursor-pointer"
                    >
                      No
                    </button>
                  </span>
                {:else}
                  <div class="inline-flex items-center gap-1">
                    <!-- Toggle active button -->
                    <button
                      type="button"
                      on:click={() => toggleActive(rec)}
                      title={isPaused ? 'Resume schedule' : 'Pause schedule'}
                      class="p-1 rounded-lg text-neutral-400 hover:text-neutral-100 hover:bg-neutral-800 transition-colors cursor-pointer"
                    >
                      {#if isPaused}
                        <svg class="w-3.5 h-3.5 text-emerald-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                          <polygon points="5 3 19 12 5 21 5 3"/>
                        </svg>
                      {:else}
                        <svg class="w-3.5 h-3.5 text-neutral-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                          <rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/>
                        </svg>
                      {/if}
                    </button>

                    <!-- Edit button -->
                    <button
                      type="button"
                      on:click={() => startEdit(rec)}
                      title="Edit recurring expense"
                      class="p-1 rounded-lg text-neutral-400 hover:text-sky-400 hover:bg-sky-950/40 transition-colors cursor-pointer"
                    >
                      <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                        <path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/>
                      </svg>
                    </button>

                    <!-- Delete button -->
                    <button
                      type="button"
                      on:click={() => requestDelete(rec.id)}
                      title="Remove"
                      class="p-1 rounded-lg text-neutral-500 hover:text-red-400 hover:bg-red-950/40 transition-colors cursor-pointer"
                    >
                      <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                        <polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14H6L5 6"/><path d="M10 11v6M14 11v6"/>
                      </svg>
                    </button>
                  </div>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}

  <!-- ── Add New Template Form ────────────────────────────────────────── -->
  <div class="card-sub p-5 border border-neutral-800 rounded-2xl bg-neutral-900/60">
    <h3 class="text-sm font-semibold text-neutral-200 mb-1">Add Recurring Expense</h3>
    <p class="text-xs text-neutral-400 mb-4">Set up a new recurring commitment with custom interval and timeline bounds.</p>

    {#if error}
      <div class="bg-red-950/40 border border-red-800 text-red-400 rounded-xl px-3.5 py-2 text-xs mb-4">
        {error}
      </div>
    {/if}

    <form on:submit|preventDefault={handleSubmit} class="space-y-4">
      <!-- Name and Amount -->
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div>
          <label for="rec-name" class="block text-xs font-medium text-neutral-400 mb-1.5">Expense Name</label>
          <input
            id="rec-name"
            class="input-field"
            placeholder="e.g. Netflix, Rent, Internet"
            bind:value={form.name}
          />
        </div>
        <div>
          <label for="rec-amount" class="block text-xs font-medium text-neutral-400 mb-1.5">Amount ({$currencySymbol})</label>
          <input
            id="rec-amount"
            class="input-field tabular-nums"
            type="number"
            min="0.01"
            step="0.01"
            placeholder="0.00"
            bind:value={form.cost_euros}
          />
        </div>
      </div>

      <!-- Frequency, Category & Day of Month / Start date -->
      <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
        <div>
          <label for="rec-frequency" class="block text-xs font-medium text-neutral-400 mb-1.5">Frequency / Interval</label>
          <select
            id="rec-frequency"
            class="select-field"
            bind:value={form.frequency}
          >
            {#each FREQUENCIES as f}
              <option value={f.value}>{f.label}</option>
            {/each}
          </select>
        </div>

        <div>
          <label for="rec-cat" class="block text-xs font-medium text-neutral-400 mb-1.5">Category</label>
          <select
            id="rec-cat"
            class="select-field"
            bind:value={form.category}
          >
            <option value="" disabled>Select category…</option>
            {#each $splits as s}
              <option value={s.category}>{s.category}</option>
            {/each}
          </select>
        </div>

        {#if form.frequency === 'monthly'}
          <div>
            <label for="rec-day" class="block text-xs font-medium text-neutral-400 mb-1.5">Day of Month</label>
            <input
              id="rec-day"
              type="number"
              min="1"
              max="31"
              class="input-field"
              bind:value={form.day_of_month}
            />
          </div>
        {:else}
          <div>
            <label for="rec-start-anchor" class="block text-xs font-medium text-neutral-400 mb-1.5">Anchor Start Date</label>
            <input
              id="rec-start-anchor"
              type="date"
              class="input-field"
              bind:value={form.start_date}
            />
          </div>
        {/if}
      </div>

      <!-- Timeline bounds: Start Date and Optional End Date -->
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
        {#if form.frequency === 'monthly'}
          <div>
            <label for="rec-start-date" class="block text-xs font-medium text-neutral-400 mb-1.5">Start Date</label>
            <input
              id="rec-start-date"
              type="date"
              class="input-field"
              bind:value={form.start_date}
            />
          </div>
        {/if}

        <div>
          <label for="rec-end-date" class="block text-xs font-medium text-neutral-400 mb-1.5">
            End Date <span class="text-neutral-500 font-normal">(optional)</span>
          </label>
          <input
            id="rec-end-date"
            type="date"
            class="input-field"
            placeholder="No end date"
            bind:value={form.end_date}
          />
        </div>
      </div>

      <!-- Payer and Joint Source -->
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-4 items-end">
        <div>
          <label for="rec-payer" class="block text-xs font-medium text-neutral-400 mb-1.5">Paid by</label>
          <select
            id="rec-payer"
            class="select-field"
            bind:value={form.who_paid}
            disabled={form.is_joint}
          >
            {#each activeUsers as u}
              <option value={u.name}>{u.name}</option>
            {/each}
          </select>
        </div>

        <div class="flex flex-col gap-1.5">
          <span class="text-xs font-medium text-neutral-400">Payment Source</span>
          <div class="inline-flex rounded-xl bg-neutral-950 p-1 border border-neutral-800 h-[42px] items-center self-start">
            <button
              type="button"
              id="rec-source-personal"
              on:click={() => (form.is_joint = false)}
              class="px-4 py-1.5 rounded-lg text-xs font-semibold transition-colors cursor-pointer {!form.is_joint ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-400 hover:text-neutral-200'}"
            >
              Personal
            </button>
            <button
              type="button"
              id="rec-source-joint"
              on:click={() => (form.is_joint = true)}
              class="px-4 py-1.5 rounded-lg text-xs font-semibold transition-colors cursor-pointer {form.is_joint ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-400 hover:text-neutral-200'}"
            >
              🏦 Joint
            </button>
          </div>
        </div>
      </div>

      <button
        type="submit"
        disabled={saving}
        class="btn-primary cursor-pointer"
      >
        {saving ? 'Saving…' : '+ Add Recurring'}
      </button>
    </form>
  </div>
</div>

<!-- ── Edit Recurring Modal ────────────────────────────────────────────── -->
{#if editingItem}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm animate-fadeIn">
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div
      class="bg-neutral-900 border border-neutral-800 rounded-2xl max-w-xl w-full p-6 shadow-2xl space-y-5 max-h-[90vh] overflow-y-auto"
      on:click|stopPropagation
    >
      <div class="flex items-center justify-between border-b border-neutral-800 pb-4">
        <div>
          <h3 class="text-base font-bold text-neutral-100">Edit Recurring Commitment</h3>
          <p class="text-xs text-neutral-400 mt-0.5">Update schedule, amount, interval, or timeline bounds.</p>
        </div>
        <button
          type="button"
          on:click={cancelEdit}
          class="p-1.5 rounded-lg text-neutral-400 hover:text-white hover:bg-neutral-800 transition-colors"
        >
          ✕
        </button>
      </div>

      {#if editError}
        <div class="bg-red-950/40 border border-red-800 text-red-400 rounded-xl px-3.5 py-2.5 text-xs">
          {editError}
        </div>
      {/if}

      <form on:submit|preventDefault={handleEditSubmit} class="space-y-4">
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <div>
            <label for="edit-rec-name" class="block text-xs font-medium text-neutral-400 mb-1.5">Expense Name</label>
            <input
              id="edit-rec-name"
              class="input-field"
              bind:value={editForm.name}
              required
            />
          </div>
          <div>
            <label for="edit-rec-amount" class="block text-xs font-medium text-neutral-400 mb-1.5">Amount ({$currencySymbol})</label>
            <input
              id="edit-rec-amount"
              class="input-field tabular-nums"
              type="number"
              min="0.01"
              step="0.01"
              bind:value={editForm.cost_euros}
              required
            />
          </div>
        </div>

        <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
          <div>
            <label for="edit-rec-frequency" class="block text-xs font-medium text-neutral-400 mb-1.5">Frequency</label>
            <select
              id="edit-rec-frequency"
              class="select-field"
              bind:value={editForm.frequency}
            >
              {#each FREQUENCIES as f}
                <option value={f.value}>{f.label}</option>
              {/each}
            </select>
          </div>

          <div>
            <label for="edit-rec-cat" class="block text-xs font-medium text-neutral-400 mb-1.5">Category</label>
            <select
              id="edit-rec-cat"
              class="select-field"
              bind:value={editForm.category}
              required
            >
              {#each $splits as s}
                <option value={s.category}>{s.category}</option>
              {/each}
            </select>
          </div>

          {#if editForm.frequency === 'monthly'}
            <div>
              <label for="edit-rec-day" class="block text-xs font-medium text-neutral-400 mb-1.5">Day of Month</label>
              <input
                id="edit-rec-day"
                type="number"
                min="1"
                max="31"
                class="input-field"
                bind:value={editForm.day_of_month}
              />
            </div>
          {/if}
        </div>

        <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <div>
            <label for="edit-rec-start" class="block text-xs font-medium text-neutral-400 mb-1.5">Start Date</label>
            <input
              id="edit-rec-start"
              type="date"
              class="input-field"
              bind:value={editForm.start_date}
              required
            />
          </div>
          <div>
            <label for="edit-rec-end" class="block text-xs font-medium text-neutral-400 mb-1.5">End Date (optional)</label>
            <input
              id="edit-rec-end"
              type="date"
              class="input-field"
              placeholder="No end date"
              bind:value={editForm.end_date}
            />
          </div>
        </div>

        <div class="grid grid-cols-1 sm:grid-cols-2 gap-4 items-end">
          <div>
            <label for="edit-rec-payer" class="block text-xs font-medium text-neutral-400 mb-1.5">Paid by</label>
            <select
              id="edit-rec-payer"
              class="select-field"
              bind:value={editForm.who_paid}
              disabled={editForm.is_joint}
            >
              {#each activeUsers as u}
                <option value={u.name}>{u.name}</option>
              {/each}
            </select>
          </div>

          <div class="flex flex-col gap-1.5">
            <span class="text-xs font-medium text-neutral-400">Payment Source</span>
            <div class="inline-flex rounded-xl bg-neutral-950 p-1 border border-neutral-800 h-[42px] items-center">
              <button
                type="button"
                on:click={() => (editForm.is_joint = false)}
                class="px-4 py-1.5 rounded-lg text-xs font-semibold transition-colors cursor-pointer {!editForm.is_joint ? 'bg-indigo-600 text-white' : 'text-neutral-400'}"
              >
                Personal
              </button>
              <button
                type="button"
                on:click={() => (editForm.is_joint = true)}
                class="px-4 py-1.5 rounded-lg text-xs font-semibold transition-colors cursor-pointer {editForm.is_joint ? 'bg-indigo-600 text-white' : 'text-neutral-400'}"
              >
                🏦 Joint
              </button>
            </div>
          </div>
        </div>

        <!-- Active Toggle -->
        <div class="flex items-center gap-2 pt-1">
          <input
            type="checkbox"
            id="edit-rec-active"
            bind:checked={editForm.is_active}
            class="w-4 h-4 rounded border-neutral-700 bg-neutral-800 text-indigo-600 focus:ring-indigo-500"
          />
          <label for="edit-rec-active" class="text-xs text-neutral-300 font-medium">
            Active schedule (participates in automatic expense generation)
          </label>
        </div>

        <div class="flex justify-end gap-3 pt-3 border-t border-neutral-800">
          <button
            type="button"
            on:click={cancelEdit}
            class="px-4 py-2 rounded-xl text-xs font-semibold text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800 transition-colors cursor-pointer"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={editSaving}
            class="btn-primary cursor-pointer"
          >
            {editSaving ? 'Saving…' : 'Save Changes'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}
