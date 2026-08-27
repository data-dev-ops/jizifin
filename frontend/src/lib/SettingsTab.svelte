<script>
  /**
   * SettingsTab.svelte
   *
   * Central control center for household settings, appearance & theming,
   * feature modules, form defaults, visualizations, and zero-knowledge database exports.
   */

  import UserManager from './UserManager.svelte';
  import { exportDatabase, resetDatabase } from './api.js';
  import { deriveKey, encryptText } from './crypto.js';
  import {
    theme,
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
    authSalt,
    cryptoKey,
    sessionToken,
    activeDeviceMode,
    detectedDeviceType,
    loadDeviceProfile,
    saveCurrentToProfile,
    copyProfile,
    resetProfileToDefaults,
    experienceTier,
    privacyShield,
    currencyPrecisionMode,
    projectDisplayFilter,
    expenseRowDensity,
    formMemoryMode,
    enabledFormFields,
    dashboardWidgets,
    textScale,
    highContrast,
    applyExperienceTier
  } from './stores.js';

  export let tabs = [];
  export let onToggleJointPrompt = () => {};

  let exporting = false;
  let exportError = '';
  let showResetModal = false;
  let resetConfirmSalt = '';
  let resetError = '';
  let resetting = false;
  let tabToggleWarning = '';
  let profileFeedback = '';
  let profileTimer;

  function showFeedback(msg) {
    profileFeedback = msg;
    if (profileTimer) clearTimeout(profileTimer);
    profileTimer = setTimeout(() => {
      profileFeedback = '';
    }, 4000);
  }

  function handleSwitchProfile(mode) {
    loadDeviceProfile(mode);
    showFeedback(`Loaded ${mode === 'mobile' ? '📱 Mobile' : '💻 Desktop'} profile settings.`);
  }

  function handleSaveDesktop() {
    saveCurrentToProfile('desktop');
    showFeedback('💾 Current settings saved to Desktop Mode profile!');
  }

  function handleSaveMobile() {
    saveCurrentToProfile('mobile');
    showFeedback('💾 Current settings saved to Mobile Mode profile!');
  }

  function handleCopyProfile() {
    const target = $activeDeviceMode === 'desktop' ? 'mobile' : 'desktop';
    copyProfile($activeDeviceMode, target);
    showFeedback(`📋 Copied ${$activeDeviceMode} settings into ${target} profile!`);
  }

  function handleResetProfile() {
    resetProfileToDefaults($activeDeviceMode);
    showFeedback(`↺ Reset ${$activeDeviceMode} profile to standard defaults.`);
  }

  function handleSelectPreset(tier) {
    applyExperienceTier(tier);
    const names = {
      focused: '⚡ Streamlined / Minimal Preset',
      teen: '⚡ Streamlined / Minimal Preset',
      legibility: '👓 High Legibility Preset',
      senior: '👓 High Legibility Preset',
      standard: '🏡 Standard / Balanced Preset',
      detailed: '📊 Detailed / Power User Preset',
      auditor: '📊 Detailed / Power User Preset',
    };
    showFeedback(`Applied preset: ${names[tier] || tier}`);
  }

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

  async function handleResetConfirm() {
    if (!resetConfirmSalt.trim()) {
      resetError = 'Master password is required.';
      return;
    }
    resetting = true;
    resetError = '';
    try {
      const key = await deriveKey(resetConfirmSalt);
      const proof = await encryptText("FinanceTrackerAuth", key);
      await resetDatabase(proof);
      authSalt.set('');
      cryptoKey.set(null);
      sessionToken.set('');
      showResetModal = false;
      if (typeof window !== 'undefined') {
        window.location.href = '/';
      }
    } catch (e) {
      resetError = e.message || 'Incorrect master password or reset failed.';
    } finally {
      resetting = false;
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
    <h1 class="page-title">Settings</h1>
    <p class="page-subtitle">Configure personalized display presets, privacy shields, view filters, rapid logging accelerators, and device display profiles.</p>
  </header>

  <!-- ── 0. Device Display Profiles & Workflow Presets ───────────────────────── -->
  <div class="card space-y-5 border-2 border-indigo-200/80 dark:border-indigo-900/50 bg-gradient-to-b from-indigo-50/30 to-transparent dark:from-indigo-950/20 dark:to-transparent">
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-200 dark:border-neutral-800 pb-4">
      <div>
        <h2 class="text-sm font-bold text-neutral-900 dark:text-white flex items-center gap-2">
          <span>💻 📱 Device Display Profiles & Workflow Presets</span>
        </h2>
        <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-1">
          Customize separate experiences for desktop and mobile devices, or apply functional workflow presets.
        </p>
      </div>

      <!-- Live detected device badge -->
      <div class="flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-neutral-100 dark:bg-neutral-800 border border-neutral-200 dark:border-neutral-700 text-xs font-medium text-neutral-700 dark:text-neutral-300 shrink-0 self-start sm:self-auto">
        <span class="inline-block w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></span>
        <span>Current Device:</span>
        <strong class="text-neutral-900 dark:text-white capitalize">{$detectedDeviceType === 'mobile' ? '📱 Mobile' : '💻 Desktop'}</strong>
      </div>
    </div>

    <!-- Active Profile Mode Switcher -->
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-3.5 rounded-xl bg-neutral-50/90 dark:bg-neutral-950/80 border border-neutral-200 dark:border-neutral-800">
      <div class="space-y-0.5">
        <p class="text-xs font-semibold text-neutral-800 dark:text-neutral-200">
          Currently Editing Profile: <span class="capitalize text-indigo-600 dark:text-indigo-400 font-bold">{$activeDeviceMode} Mode</span>
        </p>
        <p class="text-[11px] text-neutral-500 dark:text-neutral-400">
          Switch to customize preferences for that specific device format.
        </p>
      </div>

      <div class="flex bg-white dark:bg-neutral-900 rounded-lg p-1 border border-neutral-200 dark:border-neutral-800 shrink-0">
        <button
          id="btn-switch-profile-desktop"
          type="button"
          on:click={() => handleSwitchProfile('desktop')}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$activeDeviceMode === 'desktop' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-white'}"
        >
          <span>💻 Desktop Profile</span>
        </button>
        <button
          id="btn-switch-profile-mobile"
          type="button"
          on:click={() => handleSwitchProfile('mobile')}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$activeDeviceMode === 'mobile' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-white'}"
        >
          <span>📱 Mobile Profile</span>
        </button>
      </div>
    </div>

    <!-- Quick Display & Workflow Presets -->
    <div class="space-y-2 pt-1">
      <p class="text-xs font-bold text-neutral-800 dark:text-neutral-200 flex items-center gap-1.5">
        <span>⚡ Quick Display & Workflow Presets:</span>
      </p>
      <div class="grid grid-cols-2 sm:grid-cols-4 gap-2.5">
        <!-- Streamlined / Minimal -->
        <button
          type="button"
          id="preset-focused"
          on:click={() => handleSelectPreset('focused')}
          class="p-2.5 rounded-xl border text-left transition-all {$experienceTier === 'focused' || $experienceTier === 'teen' ? 'border-indigo-600 bg-indigo-50/70 dark:bg-indigo-950/50 shadow-xs' : 'border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 hover:border-neutral-300'}"
        >
          <span class="text-base block mb-1">⚡</span>
          <p class="text-xs font-bold text-neutral-900 dark:text-white">Streamlined</p>
          <p class="text-[10px] text-neutral-500 mt-0.5">Minimal fields, clean single-line rows, fast logging</p>
        </button>

        <!-- High Legibility -->
        <button
          type="button"
          id="preset-legibility"
          on:click={() => handleSelectPreset('legibility')}
          class="p-2.5 rounded-xl border text-left transition-all {$experienceTier === 'legibility' || $experienceTier === 'senior' ? 'border-indigo-600 bg-indigo-50/70 dark:bg-indigo-950/50 shadow-xs' : 'border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 hover:border-neutral-300'}"
        >
          <span class="text-base block mb-1">👓</span>
          <p class="text-xs font-bold text-neutral-900 dark:text-white">High Legibility</p>
          <p class="text-[10px] text-neutral-500 mt-0.5">115% larger text, high-contrast borders, clean numbers</p>
        </button>

        <!-- Standard / Balanced -->
        <button
          type="button"
          id="preset-standard"
          on:click={() => handleSelectPreset('standard')}
          class="p-2.5 rounded-xl border text-left transition-all {$experienceTier === 'standard' ? 'border-indigo-600 bg-indigo-50/70 dark:bg-indigo-950/50 shadow-xs' : 'border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 hover:border-neutral-300'}"
        >
          <span class="text-base block mb-1">🏡</span>
          <p class="text-xs font-bold text-neutral-900 dark:text-white">Standard</p>
          <p class="text-[10px] text-neutral-500 mt-0.5">Full household overview, default density, all modules</p>
        </button>

        <!-- Detailed / Power User -->
        <button
          type="button"
          id="preset-detailed"
          on:click={() => handleSelectPreset('detailed')}
          class="p-2.5 rounded-xl border text-left transition-all {$experienceTier === 'detailed' || $experienceTier === 'auditor' ? 'border-indigo-600 bg-indigo-50/70 dark:bg-indigo-950/50 shadow-xs' : 'border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 hover:border-neutral-300'}"
        >
          <span class="text-base block mb-1">📊</span>
          <p class="text-xs font-bold text-neutral-900 dark:text-white">Power User</p>
          <p class="text-[10px] text-neutral-500 mt-0.5">Exact cents, full metadata chips, SQL console</p>
        </button>
      </div>
    </div>

    {#if profileFeedback}
      <div class="p-3 bg-emerald-50 dark:bg-emerald-950/60 border border-emerald-200 dark:border-emerald-800 rounded-xl text-emerald-800 dark:text-emerald-300 text-xs flex items-center justify-between gap-2 animate-fadeIn">
        <span>{profileFeedback}</span>
        <button on:click={() => (profileFeedback = '')} class="text-emerald-600 dark:text-emerald-400 hover:opacity-80 text-sm font-bold flex-none px-1">×</button>
      </div>
    {/if}

    <!-- Quick Profile Action Buttons -->
    <div class="flex flex-wrap items-center justify-between gap-2.5 pt-1">
      <div class="flex items-center gap-2 flex-wrap">
        <button
          id="btn-save-desktop-profile"
          type="button"
          on:click={handleSaveDesktop}
          class="px-3.5 py-2 rounded-xl text-xs font-semibold bg-white dark:bg-neutral-900 text-neutral-800 dark:text-neutral-200 hover:bg-neutral-50 dark:hover:bg-neutral-800 border border-neutral-300 dark:border-neutral-700 shadow-xs transition-all flex items-center gap-1.5 cursor-pointer"
        >
          <span>💾 Save to Desktop Mode</span>
        </button>

        <button
          id="btn-save-mobile-profile"
          type="button"
          on:click={handleSaveMobile}
          class="px-3.5 py-2 rounded-xl text-xs font-semibold bg-white dark:bg-neutral-900 text-neutral-800 dark:text-neutral-200 hover:bg-neutral-50 dark:hover:bg-neutral-800 border border-neutral-300 dark:border-neutral-700 shadow-xs transition-all flex items-center gap-1.5 cursor-pointer"
        >
          <span>💾 Save to Mobile Mode</span>
        </button>
      </div>

      <div class="flex items-center gap-2 flex-wrap">
        <button
          id="btn-copy-profile"
          type="button"
          on:click={handleCopyProfile}
          title="Copy current profile settings to the other device mode"
          class="px-3 py-1.5 rounded-lg text-xs font-medium text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-white hover:bg-neutral-100 dark:hover:bg-neutral-800 transition-colors cursor-pointer"
        >
          🔄 Copy to {$activeDeviceMode === 'desktop' ? 'Mobile' : 'Desktop'}
        </button>

        <button
          id="btn-reset-profile"
          type="button"
          on:click={handleResetProfile}
          title="Reset this profile to standard defaults"
          class="px-3 py-1.5 rounded-lg text-xs font-medium text-rose-600 dark:text-rose-400 hover:bg-rose-50 dark:hover:bg-rose-950/40 transition-colors cursor-pointer"
        >
          ↺ Reset {$activeDeviceMode} Defaults
        </button>
      </div>
    </div>
  </div>

  <!-- ── 1. Theme, Privacy Shield & Legibility ───────────────────────────────── -->
  <div class="card space-y-5">
    <div class="border-b border-neutral-200 dark:border-neutral-800 pb-4">
      <h2 class="text-sm font-bold text-neutral-900 dark:text-white flex items-center gap-2">
        <span>🎨 Appearance, Privacy Shield & Accessibility</span>
      </h2>
      <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-1">Configure interface theme, public screen privacy masks, and legibility scaling.</p>
    </div>

    <!-- Theme Mode Cards -->
    <div class="grid grid-cols-1 sm:grid-cols-3 gap-3.5">
      <!-- Dark Theme Card -->
      <button
        type="button"
        id="theme-dark"
        on:click={() => theme.set('dark')}
        class="flex flex-col items-start p-4 rounded-xl border-2 transition-all duration-200 text-left cursor-pointer
               {$theme === 'dark'
                 ? 'border-indigo-600 bg-indigo-50/50 dark:bg-indigo-950/40 shadow-sm shadow-indigo-500/10'
                 : 'border-neutral-200 dark:border-neutral-800 bg-neutral-50 dark:bg-neutral-950/60 hover:border-neutral-300 dark:hover:border-neutral-700'}"
      >
        <div class="flex items-center justify-between w-full mb-3">
          <div class="w-8 h-8 rounded-lg bg-neutral-900 text-indigo-400 flex items-center justify-center text-sm shadow-inner">
            🌙
          </div>
          {#if $theme === 'dark'}
            <span class="badge-indigo">Active</span>
          {/if}
        </div>
        <p class="text-sm font-semibold text-neutral-900 dark:text-neutral-100">Dark Mode</p>
        <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-1 leading-relaxed">
          Signature deep luxury navy-slate background and crisp typography.
        </p>
      </button>

      <!-- Light Theme Card -->
      <button
        type="button"
        id="theme-light"
        on:click={() => theme.set('light')}
        class="flex flex-col items-start p-4 rounded-xl border-2 transition-all duration-200 text-left cursor-pointer
               {$theme === 'light'
                 ? 'border-indigo-600 bg-indigo-50/50 dark:bg-indigo-950/40 shadow-sm shadow-indigo-500/10'
                 : 'border-neutral-200 dark:border-neutral-800 bg-neutral-50 dark:bg-neutral-950/60 hover:border-neutral-300 dark:hover:border-neutral-700'}"
      >
        <div class="flex items-center justify-between w-full mb-3">
          <div class="w-8 h-8 rounded-lg bg-amber-100 dark:bg-amber-950 text-amber-600 dark:text-amber-400 flex items-center justify-center text-sm shadow-inner">
            ☀️
          </div>
          {#if $theme === 'light'}
            <span class="badge-indigo">Active</span>
          {/if}
        </div>
        <p class="text-sm font-semibold text-neutral-900 dark:text-neutral-100">Light Mode</p>
        <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-1 leading-relaxed">
          Clean modern aesthetic with layered surfaces and crisp financial cards.
        </p>
      </button>

      <!-- System Default Card -->
      <button
        type="button"
        id="theme-system"
        on:click={() => theme.set('system')}
        class="flex flex-col items-start p-4 rounded-xl border-2 transition-all duration-200 text-left cursor-pointer
               {$theme === 'system'
                 ? 'border-indigo-600 bg-indigo-50/50 dark:bg-indigo-950/40 shadow-sm shadow-indigo-500/10'
                 : 'border-neutral-200 dark:border-neutral-800 bg-neutral-50 dark:bg-neutral-950/60 hover:border-neutral-300 dark:hover:border-neutral-700'}"
      >
        <div class="flex items-center justify-between w-full mb-3">
          <div class="w-8 h-8 rounded-lg bg-neutral-200 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300 flex items-center justify-center text-sm shadow-inner">
            💻
          </div>
          {#if $theme === 'system'}
            <span class="badge-indigo">Active</span>
          {/if}
        </div>
        <p class="text-sm font-semibold text-neutral-900 dark:text-neutral-100">System Sync</p>
        <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-1 leading-relaxed">
          Automatically adapts between light and dark modes based on system settings.
        </p>
      </button>
    </div>

    <!-- Privacy Shield & Legibility Controls -->
    <div class="space-y-4 pt-2">
      <!-- Privacy Shield Toggle -->
      <div class="flex items-center justify-between gap-4 p-3.5 rounded-xl card-sub">
        <div class="space-y-0.5">
          <div class="flex items-center gap-2">
            <span class="text-base">🕶️</span>
            <p class="text-sm font-semibold text-neutral-800 dark:text-neutral-200">Privacy Shield (Stealth Blur)</p>
            {#if $privacyShield}
              <span class="badge-indigo">Active</span>
            {/if}
          </div>
          <p class="text-xs text-neutral-500 dark:text-neutral-400 max-w-xl">
            Blurs salary and transaction numbers across the app to prevent shoulder-surfing on public transport or screen-shares. Hover or tap to unmask temporarily.
          </p>
        </div>
        <button
          id="toggle-privacy-shield"
          role="switch"
          aria-checked={$privacyShield}
          on:click={() => privacyShield.update((v) => !v)}
          class="relative inline-flex h-6 w-11 flex-none cursor-pointer rounded-full border-2 border-transparent
                 transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-indigo-500
                 {$privacyShield ? 'bg-indigo-600' : 'bg-neutral-300 dark:bg-neutral-700'}"
        >
          <span
            class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow
                   transition duration-200 ease-in-out
                   {$privacyShield ? 'translate-x-5' : 'translate-x-0'}"
          ></span>
        </button>
      </div>

      <!-- Text Scale -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-t border-neutral-200 dark:border-neutral-800/60 pt-3.5">
        <div>
          <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">Typography Scale (Accessibility)</p>
          <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Enlarge font sizes for effortless legibility across cards and tables.</p>
        </div>
        <div class="flex bg-neutral-100 dark:bg-neutral-950 rounded-lg p-0.5 border border-neutral-200 dark:border-neutral-800">
          <button
            type="button"
            id="btn-scale-100"
            on:click={() => textScale.set('100')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$textScale === '100' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900'}"
          >
            100% Standard
          </button>
          <button
            type="button"
            id="btn-scale-115"
            on:click={() => textScale.set('115')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$textScale === '115' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900'}"
          >
            115% Comfortable
          </button>
          <button
            type="button"
            id="btn-scale-130"
            on:click={() => textScale.set('130')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$textScale === '130' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900'}"
          >
            130% Large
          </button>
        </div>
      </div>

      <!-- High Contrast Mode -->
      <div class="flex items-center justify-between gap-4 border-t border-neutral-200 dark:border-neutral-800/60 pt-3.5">
        <div>
          <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">High-Contrast Border Mode</p>
          <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Increases card border weight and button contrast for users with low vision.</p>
        </div>
        <button
          id="toggle-high-contrast"
          role="switch"
          aria-checked={$highContrast}
          on:click={() => highContrast.update((v) => !v)}
          class="relative inline-flex h-6 w-11 flex-none cursor-pointer rounded-full border-2 border-transparent
                 transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-indigo-500
                 {$highContrast ? 'bg-indigo-600' : 'bg-neutral-300 dark:bg-neutral-700'}"
        >
          <span
            class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow
                   transition duration-200 ease-in-out
                   {$highContrast ? 'translate-x-5' : 'translate-x-0'}"
          ></span>
        </button>
      </div>
    </div>
  </div>

  <!-- ── 2. Household Members ──────────────────────────────────────────────── -->
  <div class="card">
    <div class="flex items-center justify-between mb-5 border-b border-neutral-200 dark:border-neutral-800 pb-4">
      <div>
        <h2 class="text-sm font-bold text-neutral-900 dark:text-white flex items-center gap-2">
          <span>👥 Household Members</span>
        </h2>
        <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-1">Configure active household participants and personalized avatar colors.</p>
      </div>
    </div>
    <UserManager />
  </div>

  <!-- ── 3. Feature Modules & Integrations ──────────────────────────────────── -->
  <div class="card space-y-6">
    <div class="border-b border-neutral-200 dark:border-neutral-800 pb-4">
      <h2 class="text-sm font-bold text-neutral-900 dark:text-white flex items-center gap-2">
        <span>🧩 Feature Modules</span>
      </h2>
      <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-1">Opt in or out of advanced modules based on your household structure.</p>
    </div>

    <div class="space-y-4">
      <!-- Joint Account Module -->
      <div class="flex items-center justify-between gap-4 p-4 rounded-xl card-sub">
        <div class="space-y-1">
          <div class="flex items-center gap-2">
            <span class="text-base">🏦</span>
            <p class="text-sm font-semibold text-neutral-800 dark:text-neutral-200">Joint Account Management</p>
            {#if $jointAccountEnabled}
              <span class="badge-emerald">Active</span>
            {/if}
          </div>
          <p class="text-xs text-neutral-500 dark:text-neutral-400 leading-relaxed max-w-xl">
            Track shared household balances, automated monthly deposit obligations, expected recurring costs, and balance corrections.
          </p>
        </div>
        <button
          id="toggle-joint-account-enabled"
          role="switch"
          aria-checked={$jointAccountEnabled}
          on:click={handleJointToggle}
          class="relative inline-flex h-6 w-11 flex-none cursor-pointer rounded-full border-2 border-transparent
                 transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:ring-offset-white dark:focus:ring-offset-neutral-900
                 {$jointAccountEnabled ? 'bg-indigo-600' : 'bg-neutral-300 dark:bg-neutral-700'}"
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
            <span class="text-base">🎯</span>
            <p class="text-sm font-semibold text-neutral-800 dark:text-neutral-200">Show Project Dropdown in Expense Form</p>
          </div>
          <p class="text-xs text-neutral-500 dark:text-neutral-400 leading-relaxed max-w-xl">
            Displays the project selector dropdown when logging or editing expenses.
          </p>
        </div>
        <button
          id="toggle-show-projects-setting"
          role="switch"
          aria-checked={$showProjectsInExpense}
          on:click={() => showProjectsInExpense.update((v) => !v)}
          class="relative inline-flex h-6 w-11 flex-none cursor-pointer rounded-full border-2 border-transparent
                 transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:ring-offset-white dark:focus:ring-offset-neutral-900
                 {$showProjectsInExpense ? 'bg-indigo-600' : 'bg-neutral-300 dark:bg-neutral-700'}"
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

  <!-- ── 4. View Depth & Clutter Filters ────────────────────────────────────── -->
  <div class="card space-y-6">
    <div class="border-b border-neutral-200 dark:border-neutral-800 pb-4">
      <h2 class="text-sm font-bold text-neutral-900 dark:text-white flex items-center gap-2">
        <span>📑 View Depth & Clutter Filters</span>
      </h2>
      <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-1">Control information density, ledger row detail, and navigation tabs.</p>
    </div>

    <div class="space-y-4">
      <!-- Project Lifecycle Display Filter -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-200 dark:border-neutral-800/60 pb-4">
        <div>
          <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">Default Project Goals View</p>
          <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Filter out completed savings milestones from the primary project grid.</p>
        </div>
        <div class="flex bg-neutral-100 dark:bg-neutral-950 rounded-lg p-0.5 border border-neutral-200 dark:border-neutral-800">
          <button
            type="button"
            id="setting-proj-active"
            on:click={() => projectDisplayFilter.set('active')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$projectDisplayFilter === 'active' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900'}"
          >
            Active Only
          </button>
          <button
            type="button"
            id="setting-proj-in-progress"
            on:click={() => projectDisplayFilter.set('in_progress')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$projectDisplayFilter === 'in_progress' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900'}"
          >
            In Progress
          </button>
          <button
            type="button"
            id="setting-proj-all"
            on:click={() => projectDisplayFilter.set('all')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$projectDisplayFilter === 'all' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900'}"
          >
            All Archive
          </button>
        </div>
      </div>

      <!-- Expense Ledger Density -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-200 dark:border-neutral-800/60 pb-4">
        <div>
          <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">Expense Ledger Row Density</p>
          <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Minimal removes badges for maximum speed; Detailed displays full tags and notes.</p>
        </div>
        <div class="flex bg-neutral-100 dark:bg-neutral-950 rounded-lg p-0.5 border border-neutral-200 dark:border-neutral-800">
          <button
            type="button"
            id="setting-density-minimal"
            on:click={() => expenseRowDensity.set('minimal')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$expenseRowDensity === 'minimal' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900'}"
          >
            Minimal
          </button>
          <button
            type="button"
            id="setting-density-compact"
            on:click={() => expenseRowDensity.set('compact')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$expenseRowDensity === 'compact' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900'}"
          >
            Compact
          </button>
          <button
            type="button"
            id="setting-density-detailed"
            on:click={() => expenseRowDensity.set('detailed')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$expenseRowDensity === 'detailed' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900'}"
          >
            Detailed
          </button>
        </div>
      </div>

      <!-- Currency Decimal Precision Mode -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-200 dark:border-neutral-800/60 pb-4">
        <div>
          <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">Currency Summary Decimals</p>
          <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Whole numbers on summary cards creates cleaner visual balance; Always exact shows .00 everywhere.</p>
        </div>
        <div class="flex bg-neutral-100 dark:bg-neutral-950 rounded-lg p-0.5 border border-neutral-200 dark:border-neutral-800">
          <button
            type="button"
            id="setting-precision-whole"
            on:click={() => currencyPrecisionMode.set('whole_units_on_summaries')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$currencyPrecisionMode === 'whole_units_on_summaries' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900'}"
          >
            Whole Units
          </button>
          <button
            type="button"
            id="setting-precision-exact"
            on:click={() => currencyPrecisionMode.set('always_exact')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$currencyPrecisionMode === 'always_exact' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900'}"
          >
            Always Exact (.00)
          </button>
        </div>
      </div>

      <!-- Navigation Drawer Tabs Checklist -->
      <div class="space-y-3 pt-2">
        <p class="text-xs font-semibold text-neutral-800 dark:text-neutral-200">Sidebar Drawer Tab Visibility</p>
        {#if tabToggleWarning}
          <div class="p-3 bg-amber-50 dark:bg-amber-950/60 border border-amber-200 dark:border-amber-800/80 rounded-xl text-amber-800 dark:text-amber-300 text-xs flex items-center justify-between gap-2 animate-fadeIn">
            <span>{tabToggleWarning}</span>
            <button on:click={() => (tabToggleWarning = '')} class="text-amber-500 font-bold">×</button>
          </div>
        {/if}

        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
          {#each tabs as tab (tab.id)}
            {@const isRequired = tab.id === 'settings' || tab.id === 'dashboard'}
            {@const isJointTab = tab.id === 'joint'}
            {@const isActive = isJointTab ? ($jointAccountEnabled && !!$mobileTabVisibility[tab.id]) : !!$mobileTabVisibility[tab.id]}
            <div class="flex items-center justify-between p-3 rounded-xl bg-neutral-50 dark:bg-neutral-950/80 border border-neutral-200 dark:border-neutral-800 transition-colors {isRequired ? 'opacity-90' : ''}">
              <div class="flex items-center gap-2.5 min-w-0 pr-2">
                <span class="text-neutral-500 dark:text-neutral-400 flex-none">{@html tab.icon}</span>
                <div class="min-w-0">
                  <p class="text-xs font-semibold text-neutral-800 dark:text-neutral-200 truncate flex items-center gap-1.5">
                    {tab.label}
                    {#if isRequired}
                      <span class="text-[9px] uppercase font-bold tracking-wider px-1.5 py-0.2 rounded bg-indigo-100 dark:bg-indigo-950 text-indigo-700 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-800/60">Required</span>
                    {/if}
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
                       {isRequired || (isJointTab && !$jointAccountEnabled) ? 'opacity-50 cursor-not-allowed bg-indigo-300 dark:bg-indigo-900' : isActive ? 'bg-indigo-600' : 'bg-neutral-300 dark:bg-neutral-700'}"
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
    </div>
  </div>

  <!-- ── 5. Form Accelerators & Rapid Entry ─────────────────────────────────── -->
  <div class="card space-y-5">
    <div class="border-b border-neutral-200 dark:border-neutral-800 pb-4">
      <h2 class="text-sm font-bold text-neutral-900 dark:text-white flex items-center gap-2">
        <span>⚡ Form Accelerators & Rapid Logging</span>
      </h2>
      <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-1">Optimize logging speed with smart field memory and customizable optional inputs.</p>
    </div>

    <div class="space-y-4">
      <!-- Form Memory Mode -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-200 dark:border-neutral-800/60 pb-4">
        <div>
          <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">Smart Form Memory Mode</p>
          <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Remember Last automatically remembers your most recently logged payer & category.</p>
        </div>
        <div class="flex bg-neutral-100 dark:bg-neutral-950 rounded-lg p-0.5 border border-neutral-200 dark:border-neutral-800">
          <button
            type="button"
            id="setting-memory-last"
            on:click={() => formMemoryMode.set('remember_last')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$formMemoryMode === 'remember_last' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900'}"
          >
            Remember Last
          </button>
          <button
            type="button"
            id="setting-memory-static"
            on:click={() => formMemoryMode.set('static_preset')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$formMemoryMode === 'static_preset' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900'}"
          >
            Static Defaults
          </button>
          <button
            type="button"
            id="setting-memory-empty"
            on:click={() => formMemoryMode.set('empty')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$formMemoryMode === 'empty' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900'}"
          >
            Empty
          </button>
        </div>
      </div>

      <!-- Enabled Optional Form Fields -->
      <div class="space-y-3 border-b border-neutral-200 dark:border-neutral-800/60 pb-4">
        <p class="text-xs font-semibold text-neutral-800 dark:text-neutral-200">Visible Fields on Expense Form</p>
        <div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
          <label class="flex items-center gap-2 p-2.5 rounded-xl bg-neutral-50 dark:bg-neutral-950/80 border border-neutral-200 dark:border-neutral-800 text-xs font-medium text-neutral-800 dark:text-neutral-200 cursor-pointer">
            <input
              type="checkbox"
              id="field-toggle-projects"
              checked={$enabledFormFields.projects}
              on:change={(e) => enabledFormFields.update(f => ({ ...f, projects: e.target.checked }))}
              class="rounded text-indigo-600"
            />
            <span>🎯 Projects</span>
          </label>

          <label class="flex items-center gap-2 p-2.5 rounded-xl bg-neutral-50 dark:bg-neutral-950/80 border border-neutral-200 dark:border-neutral-800 text-xs font-medium text-neutral-800 dark:text-neutral-200 cursor-pointer">
            <input
              type="checkbox"
              id="field-toggle-tags"
              checked={$enabledFormFields.tags}
              on:change={(e) => enabledFormFields.update(f => ({ ...f, tags: e.target.checked }))}
              class="rounded text-indigo-600"
            />
            <span>🏷️ Tags</span>
          </label>

          <label class="flex items-center gap-2 p-2.5 rounded-xl bg-neutral-50 dark:bg-neutral-950/80 border border-neutral-200 dark:border-neutral-800 text-xs font-medium text-neutral-800 dark:text-neutral-200 cursor-pointer">
            <input
              type="checkbox"
              id="field-toggle-joint"
              checked={$enabledFormFields.joint}
              on:change={(e) => enabledFormFields.update(f => ({ ...f, joint: e.target.checked }))}
              class="rounded text-indigo-600"
            />
            <span>🏦 Joint Account</span>
          </label>

          <label class="flex items-center gap-2 p-2.5 rounded-xl bg-neutral-50 dark:bg-neutral-950/80 border border-neutral-200 dark:border-neutral-800 text-xs font-medium text-neutral-800 dark:text-neutral-200 cursor-pointer">
            <input
              type="checkbox"
              id="field-toggle-split-override"
              checked={$enabledFormFields.splitOverride}
              on:change={(e) => enabledFormFields.update(f => ({ ...f, splitOverride: e.target.checked }))}
              class="rounded text-indigo-600"
            />
            <span>✶ Custom Split</span>
          </label>
        </div>
      </div>

      <!-- Currency Symbol Presets & Input -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-200 dark:border-neutral-800/60 pb-4">
        <div>
          <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">Currency Symbol</p>
          <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Used across all currency displays, charts, and summaries.</p>
        </div>
        <div class="flex items-center gap-2 flex-wrap">
          <div class="flex bg-neutral-100 dark:bg-neutral-950 rounded-lg p-1 border border-neutral-200 dark:border-neutral-800">
            {#each CURRENCY_PRESETS as sym}
              <button
                type="button"
                on:click={() => currencySymbol.set(sym)}
                class="px-2.5 py-1 text-xs font-semibold rounded-md transition-colors {$currencySymbol === sym ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
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
            class="w-16 bg-white dark:bg-neutral-950 border border-neutral-300 dark:border-neutral-700 rounded-lg px-2.5 py-1 text-xs text-neutral-900 dark:text-neutral-100 text-center font-bold focus:outline-none focus:border-indigo-500"
            title="Custom currency symbol"
          />
        </div>
      </div>

      <!-- Default Payer -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-200 dark:border-neutral-800/60 pb-4">
        <div>
          <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">Static Default Payer</p>
          <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Pre-selected household member when adding a new expense.</p>
        </div>
        <select
          id="setting-default-payer"
          bind:value={$defaultPayer}
          class="w-full sm:w-48 bg-white dark:bg-neutral-950 border border-neutral-300 dark:border-neutral-700 rounded-lg px-3 py-1.5 text-xs text-neutral-900 dark:text-neutral-100 focus:outline-none focus:border-indigo-500"
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
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-200 dark:border-neutral-800/60 pb-4">
        <div>
          <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">Static Default Category</p>
          <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Pre-selected category when logging new expenses.</p>
        </div>
        <select
          id="setting-default-category"
          bind:value={$defaultCategory}
          class="w-full sm:w-48 bg-white dark:bg-neutral-950 border border-neutral-300 dark:border-neutral-700 rounded-lg px-3 py-1.5 text-xs text-neutral-900 dark:text-neutral-100 focus:outline-none focus:border-indigo-500"
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
            <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">Default Project</p>
            <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Pre-selected target project when logging new expenses.</p>
          </div>
          <select
            id="setting-default-project"
            bind:value={$defaultProject}
            class="w-full sm:w-48 bg-white dark:bg-neutral-950 border border-neutral-300 dark:border-neutral-700 rounded-lg px-3 py-1.5 text-xs text-neutral-900 dark:text-neutral-100 focus:outline-none focus:border-indigo-500"
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

  <!-- ── 6. Visualizations & Modular Widgets ────────────────────────────────── -->
  <div class="card space-y-5">
    <div class="border-b border-neutral-200 dark:border-neutral-800 pb-4">
      <h2 class="text-sm font-bold text-neutral-900 dark:text-white flex items-center gap-2">
        <span>📊 Visualizations & Modular Dashboard Widgets</span>
      </h2>
      <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-1">Configure chart layouts and toggle individual dashboard KPI widgets.</p>
    </div>

    <div class="space-y-4">
      <!-- Modular Dashboard Widgets -->
      <div class="space-y-3 border-b border-neutral-200 dark:border-neutral-800/60 pb-4">
        <p class="text-xs font-semibold text-neutral-800 dark:text-neutral-200">Active Dashboard Widgets</p>
        <div class="grid grid-cols-2 sm:grid-cols-3 gap-3">
          <label class="flex items-center gap-2 p-2.5 rounded-xl bg-neutral-50 dark:bg-neutral-950/80 border border-neutral-200 dark:border-neutral-800 text-xs font-medium text-neutral-800 dark:text-neutral-200 cursor-pointer">
            <input
              type="checkbox"
              id="widget-toggle-monthly-total"
              checked={$dashboardWidgets.monthlyTotal !== false}
              on:change={(e) => dashboardWidgets.update(w => ({ ...w, monthlyTotal: e.target.checked }))}
              class="rounded text-indigo-600"
            />
            <span>💰 Monthly Total Spend</span>
          </label>

          <label class="flex items-center gap-2 p-2.5 rounded-xl bg-neutral-50 dark:bg-neutral-950/80 border border-neutral-200 dark:border-neutral-800 text-xs font-medium text-neutral-800 dark:text-neutral-200 cursor-pointer">
            <input
              type="checkbox"
              id="widget-toggle-payer-breakdown"
              checked={$dashboardWidgets.payerBreakdown !== false}
              on:change={(e) => dashboardWidgets.update(w => ({ ...w, payerBreakdown: e.target.checked }))}
              class="rounded text-indigo-600"
            />
            <span>👥 Payer Breakdown Cards</span>
          </label>

          <label class="flex items-center gap-2 p-2.5 rounded-xl bg-neutral-50 dark:bg-neutral-950/80 border border-neutral-200 dark:border-neutral-800 text-xs font-medium text-neutral-800 dark:text-neutral-200 cursor-pointer">
            <input
              type="checkbox"
              id="widget-toggle-category-chart"
              checked={$dashboardWidgets.categoryChart !== false}
              on:change={(e) => dashboardWidgets.update(w => ({ ...w, categoryChart: e.target.checked }))}
              class="rounded text-indigo-600"
            />
            <span>📊 Category Chart & List</span>
          </label>

          <label class="flex items-center gap-2 p-2.5 rounded-xl bg-neutral-50 dark:bg-neutral-950/80 border border-neutral-200 dark:border-neutral-800 text-xs font-medium text-neutral-800 dark:text-neutral-200 cursor-pointer">
            <input
              type="checkbox"
              id="widget-toggle-cash-flow"
              checked={$dashboardWidgets.cashFlowSavings !== false}
              on:change={(e) => dashboardWidgets.update(w => ({ ...w, cashFlowSavings: e.target.checked }))}
              class="rounded text-indigo-600"
            />
            <span>📈 Cash Flow & Savings Rate</span>
          </label>
        </div>
      </div>

      <!-- Category Chart Style -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-200 dark:border-neutral-800/60 pb-4">
        <div>
          <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">Dashboard Spending Chart</p>
          <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Switch between a doughnut chart and a bar chart on the dashboard.</p>
        </div>
        <div class="flex bg-neutral-100 dark:bg-neutral-950 rounded-lg p-0.5 border border-neutral-200 dark:border-neutral-800 flex-none">
          <button
            id="setting-chart-doughnut"
            on:click={() => chartStyle.set('doughnut')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$chartStyle === 'doughnut' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
          >
            Doughnut
          </button>
          <button
            id="setting-chart-bar"
            on:click={() => chartStyle.set('bar')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$chartStyle === 'bar' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
          >
            Bar Chart
          </button>
        </div>
      </div>

      <!-- Split Input Mode -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-neutral-200 dark:border-neutral-800/60 pb-4">
        <div>
          <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">Split Input Style</p>
          <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Slider links 2 users to 100% automatically; Inputs provide manual percentage fields.</p>
        </div>
        <div class="flex bg-neutral-100 dark:bg-neutral-950 rounded-lg p-0.5 border border-neutral-200 dark:border-neutral-800 flex-none">
          <button
            id="setting-split-inputs"
            on:click={() => splitInputMode.set('inputs')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$splitInputMode === 'inputs' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
          >
            Inputs
          </button>
          <button
            id="setting-split-slider"
            on:click={() => splitInputMode.set('slider')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$splitInputMode === 'slider' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
          >
            Slider
          </button>
        </div>
      </div>

      <!-- Payback Display Mode -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div>
          <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">Payback Debt Visualizer</p>
          <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Cards show individual user cards with net balances; Bars show proportional stacked bars.</p>
        </div>
        <div class="flex bg-neutral-100 dark:bg-neutral-950 rounded-lg p-0.5 border border-neutral-200 dark:border-neutral-800 flex-none">
          <button
            id="setting-payback-cards"
            on:click={() => paybackDisplayMode.set('cards')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$paybackDisplayMode === 'cards' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
          >
            Cards
          </button>
          <button
            id="setting-payback-bars"
            on:click={() => paybackDisplayMode.set('bar')}
            class="px-3 py-1.5 rounded-md text-xs font-semibold transition-all {$paybackDisplayMode === 'bar' ? 'bg-indigo-600 text-white shadow-sm' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
          >
            Bars
          </button>
        </div>
      </div>
    </div>
  </div>

  <!-- ── 7. Zero-Knowledge Security Vault ───────────────────────────────────── -->
  <div class="card space-y-5">
    <div class="border-b border-neutral-200 dark:border-neutral-800 pb-4">
      <h2 class="text-sm font-bold text-neutral-900 dark:text-white flex items-center gap-2">
        <span>🔒 Zero-Knowledge Security & Database Backup</span>
      </h2>
      <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-1">Client-side cryptographic status and unencrypted offline SQLite backup export.</p>
    </div>

    <div class="p-3.5 bg-neutral-50 dark:bg-neutral-950/80 border border-neutral-200 dark:border-neutral-800 rounded-xl flex items-center gap-3">
      <div class="w-8 h-8 rounded-lg bg-emerald-100 dark:bg-emerald-950 text-emerald-600 dark:text-emerald-400 border border-emerald-200 dark:border-emerald-800/60 flex items-center justify-center text-sm flex-none font-bold">
        ✓
      </div>
      <div class="min-w-0 flex-1">
        <p class="text-xs font-semibold text-neutral-800 dark:text-neutral-200">256-Bit AES-GCM Encryption Active</p>
        <p class="text-[11px] text-neutral-500 dark:text-neutral-400 mt-0.5">All sensitive transaction names, notes, and labels are encrypted in browser memory with PBKDF2 before storage.</p>
      </div>
    </div>

    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pt-2">
      <div>
        <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">Export Decrypted Database</p>
        <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Download a fully decrypted SQLite database file (`.db`) for personal backup or viewing in DBeaver.</p>
        {#if exportError}
          <p class="text-xs text-rose-500 dark:text-red-400 mt-2">{exportError}</p>
        {/if}
      </div>
      <button
        id="export-db-btn"
        on:click={handleExport}
        disabled={exporting}
        class="w-full sm:w-auto px-4 py-2 bg-neutral-100 hover:bg-neutral-200 dark:bg-neutral-800 dark:hover:bg-neutral-700 text-neutral-800 dark:text-neutral-200 text-xs font-semibold rounded-xl transition-colors border border-neutral-200 dark:border-neutral-700 disabled:opacity-50 whitespace-nowrap shadow-sm"
      >
        {exporting ? 'Decrypting & Exporting…' : 'Export .db File'}
      </button>
    </div>
  </div>

  <!-- ── 8. Documentation ────────────────────────────────────────────────── -->
  <div class="card space-y-5">
    <div class="border-b border-neutral-200 dark:border-neutral-800 pb-4">
      <h2 class="text-sm font-bold text-neutral-900 dark:text-white flex items-center gap-2">
        <span>Documentation</span>
      </h2>
      <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-1">Frontend, backend, and security architecture guides.</p>
    </div>

    <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-1">
      <a
        id="settings-link-frontend-docs"
        href="/docs/frontend"
        on:click|preventDefault={() => {
          if (typeof window !== 'undefined') {
            window.history.pushState({}, '', '/docs/frontend');
            window.dispatchEvent(new PopStateEvent('popstate'));
          }
        }}
        class="p-3.5 rounded-xl border border-indigo-200 dark:border-indigo-900/60 bg-indigo-50/50 dark:bg-indigo-950/20 hover:bg-indigo-100/60 dark:hover:bg-indigo-900/30 transition-colors flex items-center justify-between group cursor-pointer"
      >
        <div>
          <p class="text-xs font-bold text-indigo-900 dark:text-indigo-200">Frontend Technical Docs</p>
          <p class="text-[11px] text-indigo-700 dark:text-indigo-400 mt-0.5">Svelte, WebCrypto, Stores & Vitest</p>
        </div>
        <span class="text-xs font-semibold text-indigo-600 dark:text-indigo-400 group-hover:translate-x-0.5 transition-transform">→</span>
      </a>

      <a
        id="settings-link-docs-hub"
        href="/docs"
        on:click|preventDefault={() => {
          if (typeof window !== 'undefined') {
            window.history.pushState({}, '', '/docs');
            window.dispatchEvent(new PopStateEvent('popstate'));
          }
        }}
        class="p-3.5 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 hover:bg-neutral-50 dark:hover:bg-neutral-800/80 transition-colors flex items-center justify-between group cursor-pointer"
      >
        <div>
          <p class="text-xs font-bold text-neutral-900 dark:text-white">Documentation Hub</p>
          <p class="text-[11px] text-neutral-500 dark:text-neutral-400 mt-0.5">Frontend, Backend & Architecture</p>
        </div>
        <span class="text-xs font-semibold text-neutral-500 group-hover:translate-x-0.5 transition-transform">→</span>
      </a>
    </div>
  </div>

  <!-- ── 9. Start from Scratch (Danger Zone) ───────────────────────────────── -->
  <div class="card space-y-5 border-2 border-rose-300/80 dark:border-rose-900/60 bg-rose-50/20 dark:bg-rose-950/10">
    <div class="border-b border-rose-200 dark:border-rose-900/40 pb-4">
      <h2 class="text-sm font-bold text-rose-900 dark:text-rose-300 flex items-center gap-2">
        <span>Start from Scratch</span>
      </h2>
      <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-1">Permanently erase all household ledger data, categories, users, and reset the instance to initial setup.</p>
    </div>

    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
      <div>
        <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">Reset Application Database</p>
        <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">This action is irreversible. You will be prompted to re-enter your current master passphrase to confirm.</p>
      </div>
      <button
        id="start-from-scratch-btn"
        on:click={() => {
          resetConfirmSalt = '';
          resetError = '';
          showResetModal = true;
        }}
        class="w-full sm:w-auto px-4 py-2 bg-rose-600 hover:bg-rose-500 text-white text-xs font-semibold rounded-xl transition-colors shadow-sm whitespace-nowrap"
      >
        Start from Scratch
      </button>
    </div>
  </div>
</div>

<!-- ── Reset Confirmation Modal ────────────────────────────────────────────── -->
{#if showResetModal}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-fadeIn">
    <div class="card max-w-md w-full shadow-2xl space-y-5 border-2 border-rose-300 dark:border-rose-900 bg-white dark:bg-neutral-900">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-xl bg-rose-500/10 border border-rose-500/30 flex items-center justify-center text-rose-600 dark:text-rose-400 text-lg flex-none font-bold">
          ⚠️
        </div>
        <div>
          <h3 class="text-base font-bold text-neutral-900 dark:text-white">Confirm Reset to Scratch</h3>
          <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Permanent, irreversible data deletion</p>
        </div>
      </div>

      <p class="text-xs text-neutral-600 dark:text-neutral-300 leading-relaxed">
        This will permanently delete all expenses, users, categories, budgets, and settings. Enter your current master passphrase to confirm.
      </p>

      <form on:submit|preventDefault={handleResetConfirm} class="space-y-4">
        <div>
          <label for="reset-confirm-salt" class="block text-xs font-semibold uppercase tracking-wider text-neutral-700 dark:text-neutral-300 mb-1.5">
            Current Master Password
          </label>
          <input
            id="reset-confirm-salt"
            type="password"
            bind:value={resetConfirmSalt}
            placeholder="Type current master passphrase..."
            class="input-field py-2 text-sm"
            disabled={resetting}
            required
          />
        </div>

        {#if resetError}
          <div class="p-2.5 rounded-lg bg-rose-500/10 border border-rose-500/20 text-xs text-rose-600 dark:text-rose-400">
            {resetError}
          </div>
        {/if}

        <div class="flex flex-col sm:flex-row gap-2.5 pt-2">
          <button
            id="cancel-reset-btn"
            type="button"
            on:click={() => (showResetModal = false)}
            disabled={resetting}
            class="btn-secondary flex-1 text-center"
          >
            Cancel
          </button>
          <button
            id="confirm-reset-btn"
            type="submit"
            disabled={resetting}
            class="px-4 py-2 bg-rose-600 hover:bg-rose-500 text-white text-xs font-semibold rounded-xl transition-colors shadow-sm flex-1 text-center disabled:opacity-50"
          >
            {resetting ? 'Resetting…' : 'Confirm & Reset Database'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}
