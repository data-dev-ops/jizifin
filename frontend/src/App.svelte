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
  import DocsHub from './lib/docs/DocsHub.svelte';
  import GettingStartedDocs from './lib/docs/GettingStartedDocs.svelte';
  import FrontendDocs from './lib/docs/FrontendDocs.svelte';
  import BackendDocs from './lib/docs/BackendDocs.svelte';
  import SystemDocs from './lib/docs/SystemDocs.svelte';
  import { fetchAllData, fetchAnalytics, fetchIncomeByPerson, fetchPaybacks, fetchBudgetAnalytics, fetchIncome, fetchIncomeCategories, fetchRecurring } from './lib/api.js';
  import { selectedMonth, projects, settlements, users, mobileTabVisibility, mobileAutoCloseMenu, mobileCompactView, mobileLargeTouchTargets, currencySymbol, splits, authSalt, tags, jointAccountEnabled, theme, initDeviceProfiles, privacyShield, textScale, highContrast } from './lib/stores.js';

  let showJointPromptModal = false;

  let activeTab = 'dashboard';
  let loading = false; // Handled after salt is entered
  let error = null;

  function getRouteFromLocation() {
    if (typeof window === 'undefined') return '/';
    const path = window.location.pathname;
    if (path === '/docs' || path === '/docs/') {
      return '/docs';
    } else if (path.startsWith('/docs/getting-started') || path.startsWith('/docs/how-to')) {
      return '/docs/getting-started';
    } else if (path.startsWith('/docs/frontend')) {
      return '/docs/frontend';
    } else if (path.startsWith('/docs/backend')) {
      return '/docs/backend';
    } else if (path.startsWith('/docs/architecture') || path.startsWith('/docs/api')) {
      return '/docs/architecture';
    } else if (window.location.hash.startsWith('#/docs') || window.location.hash.startsWith('#docs')) {
      const hashPath = window.location.hash.replace(/^#/, '');
      return hashPath.startsWith('/') ? hashPath : `/${hashPath}`;
    }
    return '/';
  }

  let currentRoute = getRouteFromLocation();

  function updateRouteFromLocation() {
    currentRoute = getRouteFromLocation();
  }

  function navigateTo(path) {
    if (typeof window !== 'undefined') {
      window.history.pushState({}, '', path);
      updateRouteFromLocation();
    }
  }

  // Sidebar collapsed by default (especially for mobile)
  let sidebarOpen = false;
  let isMobile = false;

  function cycleTheme() {
    if ($theme === 'dark') {
      theme.set('light');
    } else if ($theme === 'light') {
      theme.set('system');
    } else {
      theme.set('dark');
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
      id: 'splits', label: 'Categories',
      icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75"><path stroke-linecap="round" stroke-linejoin="round" d="M12 3v18M3 12h18"/><circle cx="12" cy="12" r="9" stroke-linecap="round"/></svg>`,
    },
    {
      id: 'budgets', label: 'Budgets',
      icon: `<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75"><path stroke-linecap="round" stroke-linejoin="round" d="M12 6v12m-3-2.818l.879.659c1.171.879 3.07.879 4.242 0 1.172-.879 1.172-2.303 0-3.182C13.536 12.219 12.768 12 12 12c-.725 0-1.45-.22-2.003-.659-1.106-.879-1.106-2.303 0-3.182s2.9-.879 4.006 0l.415.33M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>`,
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
    initDeviceProfiles();
    updateRouteFromLocation();
    const checkMobile = () => {
      isMobile = window.innerWidth < 768;
    };
    checkMobile();
    sidebarOpen = !isMobile;
    window.addEventListener('resize', checkMobile);
    window.addEventListener('popstate', updateRouteFromLocation);

    return () => {
      window.removeEventListener('resize', checkMobile);
      window.removeEventListener('popstate', updateRouteFromLocation);
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

{#if currentRoute === '/docs'}
  <DocsHub on:navigate={(e) => navigateTo(e.detail.path)} />
{:else if currentRoute === '/docs/getting-started'}
  <GettingStartedDocs on:navigate={(e) => navigateTo(e.detail.path)} />
{:else if currentRoute === '/docs/frontend'}
  <FrontendDocs on:navigate={(e) => navigateTo(e.detail.path)} />
{:else if currentRoute === '/docs/backend'}
  <BackendDocs on:navigate={(e) => navigateTo(e.detail.path)} />
{:else if currentRoute === '/docs/architecture'}
  <SystemDocs on:navigate={(e) => navigateTo(e.detail.path)} />
{:else if !$authSalt}
  <Login />
{:else}
  <div class="flex h-screen bg-slate-50 dark:bg-neutral-950 text-neutral-900 dark:text-white font-inter overflow-hidden relative {$mobileCompactView ? 'compact-layout' : ''} {$textScale !== '100' ? 'text-scale-' + $textScale : ''} {$highContrast ? 'high-contrast-mode' : ''}">

  <!-- ── Mobile overlay backdrop ──────────────────────────────────────────── -->
  {#if sidebarOpen}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div
      class="fixed inset-0 bg-black/50 dark:bg-black/70 backdrop-blur-sm z-20 md:hidden"
      on:click={() => (sidebarOpen = false)}
    ></div>
  {/if}

  <aside
    class="
      fixed md:relative z-30 md:z-auto
      h-full flex-none flex flex-col
      bg-white dark:bg-neutral-900 border-r border-neutral-200 dark:border-neutral-800
      transition-all duration-300 ease-in-out
      {sidebarOpen ? 'w-60 translate-x-0' : 'w-0 -translate-x-full'}
      overflow-hidden shadow-lg md:shadow-none
    "
  >
    <!-- Logo -->
    <div class="px-5 py-7 flex items-center gap-3 min-w-[240px]">
      <div class="w-9 h-9 rounded-xl bg-gradient-to-br from-indigo-500 to-violet-600 flex items-center justify-center text-base font-bold shadow-lg shadow-indigo-600/30 dark:shadow-indigo-900/40 flex-none text-white">
        {$currencySymbol}
      </div>
      <div>
        <p class="text-sm font-semibold leading-none text-neutral-900 dark:text-white">FinanceTracker</p>
        <p class="text-[11px] text-neutral-500 dark:text-neutral-400 mt-0.5">
          {$users.filter(u => u.is_active).map(u => u.name).join(' & ') || 'Household'}
        </p>
      </div>
    </div>

    <!-- Month selector -->
    <div class="px-3 mb-4 min-w-[240px]">
      <div class="flex items-center justify-between bg-neutral-100 dark:bg-neutral-800 rounded-xl px-2 py-1.5 border border-neutral-200/80 dark:border-neutral-700/50">
        <button
          id="month-prev"
          on:click={() => shiftMonth(-1)}
          class="w-7 h-7 flex items-center justify-center rounded-lg text-neutral-500 dark:text-neutral-400
                 hover:text-neutral-900 dark:hover:text-white hover:bg-neutral-200 dark:hover:bg-neutral-700 transition-colors text-sm"
          aria-label="Previous month"
        >‹</button>
        <span class="text-xs font-semibold text-neutral-800 dark:text-neutral-200 tabular-nums select-none">{monthLabel}</span>
        <button
          id="month-next"
          on:click={() => shiftMonth(1)}
          class="w-7 h-7 flex items-center justify-center rounded-lg text-neutral-500 dark:text-neutral-400
                 hover:text-neutral-900 dark:hover:text-white hover:bg-neutral-200 dark:hover:bg-neutral-700 transition-colors text-sm"
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
                   ? 'bg-indigo-600 text-white shadow-sm shadow-indigo-600/30'
                   : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-100 hover:bg-neutral-100 dark:hover:bg-neutral-800'}"
        >
          <span class="flex-none leading-none">{@html tab.icon}</span>
          <span class="font-medium">{tab.label}</span>
        </button>
      {/each}

      <button
        id="nav-docs"
        on:click={() => navigateTo('/docs')}
        class="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-xs text-neutral-500 hover:text-indigo-600 dark:hover:text-indigo-400 hover:bg-neutral-100 dark:hover:bg-neutral-800 transition-colors mt-2"
      >
        <svg class="w-4 h-4 flex-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 6.042A8.967 8.967 0 006 3.75c-1.052 0-2.062.18-3 .512v14.25A8.987 8.987 0 016 18c2.305 0 4.408.867 6 2.292m0-14.25a8.966 8.966 0 016-2.292c1.052 0 2.062.18 3 .512v14.25A8.987 8.987 0 0018 18a8.967 8.967 0 00-6 2.292m0-14.25v14.25" />
        </svg>
        <span class="font-medium">Documentation</span>
      </button>
    </nav>

    <!-- Footer: dynamic active-user avatars -->
    <div class="px-5 py-5 border-t border-neutral-200 dark:border-neutral-800 min-w-[240px]">
      <button
        on:click={() => selectTab('settings')}
        class="flex items-center gap-3 w-full text-left hover:opacity-80 transition-opacity"
      >
        <div class="flex -space-x-1.5">
          {#each $users.filter(u => u.is_active).slice(0, 4) as u (u.name)}
            <div
              class="w-7 h-7 rounded-full border-2 border-white dark:border-neutral-900 flex items-center justify-center text-[10px] font-bold text-white flex-none shadow-sm"
              style="background-color: {u.color}"
            >{u.name.charAt(0).toUpperCase()}</div>
          {/each}
        </div>
        <div>
          <p class="text-xs font-medium text-neutral-800 dark:text-neutral-200">
            {$users.filter(u => u.is_active).map(u => u.name).join(' & ') || 'Household'}
          </p>
          <p class="text-[10px] text-neutral-500">Shared finances · manage →</p>
        </div>
      </button>
    </div>
  </aside>

  <!-- ── Main content ──────────────────────────────────────────────────────── -->
  <main class="flex-1 overflow-y-auto bg-slate-50 dark:bg-neutral-950 min-w-0">

    <!-- ── Top bar (always visible, contains hamburger, centered time period, & quick theme toggle on right) ─ -->
    <div class="sticky top-0 z-10 bg-white/90 dark:bg-neutral-950/90 backdrop-blur-sm border-b border-neutral-200/80 dark:border-neutral-800/60 px-4 py-3 flex items-center justify-between gap-3 relative">
      <!-- Left side: Hamburger + Active Tab Title -->
      <div class="flex items-center gap-3 min-w-0">
        <button
          id="sidebar-toggle"
          on:click={() => (sidebarOpen = !sidebarOpen)}
          aria-label={sidebarOpen ? 'Close menu' : 'Open menu'}
          class="w-9 h-9 flex flex-col items-center justify-center gap-1.5 rounded-lg
                 text-neutral-500 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-white hover:bg-neutral-100 dark:hover:bg-neutral-800
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

        <span class="text-sm font-semibold text-neutral-800 dark:text-neutral-200 capitalize truncate">{activeTab}</span>
      </div>

      <!-- Center: Name of timeperiod -->
      <div class="absolute left-1/2 -translate-x-1/2 flex items-center pointer-events-none sm:pointer-events-auto">
        <span class="text-xs sm:text-sm font-semibold text-neutral-700 dark:text-neutral-200 tabular-nums px-3 py-1 rounded-full bg-neutral-100/80 dark:bg-neutral-900/80 border border-neutral-200/80 dark:border-neutral-800/80 shadow-xs">
          {monthLabel}
        </span>
      </div>

      <!-- Right side: Quick Privacy Shield + Quick theme toggle -->
      <div class="flex items-center gap-2">
        <button
          id="quick-privacy-toggle"
          type="button"
          on:click={() => privacyShield.update((v) => !v)}
          title="Privacy Shield: {$privacyShield ? 'Active (Balances Masked)' : 'Inactive'} (Click to toggle)"
          class="w-8 h-8 flex items-center justify-center rounded-lg text-xs transition-colors border cursor-pointer {$privacyShield ? 'bg-indigo-600 text-white border-indigo-500 shadow-sm' : 'bg-neutral-100 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-300 hover:text-neutral-900 dark:hover:text-white hover:bg-neutral-200 dark:hover:bg-neutral-700 border-neutral-200 dark:border-neutral-700'}"
          aria-label="Toggle Privacy Shield"
        >
          {$privacyShield ? '🕶️' : '👁️'}
        </button>

        <button
          id="quick-theme-toggle"
          type="button"
          on:click={cycleTheme}
          title="Theme: {$theme} (Click to switch)"
          class="w-8 h-8 flex items-center justify-center rounded-lg text-xs bg-neutral-100 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-300 hover:text-neutral-900 dark:hover:text-white hover:bg-neutral-200 dark:hover:bg-neutral-700 transition-colors border border-neutral-200 dark:border-neutral-700 cursor-pointer"
          aria-label="Toggle theme"
        >
          {#if $theme === 'dark'}
            🌙
          {:else if $theme === 'light'}
            ☀️
          {:else}
            💻
          {/if}
        </button>
      </div>
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

        <AnalyticsSummary on:navigateTab={(e) => selectTab(e.detail)} />
      </div>

    {:else if activeTab === 'income'}
      <div class="page-container">
        <IncomeTab />
      </div>

    {:else if activeTab === 'expenses'}
      <div class="page-container">
        <header class="page-header">
          <div>
            <h1 class="page-title flex items-center gap-2.5">
              <span>💸</span> Expenses Log & Entry
            </h1>
            <p class="page-subtitle">Log a new household expense or review {monthLabel}'s ledger history</p>
          </div>
          <span class="badge-indigo">{monthLabel}</span>
        </header>

        <div class="grid grid-cols-1 xl:grid-cols-5 gap-6 items-start">
          <div class="xl:col-span-2 card space-y-4">
            <div class="border-b border-neutral-200 dark:border-neutral-800 pb-3">
              <h2 class="text-sm font-bold text-neutral-900 dark:text-white flex items-center gap-2">
                <span>➕</span> Add Expense
              </h2>
              <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Quickly log personal or shared purchases</p>
            </div>
            <ExpenseForm />
          </div>

          <div class="xl:col-span-3 card space-y-4">
            <div class="border-b border-neutral-200 dark:border-neutral-800 pb-3 flex items-center justify-between">
              <h2 class="text-sm font-bold text-neutral-900 dark:text-white flex items-center gap-2">
                <span>📋</span> Monthly Ledger History
              </h2>
            </div>
            <ExpenseList />
          </div>
        </div>
      </div>

    {:else if activeTab === 'splits'}
      <div class="page-container">
        <SplitManager on:navigateIncome={() => (activeTab = 'income')} />
      </div>

    {:else if activeTab === 'projects'}
      <div class="page-container">
        <ProjectsTab />
      </div>

    {:else if activeTab === 'tags'}
      <div class="page-container">
        <TagsTab />
      </div>

    {:else if activeTab === 'query'}
      <div class="page-container">
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

    {:else if activeTab === 'budgets'}
      <div class="page-container">
        <BudgetManager />
      </div>

    {:else if activeTab === 'recurring'}
      <div class="page-container">
        <RecurringManager />
      </div>
    {/if}

  </main>
</div>
{/if}

{#if showJointPromptModal}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-fadeIn">
    <div class="card max-w-md w-full shadow-2xl space-y-4">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-xl bg-indigo-500/10 border border-indigo-500/30 flex items-center justify-center text-indigo-600 dark:text-indigo-400 text-lg flex-none">
          🏦
        </div>
        <div>
          <h3 class="text-base font-bold text-neutral-900 dark:text-white">Joint Account Activated</h3>
          <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Configure your household account settings</p>
        </div>
      </div>

      <p class="text-sm text-neutral-600 dark:text-neutral-300">
        Would you like to set up your joint account and connect users & category defaults now, or do it later?
      </p>

      <div class="flex flex-col sm:flex-row gap-2.5 pt-2">
        <button
          id="prompt-connect-now"
          on:click={() => {
            showJointPromptModal = false;
            activeTab = 'joint';
          }}
          class="btn-primary flex-1 text-center"
        >
          Connect & Set Up Now
        </button>
        <button
          id="prompt-do-later"
          on:click={() => {
            showJointPromptModal = false;
          }}
          class="btn-secondary flex-1 text-center"
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
