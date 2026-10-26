<script>
  import { createEventDispatcher } from 'svelte';
  import { theme } from '../stores.js';

  const dispatch = createEventDispatcher();

  let activeSection = 'overview';
  let copiedSnippetId = null;

  const sections = [
    { id: 'overview', title: '1. Stack and Lifecycle' },
    { id: 'database', title: '2. DuckDB Schema & Single Writer' },
    { id: 'algorithms', title: '3. The Auditor (Largest Remainder & Graphs)' },
    { id: 'endpoints', title: '4. API Endpoints Catalog' },
    { id: 'websockets', title: '5. WebSockets & Real-Time Events' },
    { id: 'testing', title: '6. Verification & Tests' },
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
            Clippy is the gate today; the Rust test suite is still to be written.
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
            Technical reference for the Rust API, embedded DuckDB, the in-process Auditor, integer cent arithmetic, Largest Remainder math, and graph debt reduction.
          </p>
        </div>

        <!-- Section 1: Overview & Stack -->
        <section id="overview" class="space-y-3 scroll-mt-24">
          <h2 class="text-lg font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span class="text-amber-500 font-mono">1.</span> Stack and Lifecycle
          </h2>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            The backend is a Rust workspace (axum on tokio) over an embedded DuckDB 2.0 file. Inputs are validated by typed domain values (<code class="text-indigo-500">Cents</code>, <code class="text-indigo-500">Month</code>, <code class="text-indigo-500">Ciphertext</code>, typed ids). Clippy enforces small methods: at most 20 lines, 4 parameters and low cognitive complexity.
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
                  <td class="py-2 px-3">axum / tokio</td>
                  <td class="py-2 px-3">REST routes, bearer auth, WebSocket fan-out on /ws/finance</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-semibold text-neutral-900 dark:text-white">Database</td>
                  <td class="py-2 px-3">DuckDB 2.0 (embedded)</td>
                  <td class="py-2 px-3">Single file, pooled readers, one serialised writer</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-semibold text-neutral-900 dark:text-white">Validation</td>
                  <td class="py-2 px-3">serde + domain types</td>
                  <td class="py-2 px-3">Typed values and validate() on input structs</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-semibold text-neutral-900 dark:text-white">Scheduler</td>
                  <td class="py-2 px-3">tokio loop</td>
                  <td class="py-2 px-3">Daily recurring catch-up and idle-session sweep</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-semibold text-neutral-900 dark:text-white">Auditor</td>
                  <td class="py-2 px-3">jizifin-auditor</td>
                  <td class="py-2 px-3">Settlements, deposits, budgets, income, ratios, cash flow, project equity</td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>

        <!-- Section 2: DuckDB Schema & Single Writer -->
        <section id="database" class="space-y-3 scroll-mt-24">
          <h2 class="text-lg font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span class="text-amber-500 font-mono">2.</span> DuckDB Schema & Single Writer
          </h2>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            Schema changes are numbered SQL files applied at start-up. Writes are serialised through one lock, because DuckDB aborts overlapping updates of the same row; reads use a connection pool:
          </p>

          <div class="rounded-xl bg-neutral-900 text-neutral-100 p-4 font-mono text-xs overflow-x-auto relative border border-neutral-800">
            <pre><code>migrations/0001_init.sql   -- tables, sequences, indexes
Database::write(...)       -- exclusive lock, one transaction, bumps the data version
Database::read(...)        -- pooled connection, no lock</code></pre>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs pt-1">
            <div class="p-3 rounded-xl bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 space-y-1">
              <p class="font-bold text-neutral-900 dark:text-white">No ORM (Native ANSI SQL)</p>
              <p class="text-neutral-600 dark:text-neutral-400">Repositories write parameterized native DuckDB SQL. There are no foreign keys; repositories enforce integrity.</p>
            </div>
            <div class="p-3 rounded-xl bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 space-y-1">
              <p class="font-bold text-neutral-900 dark:text-white">Integer Cent Currency</p>
              <p class="text-neutral-600 dark:text-neutral-400">Monetary amounts are stored as BIGINT cents to eliminate floating-point rounding errors.</p>
            </div>
          </div>
        </section>

        <!-- Section 3: Algorithms -->
        <section id="algorithms" class="space-y-3 scroll-mt-24">
          <h2 class="text-lg font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span class="text-amber-500 font-mono">3.</span> The Auditor (Largest Remainder & Graphs)
          </h2>
          <div class="space-y-3 text-xs text-neutral-600 dark:text-neutral-400">
            <div class="p-4 rounded-xl bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 space-y-2">
              <h3 class="font-bold text-neutral-900 dark:text-white">Hare-Niemeyer Largest Remainder Distribution</h3>
              <p class="leading-relaxed">
                Calculates exact integer cent shares for split allocations. Uses a signed floor for negative and positive amounts and resolves remainder ties deterministically via the SHA-256 of <code class="text-indigo-500">"&#123;seed&#125;:&#123;member_id&#125;"</code>, where the seed is the entry id. Every audited result is checked against invariants and returned with an input hash; a failed invariant is an HTTP 500, never a wrong number.
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
                  <td class="py-2 px-3 font-mono">/members</td>
                  <td class="py-2 px-3">None</td>
                  <td class="py-2 px-3">List household members</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-mono font-bold text-blue-600">POST</td>
                  <td class="py-2 px-3 font-mono">/entries</td>
                  <td class="py-2 px-3">EntryInput</td>
                  <td class="py-2 px-3">Log an expense, income, deposit or correction</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-mono font-bold text-amber-600">PUT</td>
                  <td class="py-2 px-3 font-mono">/categories/&#123;id&#125;/baseline</td>
                  <td class="py-2 px-3">shares</td>
                  <td class="py-2 px-3">Replace a category's open-ended split</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-mono font-bold text-blue-600">POST</td>
                  <td class="py-2 px-3 font-mono">/jobs</td>
                  <td class="py-2 px-3">JobInput</td>
                  <td class="py-2 px-3">Create an employment income stream</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-mono font-bold text-emerald-600">GET</td>
                  <td class="py-2 px-3 font-mono">/settlements/&#123;month&#125;</td>
                  <td class="py-2 px-3">?members=&amp;explain=1</td>
                  <td class="py-2 px-3">Audited settlement, or the sealed result with drift</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-mono font-bold text-emerald-600">GET</td>
                  <td class="py-2 px-3 font-mono">/views/month</td>
                  <td class="py-2 px-3">?month=YYYY-MM</td>
                  <td class="py-2 px-3">Plain aggregates for dashboards</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-mono font-bold text-emerald-600">GET</td>
                  <td class="py-2 px-3 font-mono">/accounts/&#123;id&#125;/deposit-proposal</td>
                  <td class="py-2 px-3">?month=&amp;round_to=</td>
                  <td class="py-2 px-3">Audited joint-account deposit proposal</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-mono font-bold text-emerald-600">GET</td>
                  <td class="py-2 px-3 font-mono">/entries/&#123;id&#125;/shares</td>
                  <td class="py-2 px-3">None</td>
                  <td class="py-2 px-3">Who bears what for one expense</td>
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
            Connected clients listen on <code class="text-indigo-500">WS /ws/finance</code>. Every successful write publishes <code class="text-indigo-500">&lt;table&gt;.&lt;action&gt;</code>; the client bumps its ledger version and refetches what it shows:
          </p>
          <div class="rounded-xl bg-neutral-900 text-neutral-100 p-4 font-mono text-xs overflow-x-auto border border-neutral-800">
            <pre><code>&#123;
  "event": "entries.created"
&#125;</code></pre>
          </div>
        </section>

        <!-- Section 6: Testing -->
        <section id="testing" class="space-y-3 scroll-mt-24 pb-8">
          <h2 class="text-lg font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span class="text-amber-500 font-mono">6.</span> Verification & Tests
          </h2>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            Lint and type-check the whole workspace in Docker (the Rust test suite is still to be written):
          </p>

          <div class="rounded-xl bg-neutral-900 text-neutral-100 p-4 font-mono text-xs overflow-x-auto relative border border-neutral-800">
            <div class="flex items-center justify-between pb-2 mb-2 border-b border-neutral-800 text-neutral-400 text-[11px]">
              <span>Clippy via make</span>
              <button
                on:click={() => copyCode('docker-backend-test', 'make lint-backend')}
                class="px-2 py-0.5 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-300 transition-colors"
              >
                {copiedSnippetId === 'docker-backend-test' ? '✓ Copied' : 'Copy'}
              </button>
            </div>
            <pre><code>make lint-backend</code></pre>
          </div>
        </section>
      </main>
    </div>
  </div>
</div>
