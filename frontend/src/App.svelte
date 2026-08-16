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
  import JointAccountTab from './lib/JointAccountTab.svelte';
  import SettingsTab from './lib/SettingsTab.svelte';
  import { fetchAllData, fetchAnalytics, fetchIncomeByPerson, fetchPaybacks, fetchBudgetAnalytics, fetchIncome, fetchIncomeCategories, fetchRecurring } from './lib/api.js';
  import { selectedMonth, projects, settlements, users, mobileTabVisibility, mobileAutoCloseMenu, mobileCompactView, mobileLargeTouchTargets, currencySymbol, splits, authSalt, tags, jointAccountEnabled } from './lib/stores.js';

  let showJointPromptModal = false;

  let activeTab = 'dashboard';
  let loading = false; // Handled after salt is entered
  let error = null;

  // Sidebar collapsed by default (especially for mobile)
  let sidebarOpen = false;
  let isMobile = false;

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
      id: 'splits', label: 'Categories',
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
      id: 'joint', label: 'Joint Account',
      icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75"><path stroke-linecap="round" stroke-linejoin="round" d="M2.25 18.75a60.07 60.07 0 0115.797 2.101c.727.198 1.453-.342 1.453-1.096V18.75M3.75 4.5v.75A.75.75 0 013 6h-.75m0 0v-.375c0-.621.504-1.125 1.125-1.125H20.25M2.25 6v9m18-10.5v.75c0 .414.336.75.75.75h.75m-1.5-1.5h.375c.621 0 1.125.504 1.125 1.125v9.75c0 .621-.504 1.125-1.125 1.125h-.375m1.5-1.5H21a.75.75 0 00-.75.75v.75m0 0H3.75m0 0h-.375a1.125 1.125 0 01-1.125-1.125V15m1.5 1.5v-.75A.75.75 0 003 15h-.75M15 10.5a3 3 0 11-6 0 3 3 0 016 0zm3 0h.008v.008H18V10.5zm-12 0h.008v.008H6V10.5z"/></svg>`,
    },
    {
      id: 'settings', label: 'Settings',
      icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75"><path stroke-linecap="round" stroke-linejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.324.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 011.37.49l1.296 2.247a1.125 1.125 0 01-.26 1.431l-1.003.827c-.293.24-.438.613-.431.992a6.759 6.759 0 010 .255c-.007.378.138.75.43.99l1.005.828c.424.35.534.954.26 1.43l-1.298 2.247a1.125 1.125 0 01-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.57 6.57 0 01-.22.128c-.331.183-.581.495-.644.869l-.213 1.28c-.09.543-.56.941-1.11.941h-2.594c-.55 0-1.02-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 01-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 01-1.369-.49l-1.297-2.247a1.125 1.125 0 01.26-1.431l1.004-.827c.292-.24.437-.613.43-.992a6.932 6.932 0 010-.255c.007-.378-.138-.75-.43-.99l-1.004-.828a1.125 1.125 0 01-.26-1.43l1.297-2.247a1.125 1.125 0 011.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.281z"/><path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/></svg>`,
    },
  ];

  $: visibleTabs = tabs.filter(t => {
    if (t.id === 'joint' && !$jointAccountEnabled) return false;
    if (!$mobileTabVisibility[t.id]) return false;
    return true;
  });

  $: if (visibleTabs.length > 0 && !visibleTabs.some(t => t.id === activeTab)) {
    activeTab = visibleTabs.some(t => t.id === 'settings') ? 'settings' : visibleTabs[0].id;
  }

  let unsubMonth;
  let budgetStatus = [];
  let initialLoaded = false;

  onMount(async () => {
    const checkMobile = () => {
      isMobile = window.innerWidth < 768;
    };
    checkMobile();
    sidebarOpen = !isMobile;
    window.addEventListener('resize', checkMobile);

    return () => {
      window.removeEventListener('resize', checkMobile);
    };
  });

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
        fetchRecurring(month),
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
  <div class="flex h-screen bg-neutral-950 text-white font-inter overflow-hidden relative {$mobileCompactView ? 'compact-layout' : ''} {$mobileLargeTouchTargets ? 'large-touch-targets' : ''}">

  <!-- ── Mobile overlay backdrop ──────────────────────────────────────────── -->
  {#if sidebarOpen}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div
      class="fixed inset-0 bg-black/60 backdrop-blur-sm z-20 md:hidden"
      on:click={() => (sidebarOpen = false)}
    ></div>
  {/if}

  <aside
    class="
      fixed md:relative z-30 md:z-auto
      h-full flex-none flex flex-col
      bg-neutral-900 border-r border-neutral-800
      transition-all duration-300 ease-in-out
      {sidebarOpen ? 'w-60 translate-x-0' : 'w-0 -translate-x-full'}
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
      <div class="page-container">
        <header class="page-header">
          <div>
            <h1 class="page-title">Dashboard</h1>
            <p class="page-subtitle">Your household financial command center for {monthLabel}</p>
          </div>
          <div class="flex items-center gap-2 self-start sm:self-auto">
            <span class="badge-indigo">{monthLabel}</span>
          </div>
        </header>

        <AnalyticsSummary />

        <div class="mt-6">
          <PaybackVisual />
        </div>

        <!-- Budget Health Widget -->
        {#if budgetStatus.length > 0}
          <div class="card">
            <div class="flex items-center justify-between mb-4">
              <div>
                <h2 class="text-sm font-semibold text-neutral-200">Budget Health</h2>
                <p class="text-xs text-neutral-500 mt-0.5">{monthLabel} spending vs category limits</p>
              </div>
              <button id="goto-budgets" on:click={() => selectTab('recurring')}
                class="btn-ghost text-indigo-400 hover:text-indigo-300">Manage →</button>
            </div>
            <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-3">
              {#each budgetStatus.filter(r => r.limit_cents > 0) as row}
                {@const color = row.pct_used >= 90 ? 'red' : row.pct_used >= 70 ? 'yellow' : 'green'}
                {@const barColor = color === 'red' ? 'bg-red-500' : color === 'yellow' ? 'bg-yellow-400' : 'bg-emerald-500'}
                {@const textColor = color === 'red' ? 'text-red-400' : color === 'yellow' ? 'text-yellow-400' : 'text-emerald-400'}
                {@const isStanding = !row.budget_month || row.budget_month === 'ALL'}
                <div class="card-sub p-3">
                  <div class="flex items-start justify-between gap-1 mb-0.5">
                    <p class="text-[11px] text-neutral-400 font-medium uppercase truncate">{row.category}</p>
                    {#if isStanding}
                      <span class="flex-none text-[9px] font-semibold uppercase tracking-wide text-neutral-400 bg-neutral-800 rounded px-1 py-0.5 leading-none">standing</span>
                    {:else}
                      <span class="flex-none text-[9px] font-semibold uppercase tracking-wide text-indigo-400 bg-indigo-950/80 rounded px-1 py-0.5 leading-none">this month</span>
                    {/if}
                  </div>
                  <p class="text-xs font-semibold text-neutral-200 mt-0.5 tabular-nums">
                    {$currencySymbol}{(row.actual_cents/100).toFixed(0)} <span class="text-neutral-500 font-normal">/ {$currencySymbol}{(row.limit_cents/100).toFixed(0)}</span>
                  </p>
                  <div class="mt-2 h-1.5 bg-neutral-800 rounded-full overflow-hidden">
                    <div class="h-full rounded-full {barColor} transition-all duration-300" style="width:{Math.min(row.pct_used,100)}%"></div>
                  </div>
                  <p class="text-[10px] {textColor} font-semibold mt-1 text-right tabular-nums">{row.pct_used.toFixed(0)}%</p>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <div class="card">
          <div class="flex items-center justify-between mb-5">
            <div>
              <h2 class="text-sm font-semibold text-neutral-200">Expense Timeline</h2>
              <p class="text-xs text-neutral-500 mt-0.5">Live — updates as expenses are logged</p>
            </div>
            <span class="flex items-center gap-1.5 text-xs text-emerald-400 font-medium">
              <span class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
              Live
            </span>
          </div>
          <RealtimeChart />
        </div>

        <div class="card">
          <div class="flex items-center justify-between mb-5">
            <div>
              <h2 class="text-sm font-semibold text-neutral-200">Income by Person</h2>
              <p class="text-xs text-neutral-500 mt-0.5">{monthLabel} — carry-forwards shown where no income recorded</p>
            </div>
          </div>
          <IncomeChart />
        </div>

        {#if $projects.length > 0}
          <div class="card">
            <div class="flex items-center justify-between mb-5">
              <div>
                <h2 class="text-sm font-semibold text-neutral-200">Savings Projects</h2>
                <p class="text-xs text-neutral-500 mt-0.5">Progress overview — go to Projects tab for full details</p>
              </div>
              <button
                id="goto-projects"
                on:click={() => selectTab('projects')}
                class="btn-ghost text-indigo-400 hover:text-indigo-300"
              >View all →</button>
            </div>
            <div class="space-y-4">
              {#each $projects as project (project.id)}
                {@const progress = Math.min(100, Math.round((project.total_spent_cents / project.target_cents) * 100))}
                {@const isComplete = project.total_spent_cents >= project.target_cents}
                <div class="card-sub p-3.5 flex items-center gap-4">
                  <div class="flex-1 min-w-0">
                    <div class="flex justify-between items-baseline mb-1">
                      <span class="text-xs font-semibold text-neutral-200 truncate">{project.name}</span>
                      <span class="text-xs tabular-nums {isComplete ? 'text-emerald-400' : 'text-neutral-400'} ml-2 flex-none">{progress}%</span>
                    </div>
                    <div class="w-full h-2 bg-neutral-800 rounded-full overflow-hidden">
                      <div
                        class="h-full rounded-full bg-gradient-to-r {isComplete ? 'from-emerald-500 to-emerald-400' : progress >= 60 ? 'from-indigo-500 to-violet-500' : progress >= 30 ? 'from-sky-600 to-indigo-500' : 'from-sky-700 to-sky-500'}"
                        style="width: {progress}%"
                      ></div>
                    </div>
                  </div>
                  <div class="text-right flex-none">
                    <p class="text-xs font-bold text-neutral-100 tabular-nums">
                      {$currencySymbol}{(project.total_spent_cents / 100).toFixed(0)}
                      <span class="text-neutral-500 font-normal">/ {$currencySymbol}{(project.target_cents / 100).toFixed(0)}</span>
                    </p>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Tags summary widget -->
        {#if $tags.length > 0}
          <div class="card">
            <div class="flex items-center justify-between mb-4">
              <div>
                <h2 class="text-sm font-semibold text-neutral-200">Tags</h2>
                <p class="text-xs text-neutral-500 mt-0.5">All-time accumulation per event tag</p>
              </div>
              <button
                id="goto-tags"
                on:click={() => selectTab('tags')}
                class="btn-ghost text-amber-400 hover:text-amber-300"
              >View all →</button>
            </div>
            <div class="flex flex-wrap gap-2">
              {#each $tags as tag (tag.id)}
                <button
                  id="dashboard-tag-chip-{tag.id}"
                  on:click={() => selectTab('tags')}
                  class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full border text-xs font-medium
                         transition-all duration-150 hover:brightness-110 active:scale-[0.97] cursor-pointer"
                  style="background-color: {tag.color}15; color: {tag.color}; border-color: {tag.color}35;"
                >
                  <span class="w-2 h-2 rounded-full flex-none" style="background-color: {tag.color}"></span>
                  {tag.name}
                  <span class="text-[10px] opacity-75 tabular-nums">{$currencySymbol}{(tag.total_amount ?? 0).toLocaleString('en-GB', {minimumFractionDigits: 2, maximumFractionDigits: 2})}</span>
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>

    {:else if activeTab === 'income'}
      <div class="page-container">
        <IncomeTab />
      </div>

    {:else if activeTab === 'expenses'}
      <div class="page-container">
        <header class="page-header">
          <div>
            <h1 class="page-title">Expenses</h1>
            <p class="page-subtitle">Log a new expense or review {monthLabel}'s history</p>
          </div>
          <span class="badge-indigo">{monthLabel}</span>
        </header>

        <div class="grid grid-cols-1 xl:grid-cols-5 gap-6">
          <div class="xl:col-span-2 card">
            <h2 class="text-sm font-semibold text-neutral-200 mb-5">Add Expense</h2>
            <ExpenseForm />
          </div>

          <div class="xl:col-span-3 card">
            <h2 class="text-sm font-semibold text-neutral-200 mb-5">Expense Log</h2>
            <ExpenseList />
          </div>
        </div>
      </div>

    {:else if activeTab === 'splits'}
      <div class="page-container">
        <header class="page-header">
          <div>
            <h1 class="page-title">Categories & Splits</h1>
            <p class="page-subtitle">Manage expense and income categories, and configure household split ratios</p>
          </div>
        </header>

        <div class="card">
          <SplitManager on:navigateIncome={() => (activeTab = 'income')} />
        </div>
      </div>

    {:else if activeTab === 'projects'}
      <div class="page-container">
        <header class="page-header">
          <div>
            <h1 class="page-title">Projects</h1>
            <p class="page-subtitle">Track savings goals and see estimated completion times</p>
          </div>
        </header>
        <ProjectsTab />
      </div>

    {:else if activeTab === 'tags'}
      <div class="page-container">
        <header class="page-header">
          <div>
            <h1 class="page-title">Tags</h1>
            <p class="page-subtitle">Track open-ended events — vacations, repairs, and more — across all months</p>
          </div>
        </header>
        <TagsTab />
      </div>

    {:else if activeTab === 'query'}
      <div class="page-container">
        <header class="page-header">
          <div>
            <h1 class="page-title">Query Console</h1>
            <p class="page-subtitle">Run raw SQL against the SQLite database — results capped at 50 rows</p>
          </div>
        </header>
        <QueryConsole />
      </div>

    {:else if activeTab === 'joint'}
      <div class="page-container">
        <JointAccountTab />
      </div>

    {:else if activeTab === 'settings'}
      <div class="page-container">
        <SettingsTab {tabs} onToggleJointPrompt={() => (showJointPromptModal = true)} />
      </div>

    {:else if activeTab === 'recurring'}
      <div class="page-container">
        <header class="page-header">
          <div>
            <h1 class="page-title">Recurring & Budgets</h1>
            <p class="page-subtitle">Automate monthly expenses and set spending limits</p>
          </div>
        </header>
        <div class="space-y-6">
          <div class="card">
            <RecurringManager />
          </div>
          <div class="card">
            <BudgetManager />
          </div>
        </div>
      </div>
    {/if}

  </main>
</div>
{/if}

{#if showJointPromptModal}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm animate-fadeIn">
    <div class="bg-neutral-900 border border-neutral-800 rounded-2xl p-6 max-w-md w-full shadow-2xl space-y-4">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-xl bg-indigo-500/10 border border-indigo-500/30 flex items-center justify-center text-indigo-400 text-lg flex-none">
          🏦
        </div>
        <div>
          <h3 class="text-base font-bold text-white">Joint Account Activated</h3>
          <p class="text-xs text-neutral-400 mt-0.5">Configure your household account settings</p>
        </div>
      </div>

      <p class="text-sm text-neutral-300">
        Would you like to set up your joint account and connect users & category defaults now, or do it later?
      </p>

      <div class="flex flex-col sm:flex-row gap-2.5 pt-2">
        <button
          id="prompt-connect-now"
          on:click={() => {
            showJointPromptModal = false;
            activeTab = 'joint';
          }}
          class="flex-1 px-4 py-2.5 bg-indigo-600 hover:bg-indigo-500 text-white font-semibold text-xs rounded-xl shadow-lg shadow-indigo-600/30 transition-all text-center"
        >
          Connect & Set Up Now
        </button>
        <button
          id="prompt-do-later"
          on:click={() => {
            showJointPromptModal = false;
          }}
          class="flex-1 px-4 py-2.5 bg-neutral-800 hover:bg-neutral-700 text-neutral-300 font-semibold text-xs rounded-xl border border-neutral-700 transition-all text-center"
        >
          Do It Later
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  :global(.compact-layout) {
    font-size: 0.85rem;
  }
  :global(.compact-layout .p-4) { padding: 0.75rem !important; }
  :global(.compact-layout .p-6) { padding: 1rem !important; }
  :global(.compact-layout .p-8) { padding: 1.25rem !important; }
  :global(.compact-layout .gap-4) { gap: 0.5rem !important; }
  :global(.compact-layout .gap-6) { gap: 0.75rem !important; }

  :global(.large-touch-targets button),
  :global(.large-touch-targets input),
  :global(.large-touch-targets select),
  :global(.large-touch-targets a) {
    min-height: 44px !important;
  }
</style>
