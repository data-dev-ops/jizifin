<script>
  import { createEventDispatcher } from 'svelte';
  import { theme, currencySymbol } from '../stores.js';

  const dispatch = createEventDispatcher();

  let searchQuery = '';

  const options = [
    {
      id: 'getting-started',
      path: '/docs/getting-started',
      title: 'Getting Started Guide',
      badge: 'How-To',
      badgeColor: 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/20',
      icon: `<svg class="w-6 h-6 text-emerald-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75"><path stroke-linecap="round" stroke-linejoin="round" d="M12 6.042A8.967 8.967 0 006 3.75c-1.052 0-2.062.18-3 .512v14.25A8.987 8.987 0 016 18c2.305 0 4.408.867 6 2.292m0-14.25a8.966 8.966 0 016-2.292c1.052 0 2.062.18 3 .512v14.25A8.987 8.987 0 0018 18a8.967 8.967 0 00-6 2.292m0-14.25v14.25"/></svg>`,
      description: 'Step-by-step onboarding walkthrough covering passphrase setup, member colors, categories, joint accounts, income streams, and month settlements.',
      topics: ['Master Passphrase', 'Adding Members', 'Category Split Ratios', 'Joint Account Setup', 'Income & Contracts', 'Month Settlements'],
      actionLabel: 'Open Getting Started →',
      isPrimary: true,
    },
    {
      id: 'frontend',
      path: '/docs/frontend',
      title: 'Frontend Technical Docs',
      badge: 'Frontend',
      badgeColor: 'bg-indigo-500/10 text-indigo-600 dark:text-indigo-400 border-indigo-500/20',
      icon: `<svg class="w-6 h-6 text-indigo-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75"><path stroke-linecap="round" stroke-linejoin="round" d="M17.25 6.75L22.5 12l-5.25 5.25m-10.5 0L1.5 12l5.25-5.25m7.5-3l-4.5 16.5"/></svg>`,
      description: 'Svelte 4 components, reactive state in stores.js, WebCrypto PBKDF2/AES-GCM encryption, Chart.js canvases, and settings validation matrices.',
      topics: ['Directory Layout', 'WebCrypto AES-GCM', 'stores.js State', 'api.js and WebSockets', 'Settings Validation Matrix', 'Vitest Suite'],
      actionLabel: 'Open Frontend Docs →',
      isPrimary: false,
    },
    {
      id: 'backend',
      path: '/docs/backend',
      title: 'Backend Services & DB Engine',
      badge: 'Backend',
      badgeColor: 'bg-amber-500/10 text-amber-600 dark:text-amber-400 border-amber-500/20',
      icon: `<svg class="w-6 h-6 text-amber-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75"><path stroke-linecap="round" stroke-linejoin="round" d="M5.25 14.25h13.5m-13.5 0a3 3 0 01-3-3m3 3a3 3 0 100 6h13.5a3 3 0 100-6m-16.5-3a3 3 0 013-3h13.5a3 3 0 013 3m-19.5 0a4.5 4.5 0 01.9-2.7L5.7 7.4A3 3 0 018.1 6h7.8a3 3 0 012.4 1.4l1.8 2.85a4.5 4.5 0 01.9 2.75"/></svg>`,
      description: 'FastAPI async routing, SQLite in WAL mode, raw SQL queries, integer cent math, Largest Remainder allocations, and Pytest coverage.',
      topics: ['FastAPI & aiosqlite', 'SQL Views (No ORM)', 'Integer Cent Math', 'Hare-Niemeyer Splits', 'Debt Simplification', 'Pytest Suite'],
      actionLabel: 'Open Backend Docs →',
      isPrimary: false,
    },
    {
      id: 'architecture',
      path: '/docs/architecture',
      title: 'Architecture & Cryptography',
      badge: 'Security',
      badgeColor: 'bg-violet-500/10 text-violet-600 dark:text-violet-400 border-violet-500/20',
      icon: `<svg class="w-6 h-6 text-violet-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75"><path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75L11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285z"/></svg>`,
      description: 'Client-server encryption boundary, PBKDF2 parameters, deterministic AES-GCM static IV rules, Caddy routing, and WebSocket sync.',
      topics: ['Client-Server Split', 'PBKDF2 SHA-256 (100k iters)', 'Static IV Determinism', 'Multi-tenant Isolation', 'WebSocket Broadcaster', 'Docker Composition'],
      actionLabel: 'Open Architecture Docs →',
      isPrimary: false,
    },
  ];

  $: filteredOptions = options.filter(opt => {
    if (!searchQuery.trim()) return true;
    const q = searchQuery.toLowerCase();
    return (
      opt.title.toLowerCase().includes(q) ||
      opt.description.toLowerCase().includes(q) ||
      opt.topics.some(t => t.toLowerCase().includes(q))
    );
  });

  function navigateTo(path) {
    dispatch('navigate', { path });
  }

  function cycleTheme() {
    if ($theme === 'dark') {
      theme.set('light');
    } else if ($theme === 'light') {
      theme.set('system');
    } else {
      theme.set('dark');
    }
  }
</script>

<div class="min-h-screen bg-slate-50 dark:bg-neutral-950 text-neutral-900 dark:text-white font-inter">
  <!-- Top Navigation Bar -->
  <header class="sticky top-0 z-30 bg-white/80 dark:bg-neutral-900/80 backdrop-blur-md border-b border-neutral-200 dark:border-neutral-800">
    <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between gap-4">
      <div class="flex items-center gap-3">
        <button
          on:click={() => navigateTo('/')}
          class="flex items-center gap-2.5 hover:opacity-80 transition-opacity"
          aria-label="Return to Jizifin app"
        >
          <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-indigo-500 to-violet-600 flex items-center justify-center text-sm font-bold text-white shadow-md shadow-indigo-500/20">
            {$currencySymbol}
          </div>
          <span class="text-base font-bold tracking-tight text-neutral-900 dark:text-white">Jizifin</span>
        </button>
        <span class="text-xs px-2 py-0.5 rounded-md bg-neutral-100 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-300 font-medium border border-neutral-200 dark:border-neutral-700">
          Documentation
        </span>
      </div>

      <div class="flex items-center gap-3">
        <button
          on:click={() => navigateTo('/')}
          class="text-xs font-medium px-3 py-1.5 rounded-lg border border-neutral-200 dark:border-neutral-700 hover:bg-neutral-100 dark:hover:bg-neutral-800 text-neutral-700 dark:text-neutral-300 transition-colors"
        >
          Back to App
        </button>

        <button
          on:click={cycleTheme}
          aria-label="Toggle theme"
          class="w-8 h-8 flex items-center justify-center rounded-lg border border-neutral-200 dark:border-neutral-700 bg-neutral-100 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300 hover:bg-neutral-200 dark:hover:bg-neutral-700 transition-colors text-xs"
        >
          {#if $theme === 'dark'}🌙{:else if $theme === 'light'}☀️{:else}💻{/if}
        </button>
      </div>
    </div>
  </header>

  <!-- Hero Section -->
  <section class="pt-12 pb-8 border-b border-neutral-200/80 dark:border-neutral-800/80">
    <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
      <h1 class="text-2xl sm:text-3xl lg:text-4xl font-extrabold tracking-tight text-neutral-900 dark:text-white">
        Jizifin Documentation
      </h1>
      <p class="mt-3 max-w-xl mx-auto text-sm text-neutral-600 dark:text-neutral-400">
        Guides, architecture notes, cryptographic protocols, and technical references.
      </p>

      <!-- Search Input -->
      <div class="mt-6 max-w-md mx-auto relative">
        <div class="absolute inset-y-0 left-0 pl-3.5 flex items-center pointer-events-none text-neutral-400">
          <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
          </svg>
        </div>
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Filter documentation topics (e.g., crypto, svelte, wal, setup)..."
          class="w-full pl-10 pr-4 py-2 rounded-xl text-sm bg-white dark:bg-neutral-900 border border-neutral-300 dark:border-neutral-700 text-neutral-900 dark:text-white placeholder-neutral-400 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 shadow-sm transition-all"
        />
      </div>
    </div>
  </section>

  <!-- Main Grid: 4 Options -->
  <main class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-10">
    <div class="flex items-center justify-between mb-6">
      <h2 class="text-sm font-bold uppercase tracking-wider text-neutral-500 dark:text-neutral-400">Documentation Categories</h2>
      <span class="text-xs font-mono text-neutral-400">4 sections available</span>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-5">
      {#each filteredOptions as opt (opt.id)}
        <div
          class="flex flex-col rounded-2xl p-5 transition-all border bg-white dark:bg-neutral-900 hover:shadow-md
                 {opt.isPrimary
                   ? 'border-emerald-300 dark:border-emerald-800'
                   : 'border-neutral-200 dark:border-neutral-800'}"
        >
          <!-- Header with icon & badge -->
          <div class="flex items-start justify-between gap-4 mb-3">
            <div class="w-10 h-10 rounded-xl bg-neutral-100 dark:bg-neutral-800 flex items-center justify-center flex-none border border-neutral-200/80 dark:border-neutral-700/60">
              {@html opt.icon}
            </div>
            <span class="text-[11px] font-semibold px-2 py-0.5 rounded-md border {opt.badgeColor}">
              {opt.badge}
            </span>
          </div>

          <!-- Title & Description -->
          <h3 class="text-base font-bold text-neutral-900 dark:text-white mb-1.5">
            {opt.title}
          </h3>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed mb-4 flex-1">
            {opt.description}
          </p>

          <!-- Key Topics Tags -->
          <div class="space-y-1.5 mb-4 pt-3 border-t border-neutral-100 dark:border-neutral-800">
            <div class="flex flex-wrap gap-1">
              {#each opt.topics as topic}
                <span class="text-[10px] px-1.5 py-0.5 rounded bg-neutral-100 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-400">
                  {topic}
                </span>
              {/each}
            </div>
          </div>

          <!-- Navigation Action Button -->
          <button
            on:click={() => navigateTo(opt.path)}
            class="w-full py-2 px-3 rounded-lg text-xs font-semibold transition-all flex items-center justify-center gap-1.5 cursor-pointer
                   {opt.isPrimary
                     ? 'bg-emerald-600 hover:bg-emerald-500 text-white'
                     : 'bg-neutral-100 dark:bg-neutral-800 hover:bg-neutral-200 dark:hover:bg-neutral-700 text-neutral-800 dark:text-neutral-200 border border-neutral-200 dark:border-neutral-700'}"
          >
            {opt.actionLabel}
          </button>
        </div>
      {/each}
    </div>

    <!-- System Metrics Bar -->
    <div class="mt-8 rounded-xl bg-neutral-100/60 dark:bg-neutral-900/60 border border-neutral-200 dark:border-neutral-800 p-4">
      <div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-center">
        <div>
          <p class="text-lg font-bold text-neutral-900 dark:text-white">325</p>
          <p class="text-[11px] text-neutral-500 dark:text-neutral-400">Frontend Tests</p>
        </div>
        <div>
          <p class="text-lg font-bold text-neutral-900 dark:text-white">328</p>
          <p class="text-[11px] text-neutral-500 dark:text-neutral-400">Backend Tests</p>
        </div>
        <div>
          <p class="text-lg font-bold text-neutral-900 dark:text-white">100,000</p>
          <p class="text-[11px] text-neutral-500 dark:text-neutral-400">PBKDF2 Iterations</p>
        </div>
        <div>
          <p class="text-lg font-bold text-neutral-900 dark:text-white">20</p>
          <p class="text-[11px] text-neutral-500 dark:text-neutral-400">Database Tables</p>
        </div>
      </div>
    </div>
  </main>
</div>
