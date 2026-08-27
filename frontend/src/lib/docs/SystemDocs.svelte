<script>
  import { createEventDispatcher } from 'svelte';
  import { theme } from '../stores.js';

  const dispatch = createEventDispatcher();

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
          <span class="font-semibold text-neutral-800 dark:text-neutral-200">Architecture & Cryptography</span>
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

  <main class="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-10 space-y-6">
    <div class="border-b border-neutral-200 dark:border-neutral-800 pb-5">
      <span class="text-xs font-semibold text-violet-600 dark:text-violet-400">Security Reference</span>
      <h1 class="text-2xl sm:text-3xl font-extrabold tracking-tight text-neutral-900 dark:text-white mt-1">
        Architecture & Cryptography
      </h1>
      <p class="text-xs sm:text-sm text-neutral-600 dark:text-neutral-400 mt-2 leading-relaxed">
        Cryptographic parameters, key derivation, and client-server boundaries.
      </p>
    </div>

    <!-- Security Model Summary -->
    <div class="space-y-3 text-xs">
      <h2 class="text-base font-bold text-neutral-900 dark:text-white">Client-Server Boundary</h2>
      <p class="text-neutral-600 dark:text-neutral-400 leading-relaxed">
        Sensitive text is encrypted in the browser using AES-GCM before sending API requests. The backend receives and stores ciphertext only.
      </p>

      <div class="p-4 rounded-xl bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 font-mono text-[11px] text-neutral-800 dark:text-neutral-200 overflow-x-auto">
        <pre><code>Client (Browser)                                        Server (FastAPI + SQLite)
+-----------------------+                              +-----------------------+
| Plaintext String      |                              |                       |
|         ↓             |                              |                       |
| PBKDF2 (100k iters)   |                              |                       |
|         ↓             |                              |                       |
| AES-GCM (Static IV)   | --- Base64URL Ciphertext --->| SQLite DB (WAL Mode)  |
|         ↓             |                              | • Exact matches       |
| Display in Svelte UI  | &lt;--- Base64URL Ciphertext ---| • GROUP BY / FK checks|
+-----------------------+                              +-----------------------+</code></pre>
      </div>
    </div>

    <!-- Cryptographic Parameters Table -->
    <div class="space-y-3 text-xs">
      <h2 class="text-base font-bold text-neutral-900 dark:text-white">Cryptographic Parameters</h2>
      <div class="overflow-x-auto rounded-xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900">
        <table class="w-full text-left">
          <thead class="bg-neutral-100 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300 uppercase font-semibold">
            <tr>
              <th class="py-2.5 px-3">Parameter</th>
              <th class="py-2.5 px-3">Value</th>
              <th class="py-2.5 px-3">Description</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-neutral-200 dark:divide-neutral-800 text-neutral-600 dark:text-neutral-400">
            <tr>
              <td class="py-2 px-3 font-semibold text-neutral-900 dark:text-white">Algorithm</td>
              <td class="py-2 px-3 font-mono">AES-GCM (256-bit)</td>
              <td class="py-2 px-3">Authenticated symmetric cipher</td>
            </tr>
            <tr>
              <td class="py-2 px-3 font-semibold text-neutral-900 dark:text-white">Key Derivation</td>
              <td class="py-2 px-3 font-mono">PBKDF2-HMAC-SHA-256</td>
              <td class="py-2 px-3">100,000 iterations</td>
            </tr>
            <tr>
              <td class="py-2 px-3 font-semibold text-neutral-900 dark:text-white">PBKDF2 Salt</td>
              <td class="py-2 px-3 font-mono">"jizifin-salt-pbkdf2"</td>
              <td class="py-2 px-3">Static salt</td>
            </tr>
            <tr>
              <td class="py-2 px-3 font-semibold text-neutral-900 dark:text-white">AES-GCM IV</td>
              <td class="py-2 px-3 font-mono">"jizifin-cryp" (12 bytes)</td>
              <td class="py-2 px-3">Static IV for deterministic matching in SQLite</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Navigation Footer -->
    <div class="flex items-center justify-between pt-4 border-t border-neutral-200 dark:border-neutral-800">
      <button
        on:click={() => navigateTo('/docs')}
        class="text-xs font-semibold px-3 py-1.5 rounded-lg border border-neutral-200 dark:border-neutral-700 hover:bg-neutral-100 dark:hover:bg-neutral-800 text-neutral-700 dark:text-neutral-300 transition-colors"
      >
        ← Back to Index
      </button>

      <button
        on:click={() => navigateTo('/docs/frontend')}
        class="text-xs font-semibold px-3 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white transition-colors"
      >
        Frontend Docs →
      </button>
    </div>
  </main>
</div>
