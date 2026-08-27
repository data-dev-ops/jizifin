<script>
  import { createEventDispatcher } from 'svelte';
  import { theme } from '../stores.js';

  const dispatch = createEventDispatcher();

  let activeSection = 'overview';
  let copiedSnippetId = null;

  const sections = [
    { id: 'overview', title: '1. Stack and Tooling' },
    { id: 'cryptography', title: '2. Cryptography (crypto.js)' },
    { id: 'stores', title: '3. Stores and State (stores.js)' },
    { id: 'api-layer', title: '4. API Client and WebSockets' },
    { id: 'components', title: '5. Component Tree' },
    { id: 'charts', title: '6. Chart.js Canvas Integration' },
    { id: 'theming', title: '7. Theming and Accessibility' },
    { id: 'validation', title: '8. Settings Validation Matrix' },
    { id: 'testing', title: '9. Testing Suite (Vitest + JSDOM)' },
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
          <span class="font-semibold text-neutral-800 dark:text-neutral-200">Frontend Technical Docs</span>
        </div>
      </div>

      <div class="flex items-center gap-3">
        <button
          on:click={() => navigateTo('/docs/backend')}
          class="text-xs text-neutral-500 hover:text-neutral-900 dark:hover:text-white transition-colors hidden md:block"
        >
          Backend Docs →
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
      <aside class="hidden lg:block w-60 flex-none sticky top-24 self-start space-y-4 max-h-[calc(100vh-7rem)] overflow-y-auto pr-2">
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
          <p class="font-bold text-neutral-800 dark:text-neutral-200 mb-1">Test Status</p>
          <p class="text-neutral-600 dark:text-neutral-400 text-[11px]">
            37 Vitest test files passing in Docker.
          </p>
        </div>
      </aside>

      <!-- Main Documentation Content -->
      <main class="flex-1 min-w-0 space-y-10">
        <!-- Title Banner -->
        <div class="border-b border-neutral-200 dark:border-neutral-800 pb-5">
          <span class="text-xs font-semibold text-indigo-600 dark:text-indigo-400">Frontend Reference</span>
          <h1 class="text-2xl sm:text-3xl font-extrabold tracking-tight text-neutral-900 dark:text-white mt-1">
            Frontend Architecture & Developer Guide
          </h1>
          <p class="text-sm text-neutral-600 dark:text-neutral-400 mt-2 leading-relaxed">
            Technical specifications for Svelte components, WebCrypto PBKDF2 and AES-GCM encryption, reactive stores, and settings validation matrices.
          </p>
        </div>

        <!-- Section 1: Overview -->
        <section id="overview" class="space-y-3 scroll-mt-24">
          <h2 class="text-lg font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span class="text-indigo-500 font-mono">1.</span> Stack and Tooling
          </h2>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            The frontend is an SPA built with Svelte 4, Tailwind CSS, and Chart.js. Financial data is held in decrypted Svelte stores in browser memory during the session.
          </p>

          <div class="overflow-x-auto rounded-xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900">
            <table class="w-full text-left text-xs">
              <thead class="bg-neutral-100 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300 uppercase font-semibold">
                <tr>
                  <th class="py-2.5 px-3">Layer</th>
                  <th class="py-2.5 px-3">Technology</th>
                  <th class="py-2.5 px-3">Role</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-neutral-200 dark:divide-neutral-800 text-neutral-600 dark:text-neutral-400">
                <tr>
                  <td class="py-2 px-3 font-semibold text-neutral-900 dark:text-white">UI</td>
                  <td class="py-2 px-3">Svelte 4</td>
                  <td class="py-2 px-3">Components and reactive stores</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-semibold text-neutral-900 dark:text-white">Styling</td>
                  <td class="py-2 px-3">Tailwind CSS</td>
                  <td class="py-2 px-3">Layouts, dark mode, high contrast</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-semibold text-neutral-900 dark:text-white">Charts</td>
                  <td class="py-2 px-3">Chart.js</td>
                  <td class="py-2 px-3">Canvas charts for expenses and income</td>
                </tr>
                <tr>
                  <td class="py-2 px-3 font-semibold text-neutral-900 dark:text-white">Crypto</td>
                  <td class="py-2 px-3">Web Crypto API</td>
                  <td class="py-2 px-3">PBKDF2 and AES-GCM encryption</td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>

        <!-- Section 2: Cryptography -->
        <section id="cryptography" class="space-y-3 scroll-mt-24">
          <h2 class="text-lg font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span class="text-indigo-500 font-mono">2.</span> Cryptography (crypto.js)
          </h2>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            Sensitive text fields (usernames, category names, descriptions, notes) are encrypted with AES-GCM using a static 12-byte IV (<code class="bg-neutral-100 dark:bg-neutral-800 px-1 py-0.5 rounded text-indigo-500">"jizifin-cryp"</code>).
          </p>

          <div class="rounded-xl bg-neutral-900 text-neutral-100 p-4 font-mono text-xs overflow-x-auto relative border border-neutral-800">
            <div class="flex items-center justify-between pb-2 mb-2 border-b border-neutral-800 text-neutral-400 text-[11px]">
              <span>src/lib/crypto.js</span>
              <button
                on:click={() => copyCode('crypto-snippet', `const STATIC_SALT = new TextEncoder().encode('jizifin-salt-pbkdf2');\nconst STATIC_IV = new Uint8Array([106, 105, 122, 105, 102, 105, 110, 45, 99, 114, 121, 112]); // "jizifin-cryp"\n\nexport async function deriveKey(passphrase) {\n  const enc = new TextEncoder();\n  const keyMaterial = await crypto.subtle.importKey('raw', enc.encode(passphrase), { name: 'PBKDF2' }, false, ['deriveKey']);\n  return crypto.subtle.deriveKey(\n    { name: 'PBKDF2', salt: STATIC_SALT, iterations: 100000, hash: 'SHA-256' },\n    keyMaterial,\n    { name: 'AES-GCM', length: 256 },\n    false,\n    ['encrypt', 'decrypt']\n  );\n}`)}
                class="px-2 py-0.5 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-300 transition-colors"
              >
                {copiedSnippetId === 'crypto-snippet' ? '✓ Copied' : 'Copy'}
              </button>
            </div>
            <pre><code>const STATIC_SALT = new TextEncoder().encode('jizifin-salt-pbkdf2');
const STATIC_IV = new Uint8Array([106, 105, 122, 105, 102, 105, 110, 45, 99, 114, 121, 112]);

export async function deriveKey(passphrase) &#123;
  const enc = new TextEncoder();
  const keyMaterial = await crypto.subtle.importKey(
    'raw', enc.encode(passphrase), &#123; name: 'PBKDF2' &#125;, false, ['deriveKey']
  );
  return crypto.subtle.deriveKey(
    &#123; name: 'PBKDF2', salt: STATIC_SALT, iterations: 100000, hash: 'SHA-256' &#125;,
    keyMaterial,
    &#123; name: 'AES-GCM', length: 256 &#125;,
    false,
    ['encrypt', 'decrypt']
  );
&#125;</code></pre>
          </div>
        </section>

        <!-- Section 3: Stores -->
        <section id="stores" class="space-y-3 scroll-mt-24">
          <h2 class="text-lg font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span class="text-indigo-500 font-mono">3.</span> Stores and State (stores.js)
          </h2>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            State is held in Svelte writable stores in <code class="bg-neutral-100 dark:bg-neutral-800 px-1 py-0.5 rounded text-indigo-500">src/lib/stores.js</code>. Decrypted data stays in memory only.
          </p>
        </section>

        <!-- Section 4: API Layer -->
        <section id="api-layer" class="space-y-3 scroll-mt-24">
          <h2 class="text-lg font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span class="text-indigo-500 font-mono">4.</span> API Client and WebSockets
          </h2>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            All API calls pass through `request()` in `api.js`. Non-2xx responses throw an Error with the server detail message.
          </p>
        </section>

        <!-- Section 5: Component Tree -->
        <section id="components" class="space-y-3 scroll-mt-24">
          <h2 class="text-lg font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span class="text-indigo-500 font-mono">5.</span> Component Tree
          </h2>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs">
            <div class="p-3 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 space-y-1">
              <p class="font-bold text-neutral-900 dark:text-white">ExpenseForm & ExpenseList</p>
              <p class="text-neutral-500">Split overrides, tag date boundary checks, delete confirmation.</p>
            </div>
            <div class="p-3 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 space-y-1">
              <p class="font-bold text-neutral-900 dark:text-white">AnalyticsSummary & PaybackVisual</p>
              <p class="text-neutral-500">Debt balances, spending breakdown, month settlement locks.</p>
            </div>
          </div>
        </section>

        <!-- Section 6: Chart.js -->
        <section id="charts" class="space-y-3 scroll-mt-24">
          <h2 class="text-lg font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span class="text-indigo-500 font-mono">6.</span> Chart.js Canvas Integration
          </h2>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            Charts mount on HTML5 <code class="bg-neutral-100 dark:bg-neutral-800 px-1 py-0.5 rounded text-indigo-500">&lt;canvas&gt;</code> elements and update with `chart.update()`.
          </p>
        </section>

        <!-- Section 7: Theming -->
        <section id="theming" class="space-y-3 scroll-mt-24">
          <h2 class="text-lg font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span class="text-indigo-500 font-mono">7.</span> Theming and Accessibility
          </h2>
          <ul class="list-disc list-inside text-xs text-neutral-600 dark:text-neutral-400 space-y-1">
            <li><strong>Dark Mode:</strong> Controlled by `.dark` on `document.documentElement`.</li>
            <li><strong>Privacy Shield:</strong> Replaces numbers with bullets to prevent shoulder-surfing.</li>
            <li><strong>High Contrast:</strong> 2px borders with high-contrast text tokens.</li>
            <li><strong>Text Scaling:</strong> Root scaling with `.text-scale-110` and `.text-scale-125`.</li>
          </ul>
        </section>

        <!-- Section 8: Settings Validation Matrix -->
        <section id="validation" class="space-y-4 scroll-mt-24">
          <h2 class="text-lg font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span class="text-indigo-500 font-mono">8.</span> Settings Validation Matrix (Valid vs. Invalid)
          </h2>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            Reference table of valid and invalid settings configurations across forms and API endpoints:
          </p>

          <div class="space-y-4 text-xs">
            <!-- Split Allocations -->
            <div class="p-4 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 space-y-2">
              <h3 class="font-bold text-neutral-900 dark:text-white">1. Split Allocations (Sum to 100.0%)</h3>
              <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                <div class="p-2.5 rounded-lg bg-emerald-500/10 border border-emerald-500/20 font-mono text-[11px]">
                  <p class="text-emerald-700 dark:text-emerald-400 font-bold mb-1">VALID: 60% / 40% (Sum: 100%)</p>
                  <pre class="text-neutral-700 dark:text-neutral-300"><code>&#123;
  "allocations": [
    &#123; "user_name": "Alice", "pct": 60.0 &#125;,
    &#123; "user_name": "Bob", "pct": 40.0 &#125;
  ]
&#125;</code></pre>
                </div>
                <div class="p-2.5 rounded-lg bg-rose-500/10 border border-rose-500/20 font-mono text-[11px]">
                  <p class="text-rose-700 dark:text-rose-400 font-bold mb-1">INVALID: Sum: 90% (HTTP 422)</p>
                  <pre class="text-neutral-700 dark:text-neutral-300"><code>&#123;
  "allocations": [
    &#123; "user_name": "Alice", "pct": 50.0 &#125;,
    &#123; "user_name": "Bob", "pct": 40.0 &#125;
  ]
&#125;</code></pre>
                </div>
              </div>
            </div>

            <!-- Tag Date Boundaries -->
            <div class="p-4 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 space-y-2">
              <h3 class="font-bold text-neutral-900 dark:text-white">2. Tag Date Boundaries (start_date &le; end_date)</h3>
              <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                <div class="p-2.5 rounded-lg bg-emerald-500/10 border border-emerald-500/20 font-mono text-[11px]">
                  <p class="text-emerald-700 dark:text-emerald-400 font-bold mb-1">VALID: Chronological</p>
                  <pre class="text-neutral-700 dark:text-neutral-300"><code>&#123;
  "start_date": "2026-07-01",
  "end_date": "2026-07-15"
&#125;</code></pre>
                </div>
                <div class="p-2.5 rounded-lg bg-rose-500/10 border border-rose-500/20 font-mono text-[11px]">
                  <p class="text-rose-700 dark:text-rose-400 font-bold mb-1">INVALID: End before start (HTTP 422)</p>
                  <pre class="text-neutral-700 dark:text-neutral-300"><code>&#123;
  "start_date": "2026-07-15",
  "end_date": "2026-07-01"
&#125;</code></pre>
                </div>
              </div>
            </div>

            <!-- Joint Account Settings -->
            <div class="p-4 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 space-y-2">
              <h3 class="font-bold text-neutral-900 dark:text-white">3. Joint Account Settings (Margin 0..100)</h3>
              <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                <div class="p-2.5 rounded-lg bg-emerald-500/10 border border-emerald-500/20 font-mono text-[11px]">
                  <p class="text-emerald-700 dark:text-emerald-400 font-bold mb-1">VALID: 10% Margin & Salary Mode</p>
                  <pre class="text-neutral-700 dark:text-neutral-300"><code>&#123;
  "safety_margin_pct": 10,
  "deposit_split_mode": "salary"
&#125;</code></pre>
                </div>
                <div class="p-2.5 rounded-lg bg-rose-500/10 border border-rose-500/20 font-mono text-[11px]">
                  <p class="text-rose-700 dark:text-rose-400 font-bold mb-1">INVALID: Margin 150 (HTTP 422)</p>
                  <pre class="text-neutral-700 dark:text-neutral-300"><code>&#123;
  "safety_margin_pct": 150,
  "deposit_split_mode": "salary"
&#125;</code></pre>
                </div>
              </div>
            </div>
          </div>
        </section>

        <!-- Section 9: Testing -->
        <section id="testing" class="space-y-3 scroll-mt-24 pb-10">
          <h2 class="text-lg font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span class="text-indigo-500 font-mono">9.</span> Testing Suite (Vitest + JSDOM)
          </h2>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            Run the frontend test suite in Docker:
          </p>

          <div class="rounded-xl bg-neutral-900 text-neutral-100 p-4 font-mono text-xs overflow-x-auto relative border border-neutral-800">
            <div class="flex items-center justify-between pb-2 mb-2 border-b border-neutral-800 text-neutral-400 text-[11px]">
              <span>Docker command</span>
              <button
                on:click={() => copyCode('docker-test', 'docker run --rm -v $(pwd)/frontend/src:/app/src -v $(pwd)/frontend/index.html:/app/index.html -v $(pwd)/frontend/tailwind.config.js:/app/tailwind.config.js -v $(pwd)/frontend/vite.config.js:/app/vite.config.js jizifin-frontend-test npm test')}
                class="px-2 py-0.5 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-300 transition-colors"
              >
                {copiedSnippetId === 'docker-test' ? '✓ Copied' : 'Copy'}
              </button>
            </div>
            <pre><code>docker run --rm \
  -v $(pwd)/frontend/src:/app/src \
  -v $(pwd)/frontend/index.html:/app/index.html \
  -v $(pwd)/frontend/tailwind.config.js:/app/tailwind.config.js \
  -v $(pwd)/frontend/vite.config.js:/app/vite.config.js \
  jizifin-frontend-test npm test</code></pre>
          </div>
        </section>
      </main>
    </div>
  </div>
</div>
