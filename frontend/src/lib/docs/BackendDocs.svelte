<script>
  import { createEventDispatcher } from 'svelte';
  import { theme } from '../stores.js';

  const dispatch = createEventDispatcher();

  let activeSection = 'overview';
  let copiedSnippetId = null;

  const sections = [
    { id: 'overview', title: '1. Stack and Lifecycle' },
    { id: 'database', title: '2. SQLite Schema & WAL Mode' },
    { id: 'algorithms', title: '3. Algorithms (Largest Remainder & Graphs)' },
    { id: 'endpoints', title: '4. API Endpoints Catalog' },
    { id: 'websockets', title: '5. WebSockets & Real-Time Events' },
    { id: 'testing', title: '6. Pytest Test Suite' },
  ];

  function copyCode(id, text) {
    if (navigator.clipboard) {
      navigator.clipboard.writeText(text);
      copiedSnippetId = id;
      setTimeout(() => {
        if (copiedSnippetId === id) copiedSnippetId = null;
      }, 2000);
    }
  }

  function navigateTo(path) {
    dispatch('navigate', { path });
  }

  function scrollToSection(id) {
    activeSection = id;
    const el = document.getElementById(id);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
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
  <header class="sticky top-0 z-30 bg-white/85 dark:bg-neutral-900/85 backdrop-blur-md border-b border-neutral-200 dark:border-neutral-800">
    <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between gap-4">
      <div class="flex items-center gap-3">
        <button
          on:click={() => navigateTo('/docs')}
          class="flex items-center gap-2 text-xs font-semibold px-2.5 py-1.5 rounded-lg border border-neutral-200 dark:border-neutral-700 bg-neutral-100 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300 hover:bg-neutral-200 dark:hover:bg-neutral-700 transition-colors"
        >
          ← Docs Index
        </button>

        <div class="hidden sm:flex items-center gap-2 text-xs text-neutral-400">
          <span>/</span>
          <span class="font-semibold text-neutral-800 dark:text-neutral-200">Backend Technical Docs</span>
        </div>
      </div>

      <div class="flex items-center gap-3">
        <button
          on:click={() => navigateTo('/docs/frontend')}
          class="text-xs text-neutral-500 hover:text-neutral-900 dark:hover:text-white transition-colors hidden md:block"
        >
          Frontend Docs →
        </button>

        <button
          on:click={() => navigateTo('/')}
          class="text-xs font-medium px-3.5 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white transition-colors shadow-xs"
        >
          Open App
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

  <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
    <div class="flex gap-8">
      <!-- Sticky Sidebar Table of Contents -->
      <aside class="hidden lg:block w-64 flex-none sticky top-24 self-start space-y-4 max-h-[calc(100vh-7rem)] overflow-y-auto pr-2">
        <div class="p-3 bg-neutral-100 dark:bg-neutral-900 rounded-xl border border-neutral-200 dark:border-neutral-800">
          <p class="text-[11px] font-bold uppercase tracking-wider text-neutral-500 dark:text-neutral-400 mb-2 px-2">
            Contents
          </p>
          <nav class="space-y-0.5">
            {#each sections as sec}
              <button
                on:click={() => scrollToSection(sec.id)}
                class="w-full text-left text-xs px-2 py-1.5 rounded-lg transition-all flex items-center justify-between
                       {activeSection === sec.id
                         ? 'bg-indigo-600 text-white font-semibold'
                         : 'text-neutral-600 dark:text-neutral-400 hover:bg-neutral-200 dark:hover:bg-neutral-800 hover:text-neutral-900 dark:hover:text-white'}"
              >
                <span class="truncate">{sec.title}</span>
              </button>
            {/each}
          </nav>
        </div>

        <div class="p-3 bg-neutral-100 dark:bg-neutral-900 rounded-xl border border-neutral-200 dark:border-neutral-800 text-xs">
          <p class="font-bold text-neutral-800 dark:text-neutral-200 mb-1">Backend Test Status</p>
          <p class="text-neutral-600 dark:text-neutral-400 text-[11px]">
            328 Pytest tests passing in Docker.
          </p>
        </div>
      </aside>

      <!-- Main Documentation Content -->
      <main class="flex-1 min-w-0 space-y-10">
        <!-- Title Banner -->
        <div class="border-b border-neutral-200 dark:border-neutral-800 pb-5">
          <span class="text-xs font-semibold text-amber-600 dark:text-amber-400">Backend Reference</span>
          <h1 class="text-2xl sm:text-3xl font-extrabold tracking-tight text-neutral-900 dark:text-white mt-1">
            Backend Services & Database Architecture
          </h1>
          <p class="text-sm text-neutral-600 dark:text-neutral-400 mt-2 leading-relaxed">
            Technical reference for FastAPI routing, SQLite WAL persistence, integer cent arithmetic, Largest Remainder math, and graph debt reduction.
          </p>
        </div>

        <!-- Section 1: Overview & Stack -->
        <section id="overview" class="space-y-3 scroll-mt-24">
          <h2 class="text-lg font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span class="text-amber-500 font-mono">1.</span> Stack and Lifecycle
          </h2>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            The backend is built with Python 3.14, FastAPI, and SQLite via <code class="text-indigo-500">aiosqlite</code>. Request and response schemas are strictly validated using Pydantic v2.
          </p>

          <div class="overflow-x-auto rounded-xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900">
            <table class="w-full text-left text-xs">
              <thead class="bg-neutral-100 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300 uppercase font-semibold">
                <tr>
                  <th class="py-2.5 px-3">Subsystem</th>
                  <th class="py-2.5 px-3">Technology</th>
                  <th class="py-2.5 px-3">Role</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-neutral-200 dark:divide-neutral-800 text-neutral-600 dark:text-neutral-400">
                <tr>
                  <td class="py-2 px-3 font-semibold text-neutral-900 dark:text-white">Framework</td>
                  <td class="py-2 px-3">FastAPI</td>
                  <td class="py-2 px-3">Async REST routes, OpenAPI schemas, WebSockets</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-semibold text-neutral-900 dark:text-white">Database</td>
                  <td class="py-2 px-3">SQLite / aiosqlite</td>
                  <td class="py-2 px-3">Embedded WAL-mode database with foreign keys</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-semibold text-neutral-900 dark:text-white">Validation</td>
                  <td class="py-2 px-3">Pydantic v2</td>
                  <td class="py-2 px-3">Request models and payload constraints</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-semibold text-neutral-900 dark:text-white">Scheduler</td>
                  <td class="py-2 px-3">APScheduler</td>
                  <td class="py-2 px-3">Recurring expense occurrence generation</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-semibold text-neutral-900 dark:text-white">Crypto</td>
                  <td class="py-2 px-3">cryptography (Python)</td>
                  <td class="py-2 px-3">PBKDF2/AES-GCM for export and import verification</td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>

        <!-- Section 2: SQLite Schema & WAL Mode -->
        <section id="database" class="space-y-3 scroll-mt-24">
          <h2 class="text-lg font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span class="text-amber-500 font-mono">2.</span> SQLite Schema & WAL Mode
          </h2>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            Every database connection executes PRAGMA initialization for concurrency and data safety:
          </p>

          <div class="rounded-xl bg-neutral-900 text-neutral-100 p-4 font-mono text-xs overflow-x-auto relative border border-neutral-800">
            <pre><code>PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
PRAGMA synchronous = NORMAL;</code></pre>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs pt-1">
            <div class="p-3 rounded-xl bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 space-y-1">
              <p class="font-bold text-neutral-900 dark:text-white">No ORM (Native ANSI SQL)</p>
              <p class="text-neutral-600 dark:text-neutral-400">All queries write parameterized native SQL in database.py without ORM overhead.</p>
            </div>
            <div class="p-3 rounded-xl bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 space-y-1">
              <p class="font-bold text-neutral-900 dark:text-white">Integer Cent Currency</p>
              <p class="text-neutral-600 dark:text-neutral-400">Monetary amounts are stored as integer cents to eliminate floating-point rounding errors.</p>
            </div>
          </div>
        </section>

        <!-- Section 3: Algorithms -->
        <section id="algorithms" class="space-y-3 scroll-mt-24">
          <h2 class="text-lg font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span class="text-amber-500 font-mono">3.</span> Algorithms (Largest Remainder & Graphs)
          </h2>
          <div class="space-y-3 text-xs text-neutral-600 dark:text-neutral-400">
            <div class="p-4 rounded-xl bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 space-y-2">
              <h3 class="font-bold text-neutral-900 dark:text-white">Hare-Niemeyer Largest Remainder Distribution</h3>
              <p class="leading-relaxed">
                Calculates exact integer cent shares for split allocations. Uses <code class="text-indigo-500">math.floor()</code> for negative and positive amounts and resolves remainder ties deterministically via SHA-256 hashes of <code class="text-indigo-500">f"&#123;tx_salt&#125;:&#123;user&#125;"</code>.
              </p>
            </div>

            <div class="p-4 rounded-xl bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 space-y-2">
              <h3 class="font-bold text-neutral-900 dark:text-white">Connected Subgraph Debt Simplification</h3>
              <p class="leading-relaxed">
                Partitions household members into disjoint connected subgraphs based on transaction split participation before running greedy debt reduction, ensuring couples or tenant subgroups never cross debts.
              </p>
            </div>
          </div>
        </section>

        <!-- Section 4: Endpoints Catalog -->
        <section id="endpoints" class="space-y-3 scroll-mt-24">
          <h2 class="text-lg font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span class="text-amber-500 font-mono">4.</span> API Endpoints Catalog
          </h2>

          <div class="overflow-x-auto rounded-xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900">
            <table class="w-full text-left text-xs">
              <thead class="bg-neutral-100 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300 uppercase font-semibold">
                <tr>
                  <th class="py-2.5 px-3">Method</th>
                  <th class="py-2.5 px-3">Endpoint</th>
                  <th class="py-2.5 px-3">Payload / Query</th>
                  <th class="py-2.5 px-3">Description</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-neutral-200 dark:divide-neutral-800 text-neutral-600 dark:text-neutral-400">
                <tr>
                  <td class="py-2 px-3 font-mono font-bold text-emerald-600">GET</td>
                  <td class="py-2 px-3 font-mono">/users</td>
                  <td class="py-2 px-3">None</td>
                  <td class="py-2 px-3">List active household members</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-mono font-bold text-blue-600">POST</td>
                  <td class="py-2 px-3 font-mono">/users</td>
                  <td class="py-2 px-3">UserCreate</td>
                  <td class="py-2 px-3">Create a household member</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-mono font-bold text-blue-600">POST</td>
                  <td class="py-2 px-3 font-mono">/expenses</td>
                  <td class="py-2 px-3">ExpenseCreate</td>
                  <td class="py-2 px-3">Log an expense with split overrides</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-mono font-bold text-amber-600">PUT</td>
                  <td class="py-2 px-3 font-mono">/splits/&#123;cat&#125;</td>
                  <td class="py-2 px-3">SplitUpdate</td>
                  <td class="py-2 px-3">Update category percentage splits</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-mono font-bold text-blue-600">POST</td>
                  <td class="py-2 px-3 font-mono">/jobs</td>
                  <td class="py-2 px-3">JobCreate</td>
                  <td class="py-2 px-3">Create employment income stream</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-mono font-bold text-emerald-600">GET</td>
                  <td class="py-2 px-3 font-mono">/analytics/paybacks</td>
                  <td class="py-2 px-3">?month=YYYY-MM</td>
                  <td class="py-2 px-3">Simplified debt repayment matrix</td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>

        <!-- Section 5: WebSockets -->
        <section id="websockets" class="space-y-3 scroll-mt-24">
          <h2 class="text-lg font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span class="text-amber-500 font-mono">5.</span> WebSockets & Real-Time Events
          </h2>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            Connected clients listen on <code class="text-indigo-500">WS /ws/finance</code>. Broadcast events trigger reactive client store updates:
          </p>
          <div class="rounded-xl bg-neutral-900 text-neutral-100 p-4 font-mono text-xs overflow-x-auto border border-neutral-800">
            <pre><code>&#123;
  "event": "expense_created",
  "expense_id": 104,
  "month": "2026-08"
&#125;</code></pre>
          </div>
        </section>

        <!-- Section 6: Testing -->
        <section id="testing" class="space-y-3 scroll-mt-24 pb-8">
          <h2 class="text-lg font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span class="text-amber-500 font-mono">6.</span> Pytest Test Suite
          </h2>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            Execute the full backend test suite in Docker:
          </p>

          <div class="rounded-xl bg-neutral-900 text-neutral-100 p-4 font-mono text-xs overflow-x-auto relative border border-neutral-800">
            <div class="flex items-center justify-between pb-2 mb-2 border-b border-neutral-800 text-neutral-400 text-[11px]">
              <span>Pytest Docker command</span>
              <button
                on:click={() => copyCode('docker-backend-test', 'docker run --rm -v $(pwd)/backend/app:/app/app -v $(pwd)/backend/tests:/app/tests jizifin-backend-test pytest')}
                class="px-2 py-0.5 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-300 transition-colors"
              >
                {copiedSnippetId === 'docker-backend-test' ? '✓ Copied' : 'Copy'}
              </button>
            </div>
            <pre><code>docker run --rm \
  -v $(pwd)/backend/app:/app/app \
  -v $(pwd)/backend/tests:/app/tests \
  jizifin-backend-test pytest</code></pre>
          </div>
        </section>
      </main>
    </div>
  </div>
</div>
