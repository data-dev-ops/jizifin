<script>
  import { onMount, onDestroy } from 'svelte';
  import RealtimeChart from './lib/RealtimeChart.svelte';
  import ExpenseForm from './lib/ExpenseForm.svelte';
  import ExpenseList from './lib/ExpenseList.svelte';
  import SplitManager from './lib/SplitManager.svelte';
  import AnalyticsSummary from './lib/AnalyticsSummary.svelte';
  import IncomeChart from './lib/IncomeChart.svelte';
  import PaybackVisual from './lib/PaybackVisual.svelte';
  import QueryConsole from './lib/QueryConsole.svelte';
  import ProjectsTab from './lib/ProjectsTab.svelte';
  import RecurringManager from './lib/RecurringManager.svelte';
  import BudgetManager from './lib/BudgetManager.svelte';
  import UserManager from './lib/UserManager.svelte';
  import { fetchAllData, fetchAnalytics, fetchIncomeByPerson, fetchPaybacks, fetchBudgetAnalytics } from './lib/api.js';
  import { selectedMonth, projects, settlements, users } from './lib/stores.js';

  let activeTab = 'dashboard';
  let loading = true;
  let error = null;

  // Sidebar collapsed by default (especially for mobile)
  let sidebarOpen = false;

  const tabs = [
    { id: 'dashboard', label: 'Dashboard',  icon: '▤' },
    { id: 'expenses',  label: 'Expenses',   icon: '₂' },
    { id: 'splits',    label: 'Splits',     icon: '⊗' },
    { id: 'projects',  label: 'Projects',   icon: '▰' },
    { id: 'recurring', label: 'Recurring',  icon: '↻' },
    { id: 'query',     label: 'Query',      icon: '⌗' },
    { id: 'settings',  label: 'Settings',   icon: '⚙' },
  ];

  let unsubMonth;
  let budgetStatus = [];
  onMount(async () => {
    // On desktop (≥768px), show sidebar by default
    if (window.innerWidth >= 768) sidebarOpen = true;

    try {
      await fetchAllData($selectedMonth);
      try { budgetStatus = await fetchBudgetAnalytics($selectedMonth); } catch {}
    } catch (e) {
      error = 'Could not connect to the backend. Make sure the API is running on port 8000.';
    } finally {
      loading = false;
    }
    let skipFirst = true;
    unsubMonth = selectedMonth.subscribe((month) => {
      if (skipFirst) { skipFirst = false; return; }
      Promise.all([
        fetchAnalytics(month),
        fetchIncomeByPerson(month),
        fetchPaybacks(month),
        fetchBudgetAnalytics(month).then((rows) => { budgetStatus = rows; }),
      ]);
    });
  });

  onDestroy(() => { if (unsubMonth) unsubMonth(); });

  $: monthLabel = (() => {
    const [y, m] = $selectedMonth.split('-');
    return new Date(Number(y), Number(m) - 1, 1).toLocaleDateString('en-GB', { month: 'long', year: 'numeric' });
  })();

  function shiftMonth(delta) {
    const [y, m] = $selectedMonth.split('-').map(Number);
    const d = new Date(y, m - 1 + delta, 1);
    selectedMonth.set(
      `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}`
    );
  }

  function selectTab(id) {
    activeTab = id;
    // Auto-close sidebar on mobile after navigation
    if (window.innerWidth < 768) sidebarOpen = false;
  }
</script>

<div class="flex h-screen bg-neutral-950 text-white font-inter overflow-hidden relative">

  <!-- ── Mobile overlay backdrop ──────────────────────────────────────────── -->
  {#if sidebarOpen}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div
      class="fixed inset-0 bg-black/60 backdrop-blur-sm z-20 md:hidden"
      on:click={() => (sidebarOpen = false)}
    ></div>
  {/if}

  <!-- ── Sidebar ─────────────────────────────────────────────────────────── -->
  <aside
    class="
      fixed md:relative z-30 md:z-auto
      h-full flex-none flex flex-col
      bg-neutral-900 border-r border-neutral-800
      transition-all duration-300 ease-in-out
      {sidebarOpen ? 'w-60 translate-x-0' : 'w-0 md:w-60 -translate-x-full md:translate-x-0'}
      overflow-hidden
    "
  >
    <!-- Logo -->
    <div class="px-5 py-7 flex items-center gap-3 min-w-[240px]">
      <div class="w-9 h-9 rounded-xl bg-gradient-to-br from-indigo-500 to-violet-600 flex items-center justify-center text-base font-bold shadow-lg shadow-indigo-900/40 flex-none">
        €
      </div>
      <div>
        <p class="text-sm font-semibold leading-none">FinanceTracker</p>
        <p class="text-[11px] text-neutral-500 mt-0.5">
          {$users.filter(u => u.is_active).map(u => u.name).join(' & ') || 'Household'}
        </p>
      </div>
    </div>

    <!-- Month selector -->
    <div class="px-3 mb-4 min-w-[240px]">
      <div class="flex items-center justify-between bg-neutral-800 rounded-xl px-2 py-1.5">
        <button
          id="month-prev"
          on:click={() => shiftMonth(-1)}
          class="w-7 h-7 flex items-center justify-center rounded-lg text-neutral-400
                 hover:text-white hover:bg-neutral-700 transition-colors text-sm"
          aria-label="Previous month"
        >‹</button>
        <span class="text-xs font-semibold text-neutral-200 tabular-nums select-none">{monthLabel}</span>
        <button
          id="month-next"
          on:click={() => shiftMonth(1)}
          class="w-7 h-7 flex items-center justify-center rounded-lg text-neutral-400
                 hover:text-white hover:bg-neutral-700 transition-colors text-sm"
          aria-label="Next month"
        >›</button>
      </div>
    </div>

    <!-- Nav -->
    <nav class="flex-1 px-3 space-y-0.5 min-w-[240px]">
      {#each tabs as tab}
        <button
          id="nav-{tab.id}"
          on:click={() => selectTab(tab.id)}
          class="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm transition-all duration-150
                 {activeTab === tab.id
                   ? 'bg-indigo-600 text-white shadow-sm shadow-indigo-900/50'
                   : 'text-neutral-400 hover:text-neutral-100 hover:bg-neutral-800'}"
        >
          <span class="text-base leading-none">{tab.icon}</span>
          <span class="font-medium">{tab.label}</span>
        </button>
      {/each}
    </nav>

    <!-- Footer: dynamic active-user avatars -->
    <div class="px-5 py-5 border-t border-neutral-800 min-w-[240px]">
      <button
        on:click={() => selectTab('settings')}
        class="flex items-center gap-3 w-full text-left hover:opacity-80 transition-opacity"
      >
        <div class="flex -space-x-1.5">
          {#each $users.filter(u => u.is_active).slice(0, 4) as u (u.name)}
            <div
              class="w-7 h-7 rounded-full border-2 border-neutral-900 flex items-center justify-center text-[10px] font-bold flex-none"
              style="background-color: {u.color}"
            >{u.name.charAt(0).toUpperCase()}</div>
          {/each}
        </div>
        <div>
          <p class="text-xs font-medium text-neutral-200">
            {$users.filter(u => u.is_active).map(u => u.name).join(' & ') || 'Household'}
          </p>
          <p class="text-[10px] text-neutral-500">Shared finances · manage →</p>
        </div>
      </button>
    </div>
  </aside>

  <!-- ── Main content ──────────────────────────────────────────────────────── -->
  <main class="flex-1 overflow-y-auto bg-neutral-950 min-w-0">

    <!-- ── Top bar (always visible, contains hamburger) ───────────────────── -->
    <div class="sticky top-0 z-10 bg-neutral-950/90 backdrop-blur-sm border-b border-neutral-800/60 px-4 py-3 flex items-center gap-3">
      <button
        id="sidebar-toggle"
        on:click={() => (sidebarOpen = !sidebarOpen)}
        aria-label={sidebarOpen ? 'Close menu' : 'Open menu'}
        class="w-9 h-9 flex flex-col items-center justify-center gap-1.5 rounded-lg
               text-neutral-400 hover:text-white hover:bg-neutral-800
               transition-all duration-150 flex-none"
      >
        <!-- Animated hamburger / X -->
        <span
          class="block h-0.5 bg-current rounded-full transition-all duration-200 origin-center"
          style="width: {sidebarOpen ? '18px' : '18px'}; transform: {sidebarOpen ? 'translateY(4px) rotate(45deg)' : 'none'}"
        ></span>
        <span
          class="block h-0.5 bg-current rounded-full transition-all duration-200"
          style="width: 14px; opacity: {sidebarOpen ? 0 : 1}"
        ></span>
        <span
          class="block h-0.5 bg-current rounded-full transition-all duration-200 origin-center"
          style="width: {sidebarOpen ? '18px' : '18px'}; transform: {sidebarOpen ? 'translateY(-4px) rotate(-45deg)' : 'none'}"
        ></span>
      </button>

      <span class="text-sm font-semibold text-neutral-200 capitalize">{activeTab}</span>

      <span class="ml-auto text-xs text-neutral-500 tabular-nums">{monthLabel}</span>
    </div>

    {#if loading}
      <div class="flex items-center justify-center" style="height: calc(100vh - 57px)">
        <div class="flex flex-col items-center gap-4">
          <div class="w-10 h-10 rounded-full border-2 border-indigo-500 border-t-transparent animate-spin"></div>
          <p class="text-neutral-400 text-sm">Loading your finances…</p>
        </div>
      </div>

    {:else if error}
      <div class="flex items-center justify-center p-8" style="height: calc(100vh - 57px)">
        <div class="bg-red-950/60 border border-red-800 rounded-2xl p-6 max-w-md text-center">
          <p class="text-red-300 font-semibold mb-2">Connection error</p>
          <p class="text-red-400 text-sm">{error}</p>
        </div>
      </div>

    {:else if activeTab === 'dashboard'}
      <div class="p-4 sm:p-6 md:p-8 max-w-7xl mx-auto">
        <header class="mb-6 md:mb-8">
          <h1 class="text-xl sm:text-2xl font-bold text-white">Dashboard</h1>
          <p class="text-neutral-400 text-sm mt-1">Your financial overview for {monthLabel}</p>
        </header>

        <AnalyticsSummary />

        <div class="mt-6">
          <PaybackVisual />
        </div>

        <!-- Budget Health Widget -->
        {#if budgetStatus.length > 0}
          <div class="mt-6 bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6">
            <div class="flex items-center justify-between mb-4">
              <div>
                <h2 class="text-sm font-semibold text-neutral-200">Budget Health</h2>
                <p class="text-xs text-neutral-500 mt-0.5">{monthLabel}</p>
              </div>
              <button id="goto-budgets" on:click={() => selectTab('recurring')}
                class="text-xs text-indigo-400 hover:text-indigo-300 transition-colors">Manage →</button>
            </div>
            <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-3">
              {#each budgetStatus.filter(r => r.limit_cents > 0) as row}
                {@const color = row.pct_used >= 90 ? 'red' : row.pct_used >= 70 ? 'yellow' : 'green'}
                {@const barColor = color === 'red' ? 'bg-red-500' : color === 'yellow' ? 'bg-yellow-400' : 'bg-emerald-500'}
                {@const textColor = color === 'red' ? 'text-red-400' : color === 'yellow' ? 'text-yellow-400' : 'text-emerald-400'}
                <div class="bg-neutral-950 border border-neutral-800 rounded-xl p-3">
                  <p class="text-[11px] text-neutral-500 font-medium uppercase truncate">{row.category}</p>
                  <p class="text-xs font-semibold text-neutral-200 mt-0.5 tabular-nums">
                    €{(row.actual_cents/100).toFixed(0)} <span class="text-neutral-600 font-normal">/ €{(row.limit_cents/100).toFixed(0)}</span>
                  </p>
                  <div class="mt-1.5 h-1 bg-neutral-800 rounded-full overflow-hidden">
                    <div class="h-full rounded-full {barColor} transition-all" style="width:{Math.min(row.pct_used,100)}%"></div>
                  </div>
                  <p class="text-[10px] {textColor} font-semibold mt-0.5 text-right">{row.pct_used.toFixed(0)}%</p>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <div class="mt-6 bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6">
          <div class="flex items-center justify-between mb-5">
            <div>
              <h2 class="text-sm font-semibold text-neutral-200">Expense Timeline</h2>
              <p class="text-xs text-neutral-500 mt-0.5">Live — updates as expenses are logged</p>
            </div>
            <span class="flex items-center gap-1.5 text-xs text-emerald-400">
              <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
              Live
            </span>
          </div>
          <RealtimeChart />
        </div>

        <div class="mt-6 bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6">
          <div class="flex items-center justify-between mb-5">
            <div>
              <h2 class="text-sm font-semibold text-neutral-200">Income by Person</h2>
              <p class="text-xs text-neutral-500 mt-0.5">{monthLabel} — carry-forwards shown where no income recorded</p>
            </div>
          </div>
          <IncomeChart />
        </div>

        {#if $projects.length > 0}
          <div class="mt-6 bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6">
            <div class="flex items-center justify-between mb-5">
              <div>
                <h2 class="text-sm font-semibold text-neutral-200">Savings Projects</h2>
                <p class="text-xs text-neutral-500 mt-0.5">Progress overview — go to Projects tab for full details</p>
              </div>
              <button
                id="goto-projects"
                on:click={() => selectTab('projects')}
                class="text-xs text-indigo-400 hover:text-indigo-300 transition-colors"
              >View all →</button>
            </div>
            <div class="space-y-4">
              {#each $projects as project (project.id)}
                {@const progress = Math.min(100, Math.round((project.total_spent_cents / project.target_cents) * 100))}
                {@const isComplete = project.total_spent_cents >= project.target_cents}
                <div class="flex items-center gap-4">
                  <div class="flex-1 min-w-0">
                    <div class="flex justify-between items-baseline mb-1">
                      <span class="text-xs font-medium text-neutral-200 truncate">{project.name}</span>
                      <span class="text-xs tabular-nums {isComplete ? 'text-emerald-400' : 'text-neutral-400'} ml-2 flex-none">{progress}%</span>
                    </div>
                    <div class="w-full h-1.5 bg-neutral-800 rounded-full overflow-hidden">
                      <div
                        class="h-full rounded-full bg-gradient-to-r {isComplete ? 'from-emerald-500 to-emerald-400' : progress >= 60 ? 'from-indigo-500 to-violet-500' : progress >= 30 ? 'from-sky-600 to-indigo-500' : 'from-sky-700 to-sky-500'}"
                        style="width: {progress}%"
                      ></div>
                    </div>
                  </div>
                  <div class="text-right flex-none">
                    <p class="text-xs font-semibold text-neutral-100">
                      €{(project.total_spent_cents / 100).toFixed(0)}
                      <span class="text-neutral-600 font-normal">/ €{(project.target_cents / 100).toFixed(0)}</span>
                    </p>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>

    {:else if activeTab === 'expenses'}
      <div class="p-4 sm:p-6 md:p-8 max-w-7xl mx-auto">
        <header class="mb-6 md:mb-8">
          <h1 class="text-xl sm:text-2xl font-bold text-white">Expenses</h1>
          <p class="text-neutral-400 text-sm mt-1">Log a new expense or review {monthLabel}'s history</p>
        </header>

        <div class="grid grid-cols-1 xl:grid-cols-5 gap-6">
          <div class="xl:col-span-2 bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6">
            <h2 class="text-sm font-semibold text-neutral-300 mb-5">Add Expense</h2>
            <ExpenseForm />
          </div>

          <div class="xl:col-span-3 bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6">
            <h2 class="text-sm font-semibold text-neutral-300 mb-5">Expense Log</h2>
            <ExpenseList />
          </div>
        </div>
      </div>

    {:else if activeTab === 'splits'}
      <div class="p-4 sm:p-6 md:p-8 max-w-3xl mx-auto">
        <header class="mb-6 md:mb-8">
          <h1 class="text-xl sm:text-2xl font-bold text-white">Split Configuration</h1>
          <p class="text-neutral-400 text-sm mt-1">Adjust how shared costs are divided between household members</p>
        </header>

        <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6">
          <SplitManager />
        </div>
      </div>

    {:else if activeTab === 'projects'}
      <div class="p-4 sm:p-6 md:p-8 max-w-7xl mx-auto">
        <header class="mb-6 md:mb-8">
          <h1 class="text-xl sm:text-2xl font-bold text-white">Projects</h1>
          <p class="text-neutral-400 text-sm mt-1">Track savings goals and see estimated completion times</p>
        </header>
        <ProjectsTab />
      </div>

    {:else if activeTab === 'query'}
      <div class="p-4 sm:p-6 md:p-8 max-w-5xl mx-auto">
        <header class="mb-6 md:mb-8">
          <h1 class="text-xl sm:text-2xl font-bold text-white">Query Console</h1>
          <p class="text-neutral-400 text-sm mt-1">Run raw SQL against the SQLite database — results capped at 50 rows</p>
        </header>
        <QueryConsole />
      </div>

    {:else if activeTab === 'settings'}
      <div class="p-4 sm:p-6 md:p-8 max-w-3xl mx-auto">
        <header class="mb-6 md:mb-8">
          <h1 class="text-xl sm:text-2xl font-bold text-white">Household Members</h1>
          <p class="text-neutral-400 text-sm mt-1">Add members, set active status, and customise avatar colours</p>
        </header>
        <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6">
          <UserManager />
        </div>
      </div>

    {:else if activeTab === 'recurring'}
      <div class="p-4 sm:p-6 md:p-8 max-w-5xl mx-auto">
        <header class="mb-6 md:mb-8">
          <h1 class="text-xl sm:text-2xl font-bold text-white">Recurring & Budgets</h1>
          <p class="text-neutral-400 text-sm mt-1">Automate monthly expenses and set spending limits</p>
        </header>
        <div class="space-y-10">
          <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6">
            <RecurringManager />
          </div>
          <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6">
            <BudgetManager />
          </div>
        </div>
      </div>
    {/if}

  </main>
</div>
