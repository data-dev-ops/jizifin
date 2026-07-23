<script>
  import { onMount, onDestroy } from 'svelte';
  import RealtimeChart from './lib/RealtimeChart.svelte';
  import ExpenseForm from './lib/ExpenseForm.svelte';
  import ExpenseList from './lib/ExpenseList.svelte';
  import SplitManager from './lib/SplitManager.svelte';
  import AnalyticsSummary from './lib/AnalyticsSummary.svelte';
  import IncomeChart from './lib/IncomeChart.svelte';
  import IncomeTab from './lib/IncomeTab.svelte';
  import PaybackVisual from './lib/PaybackVisual.svelte';
  import QueryConsole from './lib/QueryConsole.svelte';
  import TagsTab from './lib/TagsTab.svelte';
  import ProjectsTab from './lib/ProjectsTab.svelte';
  import RecurringManager from './lib/RecurringManager.svelte';
  import BudgetManager from './lib/BudgetManager.svelte';
  import UserManager from './lib/UserManager.svelte';
  import Login from './lib/Login.svelte';
  import { fetchAllData, fetchAnalytics, fetchIncomeByPerson, fetchPaybacks, fetchBudgetAnalytics, exportDatabase, fetchIncome, fetchIncomeCategories } from './lib/api.js';
  import { selectedMonth, projects, settlements, users, mobileSplitsEditable, mobileTabVisibility, mobileAutoCloseMenu, mobileCompactView, mobileLargeTouchTargets, defaultPayer, defaultCategory, showQueryTab, currencySymbol, splits, authSalt, tags, splitInputMode, paybackDisplayMode, chartStyle, incomeEntries, incomeCategories } from './lib/stores.js';

  let activeTab = 'dashboard';
  let loading = false; // Handled after salt is entered
  let error = null;
  let exporting = false;
  let exportError = '';

  // Sidebar collapsed by default (especially for mobile)
  let sidebarOpen = false;
  let isMobile = false;
  let tabToggleWarning = '';

  async function handleExport() {
    exporting = true;
    exportError = '';
    try {
      await exportDatabase($authSalt);
    } catch (e) {
      exportError = e.message;
    } finally {
      exporting = false;
    }
  }

  const tabs = [
    {
      id: 'dashboard', label: 'Dashboard',
      icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75"><rect x="3" y="3" width="7" height="7" rx="1" stroke-linecap="round" stroke-linejoin="round"/><rect x="14" y="3" width="7" height="7" rx="1" stroke-linecap="round" stroke-linejoin="round"/><rect x="3" y="14" width="7" height="7" rx="1" stroke-linecap="round" stroke-linejoin="round"/><rect x="14" y="14" width="7" height="7" rx="1" stroke-linecap="round" stroke-linejoin="round"/></svg>`,
    },
    {
      id: 'expenses', label: 'Expenses',
      icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75"><path stroke-linecap="round" stroke-linejoin="round" d="M9 14l2 2 4-4M7.5 3.75A1.5 1.5 0 006 5.25v13.5A1.5 1.5 0 007.5 20.25h9A1.5 1.5 0 0018 18.75V5.25A1.5 1.5 0 0016.5 3.75H7.5z"/></svg>`,
    },
    {
      id: 'income', label: 'Income',
      icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75"><path stroke-linecap="round" stroke-linejoin="round" d="M12 6v12m-3-2.818l.879.659c1.171.879 3.07.879 4.242 0 1.172-.879 1.172-2.303 0-3.182C13.536 12.219 12.768 12 12 12c-.725 0-1.45-.22-2.003-.659-1.106-.879-1.106-2.303 0-3.182s2.9-.879 4.006 0l.415.33M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>`,
    },
    {
      id: 'splits', label: 'Splits',
      icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75"><path stroke-linecap="round" stroke-linejoin="round" d="M12 3v18M3 12h18"/><circle cx="12" cy="12" r="9" stroke-linecap="round"/></svg>`,
    },
    {
      id: 'projects', label: 'Projects',
      icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75"><path stroke-linecap="round" stroke-linejoin="round" d="M3 13.125C3 12.504 3.504 12 4.125 12h2.25c.621 0 1.125.504 1.125 1.125v6.75C7.5 20.496 6.996 21 6.375 21h-2.25A1.125 1.125 0 013 19.875v-6.75zM9.75 8.625c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125v11.25c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 01-1.125-1.125V8.625zM16.5 4.125c0-.621.504-1.125 1.125-1.125h2.25C20.496 3 21 3.504 21 4.125v15.75c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 01-1.125-1.125V4.125z"/></svg>`,
    },
    {
      id: 'tags', label: 'Tags',
      icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75"><path stroke-linecap="round" stroke-linejoin="round" d="M9.568 3H5.25A2.25 2.25 0 003 5.25v4.318c0 .597.237 1.17.659 1.591l9.581 9.581c.699.699 1.78.872 2.607.33a18.095 18.095 0 005.223-5.223c.542-.827.369-1.908-.33-2.607L11.16 3.66A2.25 2.25 0 009.568 3z"/><path stroke-linecap="round" stroke-linejoin="round" d="M6 6h.008v.008H6V6z"/></svg>`,
    },
    {
      id: 'recurring', label: 'Recurring',
      icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75"><path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99"/></svg>`,
    },
    {
      id: 'query', label: 'Query',
      icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75"><path stroke-linecap="round" stroke-linejoin="round" d="M17.25 6.75L22.5 12l-5.25 5.25m-10.5 0L1.5 12l5.25-5.25m7.5-3l-4.5 16.5"/></svg>`,
    },
    {
      id: 'settings', label: 'Settings',
      icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75"><path stroke-linecap="round" stroke-linejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.324.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 011.37.49l1.296 2.247a1.125 1.125 0 01-.26 1.431l-1.003.827c-.293.24-.438.613-.431.992a6.759 6.759 0 010 .255c-.007.378.138.75.43.99l1.005.828c.424.35.534.954.26 1.43l-1.298 2.247a1.125 1.125 0 01-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.57 6.57 0 01-.22.128c-.331.183-.581.495-.644.869l-.213 1.28c-.09.543-.56.941-1.11.941h-2.594c-.55 0-1.02-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 01-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 01-1.369-.49l-1.297-2.247a1.125 1.125 0 01.26-1.431l1.004-.827c.292-.24.437-.613.43-.992a6.932 6.932 0 010-.255c.007-.378-.138-.75-.43-.99l-1.004-.828a1.125 1.125 0 01-.26-1.43l1.297-2.247a1.125 1.125 0 011.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.281z"/><path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/></svg>`,
    },
  ];

  $: visibleTabs = tabs.filter(t => {
    if (t.id === 'query' && !$showQueryTab) return false;
    if (isMobile && !$mobileTabVisibility[t.id]) return false;
    return true;
  });

  $: if (isMobile && visibleTabs.length > 0 && !visibleTabs.some(t => t.id === activeTab)) {
    activeTab = visibleTabs.some(t => t.id === 'settings') ? 'settings' : visibleTabs[0].id;
  }

  let unsubMonth;
  let budgetStatus = [];
  let initialLoaded = false;

  onMount(async () => {
    const checkMobile = () => {
      isMobile = window.innerWidth < 768;
      if (window.innerWidth >= 768) sidebarOpen = true;
    };
    checkMobile();
    window.addEventListener('resize', checkMobile);

    return () => {
      window.removeEventListener('resize', checkMobile);
    };
  });

  function toggleMobileTab(tabId) {
    tabToggleWarning = '';
    if (tabId === 'settings') {
      tabToggleWarning = 'Settings tab is required and cannot be disabled on mobile.';
      return;
    }

    const currentVis = { ...$mobileTabVisibility };
    const currentState = !!currentVis[tabId];
    
    if (currentState) {
      // Turning off tabId: enforce at least 1 non-settings tab remains active
      const activeNonSettingsCount = tabs.filter(t => t.id !== 'settings' && currentVis[t.id]).length;
      if (activeNonSettingsCount <= 1) {
        tabToggleWarning = 'Settings and at least 1 additional tab must remain active on mobile.';
        return;
      }
    }

    mobileTabVisibility.update(v => ({
      ...v,
      [tabId]: !currentState
    }));
  }

  $: if ($authSalt && !initialLoaded) {
    initialLoaded = true;
    loading = true;
    fetchAllData($selectedMonth)
      .then(async () => {
        try { budgetStatus = await fetchBudgetAnalytics($selectedMonth); } catch {}
      })
      .catch((e) => {
        error = 'Could not connect to the backend. Make sure the API service is running.';
      })
      .finally(() => {
        loading = false;
      });

    let skipFirst = true;
    unsubMonth = selectedMonth.subscribe((month) => {
      if (skipFirst) { skipFirst = false; return; }
      Promise.all([
        fetchAnalytics(month),
        fetchIncomeByPerson(month),
        fetchPaybacks(month),
        fetchIncome(month),
        fetchBudgetAnalytics(month).then((rows) => { budgetStatus = rows; }),
      ]);
    });
  }

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
    // Auto-close sidebar on mobile after navigation if enabled
    if (isMobile && $mobileAutoCloseMenu) {
      sidebarOpen = false;
    }
  }
</script>

{#if !$authSalt}
  <Login />
{:else}
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
      <div class="w-9 h-9 rounded-xl bg-gradient-to-br from-indigo-500 to-violet-600 flex items-center justify-center text-base font-bold shadow-lg shadow-indigo-900/40 flex-none text-white">
        {$currencySymbol}
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
      {#each visibleTabs as tab}
        <button
          id="nav-{tab.id}"
          on:click={() => selectTab(tab.id)}
          class="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm transition-all duration-150
                 {activeTab === tab.id
                   ? 'bg-indigo-600 text-white shadow-sm shadow-indigo-900/50'
                   : 'text-neutral-400 hover:text-neutral-100 hover:bg-neutral-800'}"
        >
          <span class="flex-none leading-none">{@html tab.icon}</span>
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
                {@const isStanding = !row.budget_month || row.budget_month === 'ALL'}
                <div class="bg-neutral-950 border border-neutral-800 rounded-xl p-3">
                  <div class="flex items-start justify-between gap-1 mb-0.5">
                    <p class="text-[11px] text-neutral-500 font-medium uppercase truncate">{row.category}</p>
                    {#if isStanding}
                      <span class="flex-none text-[9px] font-semibold uppercase tracking-wide text-neutral-600 bg-neutral-800 rounded px-1 py-0.5 leading-none">standing</span>
                    {:else}
                      <span class="flex-none text-[9px] font-semibold uppercase tracking-wide text-indigo-500 bg-indigo-950/60 rounded px-1 py-0.5 leading-none">this month</span>
                    {/if}
                  </div>
                  <p class="text-xs font-semibold text-neutral-200 mt-0.5 tabular-nums">
                    {$currencySymbol}{(row.actual_cents/100).toFixed(0)} <span class="text-neutral-600 font-normal">/ {$currencySymbol}{(row.limit_cents/100).toFixed(0)}</span>
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
                      {$currencySymbol}{(project.total_spent_cents / 100).toFixed(0)}
                      <span class="text-neutral-600 font-normal">/ {$currencySymbol}{(project.target_cents / 100).toFixed(0)}</span>
                    </p>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Tags summary widget -->
        {#if $tags.length > 0}
          <div class="mt-6 bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6">
            <div class="flex items-center justify-between mb-4">
              <div>
                <h2 class="text-sm font-semibold text-neutral-200">Tags</h2>
                <p class="text-xs text-neutral-500 mt-0.5">All-time accumulation per event tag</p>
              </div>
              <button
                id="goto-tags"
                on:click={() => selectTab('tags')}
                class="text-xs text-amber-400 hover:text-amber-300 transition-colors"
              >View all →</button>
            </div>
            <div class="flex flex-wrap gap-2">
              {#each $tags as tag (tag.id)}
                <button
                  id="dashboard-tag-chip-{tag.id}"
                  on:click={() => selectTab('tags')}
                  class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full border text-xs font-medium
                         transition-all duration-150 hover:brightness-110 active:scale-[0.97]"
                  style="background-color: {tag.color}15; color: {tag.color}; border-color: {tag.color}35;"
                >
                  <span class="w-2 h-2 rounded-full flex-none" style="background-color: {tag.color}"></span>
                  {tag.name}
                  <span class="text-[10px] opacity-70">{$currencySymbol}{(tag.total_amount ?? 0).toLocaleString('en-GB', {minimumFractionDigits: 2, maximumFractionDigits: 2})}</span>
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>

    {:else if activeTab === 'income'}
      <div class="p-4 sm:p-6 md:p-8 max-w-7xl mx-auto">
        <IncomeTab />
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

    {:else if activeTab === 'tags'}
      <div class="p-4 sm:p-6 md:p-8 max-w-7xl mx-auto">
        <header class="mb-6 md:mb-8">
          <h1 class="text-xl sm:text-2xl font-bold text-white">Tags</h1>
          <p class="text-neutral-400 text-sm mt-1">Track open-ended events — vacations, repairs, and more — across all months</p>
        </header>
        <TagsTab />
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
          <h1 class="text-xl sm:text-2xl font-bold text-white">Settings</h1>
          <p class="text-neutral-400 text-sm mt-1">Household members and display preferences</p>
        </header>

        <!-- Mobile Preferences -->
        <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6 mb-6">
          <div class="flex items-center justify-between mb-4">
            <div>
              <p class="text-xs font-semibold text-neutral-400 uppercase tracking-wider">Mobile Preferences</p>
              <p class="text-xs text-neutral-500 mt-1">Configure tab visibility and behavior options for mobile screens.</p>
            </div>
          </div>

          {#if tabToggleWarning}
            <div class="mb-4 p-3 bg-amber-950/60 border border-amber-800/80 rounded-xl text-amber-300 text-xs flex items-center justify-between gap-2 animate-fadeIn">
              <div class="flex items-center gap-2">
                <svg class="w-4 h-4 text-amber-400 flex-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/></svg>
                <span>{tabToggleWarning}</span>
              </div>
              <button on:click={() => (tabToggleWarning = '')} class="text-amber-400 hover:text-amber-200 text-sm font-bold flex-none px-1">×</button>
            </div>
          {/if}

          <!-- Sub-section: Mobile Tab Visibility -->
          <div class="mb-6 border-b border-neutral-800 pb-5">
            <p class="text-xs font-semibold text-neutral-300 mb-1">Tab Visibility Toggles</p>
            <p class="text-xs text-neutral-500 mb-4">Toggle navigation tabs on mobile screens (Settings + 1 minimum required).</p>
            
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
              {#each tabs as tab (tab.id)}
                {@const isRequired = tab.id === 'settings'}
                {@const isActive = !!$mobileTabVisibility[tab.id]}
                <div class="flex items-center justify-between p-3 rounded-xl bg-neutral-950/80 border border-neutral-800/80 transition-colors">
                  <div class="flex items-center gap-2.5 min-w-0 pr-2">
                    <span class="text-neutral-400 flex-none">{@html tab.icon}</span>
                    <div class="min-w-0">
                      <p class="text-sm font-medium text-neutral-200 truncate flex items-center gap-1.5">
                        {tab.label}
                        {#if isRequired}
                          <span class="text-[9px] uppercase font-bold tracking-wider px-1.5 py-0.5 rounded bg-indigo-950 text-indigo-400 border border-indigo-800/60">Required</span>
                        {/if}
                      </p>
                      <p class="text-[11px] text-neutral-500 truncate">
                        {isRequired ? 'Always active' : isActive ? 'Visible on mobile' : 'Hidden on mobile'}
                      </p>
                    </div>
                  </div>

                  <!-- Switch button -->
                  <button
                    id="toggle-mobile-tab-{tab.id}"
                    role="switch"
                    aria-checked={isActive}
                    disabled={isRequired}
                    on:click={() => toggleMobileTab(tab.id)}
                    class="relative inline-flex h-6 w-11 flex-none cursor-pointer rounded-full border-2 border-transparent
                           transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:ring-offset-neutral-900
                           {isRequired ? 'opacity-60 cursor-not-allowed bg-indigo-600' : isActive ? 'bg-indigo-600' : 'bg-neutral-700'}"
                  >
                    <span
                      class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow
                             transition duration-200 ease-in-out
                             {isActive ? 'translate-x-5' : 'translate-x-0'}"
                    ></span>
                  </button>
                </div>
              {/each}
            </div>
          </div>

          <!-- Additional Mobile Preferences -->
          <div class="space-y-4">
            <p class="text-xs font-semibold text-neutral-300 mb-1">Mobile Behavior Preferences</p>

            <!-- Splits editing on mobile -->
            <div class="flex items-center justify-between gap-4 border-b border-neutral-800/60 pb-3.5">
              <div class="flex items-center gap-3">
                <span class="text-lg leading-none select-none" aria-hidden="true">
                  {$mobileSplitsEditable ? '🔓' : '🔒'}
                </span>
                <div>
                  <p class="text-sm font-medium text-neutral-200">Splits editing on mobile</p>
                  <p class="text-xs text-neutral-500 mt-0.5">
                    {$mobileSplitsEditable
                      ? 'Inputs and save buttons are visible on small screens.'
                      : 'Splits are read-only on mobile — tap to unlock.'}
                  </p>
                </div>
              </div>
              <button
                id="toggle-mobile-splits"
                role="switch"
                aria-checked={$mobileSplitsEditable}
                on:click={() => mobileSplitsEditable.update(v => !v)}
                class="relative inline-flex h-6 w-11 flex-none cursor-pointer rounded-full border-2 border-transparent
                       transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:ring-offset-neutral-900
                       {$mobileSplitsEditable ? 'bg-indigo-600' : 'bg-neutral-700'}"
              >
                <span
                  class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow
                         transition duration-200 ease-in-out
                         {$mobileSplitsEditable ? 'translate-x-5' : 'translate-x-0'}"
                ></span>
              </button>
            </div>

            <!-- Auto-close navigation menu -->
            <div class="flex items-center justify-between gap-4 border-b border-neutral-800/60 pb-3.5">
              <div>
                <p class="text-sm font-medium text-neutral-200">Auto-close navigation menu</p>
                <p class="text-xs text-neutral-500 mt-0.5">Automatically dismiss sidebar drawer after selecting a tab on mobile.</p>
              </div>
              <button
                id="toggle-mobile-autoclose"
                role="switch"
                aria-checked={$mobileAutoCloseMenu}
                on:click={() => mobileAutoCloseMenu.update(v => !v)}
                class="relative inline-flex h-6 w-11 flex-none cursor-pointer rounded-full border-2 border-transparent
                       transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:ring-offset-neutral-900
                       {$mobileAutoCloseMenu ? 'bg-indigo-600' : 'bg-neutral-700'}"
              >
                <span
                  class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow
                         transition duration-200 ease-in-out
                         {$mobileAutoCloseMenu ? 'translate-x-5' : 'translate-x-0'}"
                ></span>
              </button>
            </div>

            <!-- Compact mobile layout -->
            <div class="flex items-center justify-between gap-4 border-b border-neutral-800/60 pb-3.5">
              <div>
                <p class="text-sm font-medium text-neutral-200">Compact mobile layout</p>
                <p class="text-xs text-neutral-500 mt-0.5">Use tighter padding and denser margins on mobile viewports.</p>
              </div>
              <button
                id="toggle-mobile-compact"
                role="switch"
                aria-checked={$mobileCompactView}
                on:click={() => mobileCompactView.update(v => !v)}
                class="relative inline-flex h-6 w-11 flex-none cursor-pointer rounded-full border-2 border-transparent
                       transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:ring-offset-neutral-900
                       {$mobileCompactView ? 'bg-indigo-600' : 'bg-neutral-700'}"
              >
                <span
                  class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow
                         transition duration-200 ease-in-out
                         {$mobileCompactView ? 'translate-x-5' : 'translate-x-0'}"
                ></span>
              </button>
            </div>

            <!-- Touch-friendly large targets -->
            <div class="flex items-center justify-between gap-4">
              <div>
                <p class="text-sm font-medium text-neutral-200">Touch-friendly large targets</p>
                <p class="text-xs text-neutral-500 mt-0.5">Increase tap target height and spacing on touch screens.</p>
              </div>
              <button
                id="toggle-mobile-touch-targets"
                role="switch"
                aria-checked={$mobileLargeTouchTargets}
                on:click={() => mobileLargeTouchTargets.update(v => !v)}
                class="relative inline-flex h-6 w-11 flex-none cursor-pointer rounded-full border-2 border-transparent
                       transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:ring-offset-neutral-900
                       {$mobileLargeTouchTargets ? 'bg-indigo-600' : 'bg-neutral-700'}"
              >
                <span
                  class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow
                         transition duration-200 ease-in-out
                         {$mobileLargeTouchTargets ? 'translate-x-5' : 'translate-x-0'}"
                ></span>
              </button>
            </div>
          </div>
        </div>

        <!-- Preferences Section -->
        <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6 mb-6">
          <p class="text-xs font-semibold text-neutral-400 uppercase tracking-wider mb-4">Display & Form Preferences</p>
          
          <div class="space-y-4">
            <!-- Currency Symbol -->
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2 border-b border-neutral-800 pb-4">
              <div>
                <p class="text-sm font-medium text-neutral-200">Currency Symbol</p>
                <p class="text-xs text-neutral-500 mt-0.5">Preferred currency symbol used across the dashboard (e.g. €, $, £).</p>
              </div>
              <input
                id="setting-currency-symbol"
                type="text"
                maxlength="3"
                bind:value={$currencySymbol}
                class="w-full sm:w-24 bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-1.5 text-sm text-neutral-100 placeholder-neutral-600 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 transition-colors"
              />
            </div>

            <!-- Default Payer -->
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2 border-b border-neutral-800 pb-4">
              <div>
                <p class="text-sm font-medium text-neutral-200">Default Payer</p>
                <p class="text-xs text-neutral-500 mt-0.5">Pre-selected member when logging a new expense.</p>
              </div>
              <select
                id="setting-default-payer"
                bind:value={$defaultPayer}
                class="w-full sm:w-48 bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-1.5 text-sm text-neutral-100 focus:outline-none focus:border-indigo-500 transition-colors"
              >
                <option value="">— None (require choice) —</option>
                {#each $users.filter(u => u.is_active) as u}
                  <option value={u.name}>{u.name}</option>
                {/each}
              </select>
            </div>

            <!-- Default Category -->
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2 border-b border-neutral-800 pb-4">
              <div>
                <p class="text-sm font-medium text-neutral-200">Default Category</p>
                <p class="text-xs text-neutral-500 mt-0.5">Pre-selected category when logging a new expense.</p>
              </div>
              <select
                id="setting-default-category"
                bind:value={$defaultCategory}
                class="w-full sm:w-48 bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-1.5 text-sm text-neutral-100 focus:outline-none focus:border-indigo-500 transition-colors"
              >
                <option value="">— None (require choice) —</option>
                {#each $splits as split}
                  <option value={split.category}>{split.category}</option>
                {/each}
              </select>
            </div>

            <!-- Show SQL Console -->
            <div class="flex items-center justify-between gap-4">
              <div>
                <p class="text-sm font-medium text-neutral-200">Show SQL Query Console</p>
                <p class="text-xs text-neutral-500 mt-0.5">Display the raw SQL Query tab in the navigation sidebar.</p>
              </div>
              <!-- Toggle switch -->
              <button
                id="toggle-query-console"
                role="switch"
                aria-checked={$showQueryTab}
                on:click={() => showQueryTab.update(v => !v)}
                class="relative inline-flex h-6 w-11 flex-none cursor-pointer rounded-full border-2 border-transparent
                       transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:ring-offset-neutral-900
                       {$showQueryTab ? 'bg-indigo-600' : 'bg-neutral-700'}"
              >
                <span
                  class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow
                         transition duration-200 ease-in-out
                         {$showQueryTab ? 'translate-x-5' : 'translate-x-0'}"
                ></span>
              </button>
            </div>
          </div>
        </div>

        <!-- Visualization & Controls -->
        <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6 mb-6">
          <p class="text-xs font-semibold text-neutral-400 uppercase tracking-wider mb-4">Visualization & Controls</p>

          <div class="space-y-5">
            <!-- Split Input Mode -->
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-800 pb-5">
              <div>
                <p class="text-sm font-medium text-neutral-200">Split Input Mode</p>
                <p class="text-xs text-neutral-500 mt-0.5">Slider links two users to 100% — drag one, the other follows.</p>
              </div>
              <div class="flex bg-neutral-800 rounded-lg p-0.5 flex-none">
                <button
                  id="setting-split-inputs"
                  on:click={() => splitInputMode.set('inputs')}
                  class="px-3.5 py-1.5 rounded-md text-xs font-semibold transition-all duration-200
                         {$splitInputMode === 'inputs'
                           ? 'bg-indigo-600 text-white shadow-sm shadow-indigo-900/40'
                           : 'text-neutral-400 hover:text-neutral-200'}"
                >Inputs</button>
                <button
                  id="setting-split-slider"
                  on:click={() => splitInputMode.set('slider')}
                  class="px-3.5 py-1.5 rounded-md text-xs font-semibold transition-all duration-200
                         {$splitInputMode === 'slider'
                           ? 'bg-indigo-600 text-white shadow-sm shadow-indigo-900/40'
                           : 'text-neutral-400 hover:text-neutral-200'}"
                >Slider</button>
              </div>
            </div>

            <!-- Payback Display -->
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-800 pb-5">
              <div>
                <p class="text-sm font-medium text-neutral-200">Payback Display</p>
                <p class="text-xs text-neutral-500 mt-0.5">Cards show detail per person. Bars give a compact visual overview.</p>
              </div>
              <div class="flex bg-neutral-800 rounded-lg p-0.5 flex-none">
                <button
                  id="setting-payback-cards"
                  on:click={() => paybackDisplayMode.set('cards')}
                  class="px-3.5 py-1.5 rounded-md text-xs font-semibold transition-all duration-200
                         {$paybackDisplayMode === 'cards'
                           ? 'bg-indigo-600 text-white shadow-sm shadow-indigo-900/40'
                           : 'text-neutral-400 hover:text-neutral-200'}"
                >Cards</button>
                <button
                  id="setting-payback-bars"
                  on:click={() => paybackDisplayMode.set('bar')}
                  class="px-3.5 py-1.5 rounded-md text-xs font-semibold transition-all duration-200
                         {$paybackDisplayMode === 'bar'
                           ? 'bg-indigo-600 text-white shadow-sm shadow-indigo-900/40'
                           : 'text-neutral-400 hover:text-neutral-200'}"
                >Bars</button>
              </div>
            </div>

            <!-- Category Chart Style -->
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
              <div>
                <p class="text-sm font-medium text-neutral-200">Category Chart</p>
                <p class="text-xs text-neutral-500 mt-0.5">Switch between a doughnut and a horizontal bar chart on the dashboard.</p>
              </div>
              <div class="flex bg-neutral-800 rounded-lg p-0.5 flex-none">
                <button
                  id="setting-chart-doughnut"
                  on:click={() => chartStyle.set('doughnut')}
                  class="px-3.5 py-1.5 rounded-md text-xs font-semibold transition-all duration-200
                         {$chartStyle === 'doughnut'
                           ? 'bg-indigo-600 text-white shadow-sm shadow-indigo-900/40'
                           : 'text-neutral-400 hover:text-neutral-200'}"
                >Doughnut</button>
                <button
                  id="setting-chart-bar"
                  on:click={() => chartStyle.set('bar')}
                  class="px-3.5 py-1.5 rounded-md text-xs font-semibold transition-all duration-200
                         {$chartStyle === 'bar'
                           ? 'bg-indigo-600 text-white shadow-sm shadow-indigo-900/40'
                           : 'text-neutral-400 hover:text-neutral-200'}"
                >Bar Chart</button>
              </div>
            </div>
          </div>
        </div>

        <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6 mb-6">
          <p class="text-xs font-semibold text-neutral-400 uppercase tracking-wider mb-4">Household Members</p>
          <UserManager />
        </div>

        <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6">
          <p class="text-xs font-semibold text-neutral-400 uppercase tracking-wider mb-4">Database Management</p>
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
            <div>
              <p class="text-sm font-medium text-neutral-200">Export Decrypted Database</p>
              <p class="text-xs text-neutral-500 mt-0.5">Download a fully decrypted SQLite database file to browse your data in DBeaver.</p>
              {#if exportError}
                <p class="text-xs text-red-400 mt-2">{exportError}</p>
              {/if}
            </div>
            <button
              on:click={handleExport}
              disabled={exporting}
              class="w-full sm:w-auto px-4 py-2 bg-neutral-800 hover:bg-neutral-700 text-neutral-200 text-sm font-medium rounded-lg transition-colors border border-neutral-700 disabled:opacity-50 whitespace-nowrap"
            >
              {exporting ? 'Decrypting...' : 'Export .db File'}
            </button>
          </div>
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
{/if}
