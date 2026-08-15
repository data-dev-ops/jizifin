<script>
  /**
   * SettingsTab.svelte
   *
   * Central control center for household settings, feature modules,
   * form defaults, visualizations, and zero-knowledge database exports.
   */

  import UserManager from './UserManager.svelte';
  import { exportDatabase } from './api.js';
  import {
    users,
    splits,
    projects,
    currencySymbol,
    defaultPayer,
    defaultCategory,
    defaultProject,
    splitInputMode,
    paybackDisplayMode,
    chartStyle,
    jointAccountEnabled,
    showProjectsInExpense,
    showQueryTab,
    mobileTabVisibility,
    mobileAutoCloseMenu,
    mobileCompactView,
    mobileLargeTouchTargets,
    authSalt
  } from './stores.js';

  export let tabs = [];
  export let onToggleJointPrompt = () => {};

  let exporting = false;
  let exportError = '';
  let tabToggleWarning = '';

  const CURRENCY_PRESETS = ['€', '$', '£', 'CHF', '¥', 'kr'];

  async function handleExport() {
    exporting = true;
    exportError = '';
    try {
      await exportDatabase($authSalt);
    } catch (e) {
      exportError = e.message || 'Export failed.';
    } finally {
      exporting = false;
    }
  }

  function toggleTabVisibility(tabId) {
    tabToggleWarning = '';
    if (tabId === 'settings' || tabId === 'dashboard') {
      tabToggleWarning = `${tabId === 'settings' ? 'Settings' : 'Dashboard'} is required and cannot be disabled.`;
      return;
    }

    const currentVis = { ...$mobileTabVisibility };
    const currentState = !!currentVis[tabId];

    if (currentState) {
      // Enforce at least 1 non-settings tab remains active
      const activeNonSettingsCount = tabs.filter((t) => t.id !== 'settings' && currentVis[t.id]).length;
      if (activeNonSettingsCount <= 1) {
        tabToggleWarning = 'Settings and at least 1 additional tab must remain active.';
        return;
      }
    }

    mobileTabVisibility.update((v) => ({
      ...v,
      [tabId]: !currentState,
    }));
  }

  function handleJointToggle() {
    const next = !$jointAccountEnabled;
    jointAccountEnabled.set(next);
    if (next) {
      onToggleJointPrompt();
    }
  }
</script>

<div class="p-4 sm:p-6 md:p-8 max-w-4xl mx-auto space-y-8 animate-fadeIn">
  <!-- Header -->
  <header>
    <h1 class="text-xl sm:text-2xl font-bold text-white">Settings</h1>
    <p class="text-neutral-400 text-sm mt-1">Manage household members, feature modules, form defaults, and display preferences.</p>
  </header>

  <!-- ── 1. Household Members ──────────────────────────────────────────────── -->
  <div class="card">
    <div class="flex items-center justify-between mb-5 border-b border-neutral-800 pb-4">
      <div>
        <h2 class="text-sm font-bold text-white flex items-center gap-2">
          <span>👥 Household Members</span>
        </h2>
        <p class="text-xs text-neutral-400 mt-1">Configure active household participants and personalized avatar colors.</p>
      </div>
    </div>
    <UserManager />
  </div>

  <!-- ── 2. Feature Modules & Integrations ──────────────────────────────────── -->
  <div class="card space-y-6">
    <div class="border-b border-neutral-800 pb-4">
      <h2 class="text-sm font-bold text-white flex items-center gap-2">
        <span>🧩 Feature Modules</span>
      </h2>
      <p class="text-xs text-neutral-400 mt-1">Opt in or out of advanced modules based on your household structure.</p>
    </div>

    <div class="space-y-4">
      <!-- Joint Account Module -->
      <div class="flex items-center justify-between gap-4 p-4 rounded-xl card-sub">
        <div class="space-y-1">
          <div class="flex items-center gap-2">
            <span class="text-base">🏦</span>
            <p class="text-sm font-semibold text-neutral-200">Joint Account Management</p>
            {#if $jointAccountEnabled}
              <span class="text-[10px] px-2 py-0.5 rounded-full bg-emerald-950 text-emerald-400 border border-emerald-800/60 font-semibold">Active</span>
            {/if}
          </div>
          <p class="text-xs text-neutral-400 leading-relaxed max-w-xl">
            Track shared household balances, automated monthly deposit obligations, expected recurring costs, and balance corrections. Best suited for households with 2+ members.
          </p>
        </div>
        <button
          id="toggle-joint-account-enabled"
          role="switch"
          aria-checked={$jointAccountEnabled}
          on:click={handleJointToggle}
          class="relative inline-flex h-6 w-11 flex-none cursor-pointer rounded-full border-2 border-transparent
                 transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:ring-offset-neutral-900
                 {$jointAccountEnabled ? 'bg-indigo-600' : 'bg-neutral-700'}"
        >
          <span
            class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow
                   transition duration-200 ease-in-out
                   {$jointAccountEnabled ? 'translate-x-5' : 'translate-x-0'}"
          ></span>
        </button>
      </div>

      <!-- Projects Selector in Expense Form -->
      <div class="flex items-center justify-between gap-4 p-4 rounded-xl card-sub">
        <div class="space-y-1">
          <div class="flex items-center gap-2">
            <span class="text-base">▰</span>
            <p class="text-sm font-semibold text-neutral-200">Show Project Dropdown in Expense Form</p>
          </div>
          <p class="text-xs text-neutral-400 leading-relaxed max-w-xl">
            When enabled and active projects exist, displays the project selector dropdown when logging or editing expenses.
          </p>
        </div>
        <button
          id="toggle-show-projects-setting"
          role="switch"
          aria-checked={$showProjectsInExpense}
          on:click={() => showProjectsInExpense.update((v) => !v)}
          class="relative inline-flex h-6 w-11 flex-none cursor-pointer rounded-full border-2 border-transparent
                 transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:ring-offset-neutral-900
                 {$showProjectsInExpense ? 'bg-indigo-600' : 'bg-neutral-700'}"
        >
          <span
            class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow
                   transition duration-200 ease-in-out
                   {$showProjectsInExpense ? 'translate-x-5' : 'translate-x-0'}"
          ></span>
        </button>
      </div>
    </div>
  </div>

  <!-- ── 3. Navigation Tabs Visibility ────────────────────────────────────── -->
  <div class="card space-y-5">
    <div class="border-b border-neutral-800 pb-4">
      <h2 class="text-sm font-bold text-white flex items-center gap-2">
        <span>📑 Navigation Tabs Customization</span>
      </h2>
      <p class="text-xs text-neutral-400 mt-1">Choose which tabs appear in your primary sidebar navigation drawer.</p>
    </div>

    {#if tabToggleWarning}
      <div class="p-3 bg-amber-950/60 border border-amber-800/80 rounded-xl text-amber-300 text-xs flex items-center justify-between gap-2 animate-fadeIn">
        <div class="flex items-center gap-2">
          <svg class="w-4 h-4 text-amber-400 flex-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
          </svg>
          <span>{tabToggleWarning}</span>
        </div>
        <button on:click={() => (tabToggleWarning = '')} class="text-amber-400 hover:text-amber-200 text-sm font-bold flex-none px-1">×</button>
      </div>
    {/if}

    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
      {#each tabs as tab (tab.id)}
        {@const isRequired = tab.id === 'settings' || tab.id === 'dashboard'}
        {@const isJointTab = tab.id === 'joint'}
        {@const isActive = isJointTab ? ($jointAccountEnabled && !!$mobileTabVisibility[tab.id]) : !!$mobileTabVisibility[tab.id]}
        <div class="flex items-center justify-between p-3 rounded-xl bg-neutral-950/80 border border-neutral-800 transition-colors {isRequired ? 'opacity-90' : ''}">
          <div class="flex items-center gap-2.5 min-w-0 pr-2">
            <span class="text-neutral-400 flex-none">{@html tab.icon}</span>
            <div class="min-w-0">
              <p class="text-xs font-semibold text-neutral-200 truncate flex items-center gap-1.5">
                {tab.label}
                {#if isRequired}
                  <span class="text-[9px] uppercase font-bold tracking-wider px-1.5 py-0.2 rounded bg-indigo-950 text-indigo-400 border border-indigo-800/60">Required</span>
                {/if}
              </p>
              <p class="text-[10px] text-neutral-500 truncate">
                {isRequired ? 'Always active' : isJointTab && !$jointAccountEnabled ? 'Enable module first' : isActive ? 'Active' : 'Hidden'}
              </p>
            </div>
          </div>

          <button
            id="toggle-tab-{tab.id}"
            role="switch"
            aria-checked={isActive}
            disabled={isRequired || (isJointTab && !$jointAccountEnabled)}
            on:click={() => toggleTabVisibility(tab.id)}
            class="relative inline-flex h-5 w-9 flex-none cursor-pointer rounded-full border-2 border-transparent
                   transition-colors duration-200 ease-in-out focus:outline-none
                   {isRequired || (isJointTab && !$jointAccountEnabled) ? 'opacity-50 cursor-not-allowed bg-indigo-900' : isActive ? 'bg-indigo-600' : 'bg-neutral-700'}"
          >
            <span
              class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow
                     transition duration-200 ease-in-out
                     {isActive ? 'translate-x-4' : 'translate-x-0'}"
            ></span>
          </button>
        </div>
      {/each}
    </div>
  </div>

  <!-- ── 4. Transaction & Form Defaults ────────────────────────────────────── -->
  <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-5 sm:p-7 shadow-xl shadow-black/20 space-y-5">
    <div class="border-b border-neutral-800 pb-4">
      <h2 class="text-sm font-bold text-white flex items-center gap-2">
        <span>⚡ Form & Entry Defaults</span>
      </h2>
      <p class="text-xs text-neutral-400 mt-1">Speed up logging by pre-selecting default fields for new expense entries.</p>
    </div>

    <div class="space-y-4">
      <!-- Currency Symbol Presets & Input -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-800/60 pb-4">
        <div>
          <p class="text-sm font-medium text-neutral-200">Currency Symbol</p>
          <p class="text-xs text-neutral-400 mt-0.5">Used across all currency displays, charts, and summaries.</p>
        </div>
        <div class="flex items-center gap-2 flex-wrap">
          <div class="flex bg-neutral-950 rounded-lg p-1 border border-neutral-800">
            {#each CURRENCY_PRESETS as sym}
              <button
                type="button"
                on:click={() => currencySymbol.set(sym)}
                class="px-2.5 py-1 text-xs font-semibold rounded-md transition-colors {$currencySymbol === sym ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-400 hover:text-neutral-200'}"
              >
                {sym}
              </button>
            {/each}
          </div>
          <input
            id="setting-currency-symbol"
            type="text"
            maxlength="4"
            bind:value={$currencySymbol}
            class="w-16 bg-neutral-950 border border-neutral-700 rounded-lg px-2.5 py-1 text-xs text-neutral-100 text-center font-bold focus:outline-none focus:border-indigo-500"
            title="Custom currency symbol"
          />
        </div>
      </div>

      <!-- Default Payer -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-800/60 pb-4">
        <div>
          <p class="text-sm font-medium text-neutral-200">Default Payer</p>
          <p class="text-xs text-neutral-400 mt-0.5">Pre-selected household member when adding a new expense.</p>
        </div>
        <select
          id="setting-default-payer"
          bind:value={$defaultPayer}
          class="w-full sm:w-48 bg-neutral-950 border border-neutral-700 rounded-lg px-3 py-1.5 text-xs text-neutral-100 focus:outline-none focus:border-indigo-500"
        >
          <option value="">— None (require selection) —</option>
          {#each $users.filter((u) => u.is_active) as u}
            <option value={u.name}>{u.name}</option>
          {/each}
          {#if $jointAccountEnabled}
            <option value="Joint Account">🏦 Joint Account</option>
          {/if}
        </select>
      </div>

      <!-- Default Category -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-800/60 pb-4">
        <div>
          <p class="text-sm font-medium text-neutral-200">Default Category</p>
          <p class="text-xs text-neutral-400 mt-0.5">Pre-selected category when logging new expenses.</p>
        </div>
        <select
          id="setting-default-category"
          bind:value={$defaultCategory}
          class="w-full sm:w-48 bg-neutral-950 border border-neutral-700 rounded-lg px-3 py-1.5 text-xs text-neutral-100 focus:outline-none focus:border-indigo-500"
        >
          <option value="">— None (require selection) —</option>
          {#each $splits as split}
            <option value={split.category}>{split.category}</option>
          {/each}
        </select>
      </div>

      <!-- Default Project -->
      {#if $projects.length > 0}
        <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div>
            <p class="text-sm font-medium text-neutral-200">Default Project</p>
            <p class="text-xs text-neutral-400 mt-0.5">Pre-selected target project when logging new expenses.</p>
          </div>
          <select
            id="setting-default-project"
            bind:value={$defaultProject}
            class="w-full sm:w-48 bg-neutral-950 border border-neutral-700 rounded-lg px-3 py-1.5 text-xs text-neutral-100 focus:outline-none focus:border-indigo-500"
          >
            <option value="">— None —</option>
            {#each $projects as p}
              <option value={String(p.id)}>{p.name}</option>
            {/each}
          </select>
        </div>
      {/if}
    </div>
  </div>

  <!-- ── 5. Visualizations & Controls ──────────────────────────────────────── -->
  <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-5 sm:p-7 shadow-xl shadow-black/20 space-y-5">
    <div class="border-b border-neutral-800 pb-4">
      <h2 class="text-sm font-bold text-white flex items-center gap-2">
        <span>📊 Visualizations & Controls</span>
      </h2>
      <p class="text-xs text-neutral-400 mt-1">Configure chart layouts and split allocation input modes.</p>
    </div>

    <div class="space-y-4">
      <!-- Category Chart Style -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-800/60 pb-4">
        <div>
          <p class="text-sm font-medium text-neutral-200">Dashboard Spending Chart</p>
          <p class="text-xs text-neutral-400 mt-0.5">Switch between a doughnut chart and a horizontal bar chart on the dashboard.</p>
        </div>
        <div class="flex bg-neutral-950 rounded-lg p-0.5 border border-neutral-800 flex-none">
          <button
            id="setting-chart-doughnut"
            on:click={() => chartStyle.set('doughnut')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$chartStyle === 'doughnut' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-400 hover:text-neutral-200'}"
          >
            Doughnut
          </button>
          <button
            id="setting-chart-bar"
            on:click={() => chartStyle.set('bar')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$chartStyle === 'bar' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-400 hover:text-neutral-200'}"
          >
            Bar Chart
          </button>
        </div>
      </div>

      <!-- Split Input Mode -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-800/60 pb-4">
        <div>
          <p class="text-sm font-medium text-neutral-200">Split Input Style</p>
          <p class="text-xs text-neutral-400 mt-0.5">Slider links 2 users to 100% automatically; Inputs provide manual percentage fields.</p>
        </div>
        <div class="flex bg-neutral-950 rounded-lg p-0.5 border border-neutral-800 flex-none">
          <button
            id="setting-split-inputs"
            on:click={() => splitInputMode.set('inputs')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$splitInputMode === 'inputs' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-400 hover:text-neutral-200'}"
          >
            Inputs
          </button>
          <button
            id="setting-split-slider"
            on:click={() => splitInputMode.set('slider')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$splitInputMode === 'slider' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-400 hover:text-neutral-200'}"
          >
            Slider
          </button>
        </div>
      </div>

      <!-- Payback Display Mode -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div>
          <p class="text-sm font-medium text-neutral-200">Payback Debt Visualizer</p>
          <p class="text-xs text-neutral-400 mt-0.5">Cards show individual user cards with net balances; Bars show proportional stacked bars.</p>
        </div>
        <div class="flex bg-neutral-950 rounded-lg p-0.5 border border-neutral-800 flex-none">
          <button
            id="setting-payback-cards"
            on:click={() => paybackDisplayMode.set('cards')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$paybackDisplayMode === 'cards' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-400 hover:text-neutral-200'}"
          >
            Cards
          </button>
          <button
            id="setting-payback-bars"
            on:click={() => paybackDisplayMode.set('bar')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$paybackDisplayMode === 'bar' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-400 hover:text-neutral-200'}"
          >
            Bars
          </button>
        </div>
      </div>
    </div>
  </div>

  <!-- ── 6. Mobile & Display Preferences ────────────────────────────────────── -->
  <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-5 sm:p-7 shadow-xl shadow-black/20 space-y-4">
    <div class="border-b border-neutral-800 pb-4">
      <h2 class="text-sm font-bold text-white flex items-center gap-2">
        <span>📱 Mobile & Display Experience</span>
      </h2>
      <p class="text-xs text-neutral-400 mt-1">Customize viewport density and touch behavior.</p>
    </div>

    <!-- Auto-close navigation menu -->
    <div class="flex items-center justify-between gap-4 border-b border-neutral-800/60 pb-3.5">
      <div>
        <p class="text-sm font-medium text-neutral-200">Auto-close Sidebar Menu</p>
        <p class="text-xs text-neutral-400 mt-0.5">Automatically dismiss sidebar drawer after selecting a tab on mobile.</p>
      </div>
      <button
        id="toggle-mobile-autoclose"
        role="switch"
        aria-checked={$mobileAutoCloseMenu}
        on:click={() => mobileAutoCloseMenu.update((v) => !v)}
        class="relative inline-flex h-6 w-11 flex-none cursor-pointer rounded-full border-2 border-transparent
               transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-indigo-500
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
        <p class="text-sm font-medium text-neutral-200">Compact Density Layout</p>
        <p class="text-xs text-neutral-400 mt-0.5">Use tighter padding and denser margins across all tables and cards.</p>
      </div>
      <button
        id="toggle-mobile-compact"
        role="switch"
        aria-checked={$mobileCompactView}
        on:click={() => mobileCompactView.update((v) => !v)}
        class="relative inline-flex h-6 w-11 flex-none cursor-pointer rounded-full border-2 border-transparent
               transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-indigo-500
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
        <p class="text-sm font-medium text-neutral-200">Touch-Friendly Large Targets</p>
        <p class="text-xs text-neutral-400 mt-0.5">Enforces 44px minimum tap target height for buttons and form fields on touchscreens.</p>
      </div>
      <button
        id="toggle-mobile-touch-targets"
        role="switch"
        aria-checked={$mobileLargeTouchTargets}
        on:click={() => mobileLargeTouchTargets.update((v) => !v)}
        class="relative inline-flex h-6 w-11 flex-none cursor-pointer rounded-full border-2 border-transparent
               transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-indigo-500
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

  <!-- ── 7. Security & Database Backup ─────────────────────────────────────── -->
  <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-5 sm:p-7 shadow-xl shadow-black/20 space-y-5">
    <div class="border-b border-neutral-800 pb-4">
      <h2 class="text-sm font-bold text-white flex items-center gap-2">
        <span>🔒 Zero-Knowledge Security & Database Backup</span>
      </h2>
      <p class="text-xs text-neutral-400 mt-1">Client-side cryptographic status and unencrypted offline SQLite backup export.</p>
    </div>

    <div class="p-3.5 bg-neutral-950/80 border border-neutral-800 rounded-xl flex items-center gap-3">
      <div class="w-8 h-8 rounded-lg bg-emerald-950 text-emerald-400 border border-emerald-800/60 flex items-center justify-center text-sm flex-none">
        ✓
      </div>
      <div class="min-w-0 flex-1">
        <p class="text-xs font-semibold text-neutral-200">256-Bit AES-GCM Encryption Active</p>
        <p class="text-[11px] text-neutral-400 mt-0.5">All sensitive transaction names, notes, and labels are encrypted in browser memory with PBKDF2 before storage.</p>
      </div>
    </div>

    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pt-2">
      <div>
        <p class="text-sm font-medium text-neutral-200">Export Decrypted Database</p>
        <p class="text-xs text-neutral-400 mt-0.5">Download a fully decrypted SQLite database file (`.db`) for personal backup or viewing in DBeaver.</p>
        {#if exportError}
          <p class="text-xs text-red-400 mt-2">{exportError}</p>
        {/if}
      </div>
      <button
        id="export-db-btn"
        on:click={handleExport}
        disabled={exporting}
        class="w-full sm:w-auto px-4 py-2 bg-neutral-800 hover:bg-neutral-700 text-neutral-200 text-xs font-semibold rounded-xl transition-colors border border-neutral-700 disabled:opacity-50 whitespace-nowrap shadow-sm"
      >
        {exporting ? 'Decrypting & Exporting…' : 'Export .db File'}
      </button>
    </div>
  </div>
</div>
