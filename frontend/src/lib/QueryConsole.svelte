<script>
  /**
   * QueryConsole.svelte
   *
   * Raw SQL query interface. Sends the query to POST /query,
   * renders results in a scrollable table (max 50 rows) or
   * displays the error message inline if the request fails.
   */


  const BASE_URL = `http://${window.location.hostname}:8000`;

  let sql = '';
  let loading = false;
  let error = null;
  let result = null; // { columns, rows, row_count, truncated }

  const DB_OBJECTS_SQL =
    `SELECT name, type, sql FROM sqlite_master WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' ORDER BY name`;

  const EXAMPLES = [
    'SELECT * FROM expenses ORDER BY expense_date DESC LIMIT 10',
    'SELECT * FROM splits ORDER BY category',
    'SELECT * FROM split_allocations ORDER BY category, user_name',
    'SELECT * FROM users ORDER BY name',
    'SELECT * FROM view_monthly_total',
    'SELECT * FROM view_monthly_by_category',
    "SELECT name, cost_cents / 100.0 AS euros, expense_date, who_paid, category FROM expenses WHERE strftime('%Y-%m', expense_date) = strftime('%Y-%m', 'now') ORDER BY cost_cents DESC",
  ];

  /** Named presets for quick access to user-related queries. */
  const USER_PRESETS = [
    {
      label: 'Deactivated users',
      sql:   'SELECT name, color, created_at FROM users WHERE is_active = 0 ORDER BY name',
    },
    {
      label: 'Expenses by deactivated users',
      sql:   "SELECT e.id, e.name, e.cost_cents / 100.0 AS euros, e.expense_date, e.who_paid, e.category\nFROM expenses e\nWHERE e.who_paid IN (SELECT name FROM users WHERE is_active = 0)\nORDER BY e.expense_date DESC LIMIT 50",
    },
    {
      label: 'All users + totals',
      sql:   "SELECT u.name, u.color, u.is_active,\n  COUNT(e.id) AS expense_count,\n  ROUND(COALESCE(SUM(e.cost_cents),0)/100.0,2) AS total_euros\nFROM users u\nLEFT JOIN expenses e ON e.who_paid = u.name\nGROUP BY u.name ORDER BY total_euros DESC",
    },
  ];

  async function runQuery() {
    if (!sql.trim()) return;
    loading = true;
    error = null;
    result = null;

    try {
      const res = await fetch(`${BASE_URL}/query`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ sql: sql.trim() }),
      });

      if (!res.ok) {
        const body = await res.json().catch(() => ({ detail: res.statusText }));
        error = body.detail ?? 'Unknown error';
      } else {
        result = await res.json();
      }
    } catch (e) {
      error = e.message ?? 'Network error';
    } finally {
      loading = false;
    }
  }

  function handleKeydown(e) {
    // Ctrl+Enter or Cmd+Enter to run
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      runQuery();
    }
    // Tab inserts spaces instead of leaving the textarea
    if (e.key === 'Tab') {
      e.preventDefault();
      const start = e.target.selectionStart;
      const end = e.target.selectionEnd;
      sql = sql.slice(0, start) + '  ' + sql.slice(end);
      // Restore cursor — defer one tick so Svelte updates the DOM first
      setTimeout(() => {
        e.target.selectionStart = e.target.selectionEnd = start + 2;
      }, 0);
    }
  }

  function setExample(q) {
    sql = q;
  }

  function clearAll() {
    sql = '';
    error = null;
    result = null;
  }

  async function showDbObjects() {
    sql = DB_OBJECTS_SQL;
    await runQuery();
  }
</script>

<div class="space-y-5">

  <!-- ── Editor area ──────────────────────────────────────────────────────── -->
  <div class="bg-neutral-900 rounded-2xl border border-neutral-800 overflow-hidden">

    <!-- Toolbar -->
    <div class="flex items-center justify-between px-4 py-2.5 border-b border-neutral-800 bg-neutral-950/40 flex-wrap gap-2">
      <span class="text-xs font-semibold text-neutral-400 tracking-wider uppercase">SQL Query</span>
      <div class="flex items-center gap-2">
        <button
          id="show-db-objects"
          on:click={showDbObjects}
          class="flex items-center gap-1.5 text-[11px] px-2.5 py-1 rounded-lg
                 bg-indigo-950/60 border border-indigo-800/50 text-indigo-300
                 hover:bg-indigo-900/60 hover:text-indigo-100 transition-colors font-medium"
          title="{DB_OBJECTS_SQL}"
        >
          <span class="text-base leading-none">&#9783;</span>
          Show DB objects
        </button>
        <span class="text-[11px] text-neutral-600">Ctrl+Enter to run</span>
      </div>
    </div>

    <!-- Textarea -->
    <textarea
      id="query-input"
      bind:value={sql}
      on:keydown={handleKeydown}
      placeholder="SELECT * FROM expenses LIMIT 10"
      rows="7"
      spellcheck="false"
      autocomplete="off"
      autocorrect="off"
      autocapitalize="off"
      class="w-full bg-transparent px-4 py-4 text-sm font-mono text-neutral-100 placeholder-neutral-700
             resize-none outline-none leading-relaxed"
    ></textarea>

    <!-- Action bar -->
    <div class="flex items-center justify-between px-4 py-3 border-t border-neutral-800 bg-neutral-950/20 gap-3 flex-wrap">
      <div class="flex flex-wrap gap-2">
        {#each EXAMPLES as ex, i}
          <button
            id="example-{i}"
            on:click={() => setExample(ex)}
            class="text-[11px] px-2.5 py-1 rounded-lg bg-neutral-800 text-neutral-400
                   hover:text-neutral-100 hover:bg-neutral-700 transition-colors font-mono truncate max-w-[180px]"
            title={ex}
          >{ex.split(' ').slice(0, 4).join(' ')}…</button>
        {/each}
        <!-- User-specific presets -->
        {#each USER_PRESETS as preset}
          <button
            on:click={() => { sql = preset.sql; runQuery(); }}
            class="text-[11px] px-2.5 py-1 rounded-lg bg-indigo-950/60 border border-indigo-800/50 text-indigo-300
                   hover:bg-indigo-900/60 hover:text-indigo-100 transition-colors font-medium"
            title={preset.sql}
          >{preset.label}</button>
        {/each}
      </div>

      <div class="flex items-center gap-2 flex-none">
        {#if sql || result || error}
          <button
            id="query-clear"
            on:click={clearAll}
            class="text-xs px-3 py-1.5 rounded-lg text-neutral-500 hover:text-neutral-300
                   hover:bg-neutral-800 transition-colors"
          >Clear</button>
        {/if}
        <button
          id="query-run"
          on:click={runQuery}
          disabled={loading || !sql.trim()}
          class="flex items-center gap-2 text-sm font-semibold px-4 py-1.5 rounded-lg
                 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 disabled:cursor-not-allowed
                 text-white transition-colors"
        >
          {#if loading}
            <span class="w-3.5 h-3.5 rounded-full border-2 border-white/30 border-t-white animate-spin"></span>
            Running…
          {:else}
            ▶ Run
          {/if}
        </button>
      </div>
    </div>
  </div>

  <!-- ── Error ─────────────────────────────────────────────────────────────── -->
  {#if error}
    <div class="bg-red-950/50 border border-red-800/70 rounded-2xl px-5 py-4">
      <div class="flex items-start gap-3">
        <span class="text-red-400 text-lg leading-none mt-0.5 flex-none">✕</span>
        <div class="min-w-0">
          <p class="text-red-300 text-sm font-semibold mb-1">Query Error</p>
          <pre class="text-red-400 text-xs font-mono whitespace-pre-wrap break-words">{error}</pre>
        </div>
      </div>
    </div>
  {/if}

  <!-- ── Results ───────────────────────────────────────────────────────────── -->
  {#if result}
    <div class="bg-neutral-900 rounded-2xl border border-neutral-800 overflow-hidden">

      <!-- Results header -->
      <div class="flex items-center justify-between px-5 py-3 border-b border-neutral-800 bg-neutral-950/30">
        <div class="flex items-center gap-2">
          <span class="w-2 h-2 rounded-full bg-emerald-400"></span>
          <span class="text-xs font-semibold text-neutral-300">
            {result.row_count} {result.row_count === 1 ? 'row' : 'rows'}
            {#if result.truncated}<span class="text-amber-400"> (truncated to 50)</span>{/if}
          </span>
        </div>
        <span class="text-[11px] text-neutral-600">{result.columns.length} column{result.columns.length !== 1 ? 's' : ''}</span>
      </div>

      {#if result.columns.length === 0}
        <!-- Non-SELECT statement (INSERT/UPDATE/etc.) -->
        <div class="px-5 py-8 text-center">
          <p class="text-emerald-400 text-sm font-semibold">Statement executed successfully.</p>
          <p class="text-neutral-600 text-xs mt-1">No result set returned.</p>
        </div>

      {:else}
        <!-- Scrollable table -->
        <div class="overflow-x-auto max-h-[480px] overflow-y-auto">
          <table class="w-full text-xs min-w-max">
            <thead class="sticky top-0 bg-neutral-900 border-b border-neutral-800 z-10">
              <tr>
                <th class="px-3 py-2.5 text-left font-semibold text-neutral-500 w-10 select-none">#</th>
                {#each result.columns as col}
                  <th class="px-3 py-2.5 text-left font-semibold text-neutral-400 whitespace-nowrap">{col}</th>
                {/each}
              </tr>
            </thead>
            <tbody class="divide-y divide-neutral-800/60">
              {#each result.rows as row, i}
                <tr class="hover:bg-neutral-800/40 transition-colors">
                  <td class="px-3 py-2 text-neutral-600 select-none tabular-nums">{i + 1}</td>
                  {#each row as cell}
                    <td class="px-3 py-2 text-neutral-200 font-mono whitespace-nowrap">
                      {#if cell === null}
                        <span class="text-neutral-600 italic">NULL</span>
                      {:else}
                        {cell}
                      {/if}
                    </td>
                  {/each}
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

        {#if result.truncated}
          <div class="px-5 py-2.5 border-t border-amber-900/40 bg-amber-950/20">
            <p class="text-amber-400 text-[11px]">⚠ Results truncated — only first 50 rows shown. Add LIMIT to your query for precise control.</p>
          </div>
        {/if}
      {/if}

    </div>
  {/if}

</div>
