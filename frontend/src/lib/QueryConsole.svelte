<script>
  /**
   * QueryConsole.svelte
   *
   * Advanced SQL Query Workbench & Data Explorer with full Light/Dark mode theming:
   *  - Multi-query tab workbench with persistent state
   *  - Multi-statement execution (semicolon-separated query execution)
   *  - Interactive database schema & table explorer sidebar
   *  - Result search & column sorting
   *  - CSV / JSON / Markdown export & clipboard tools
   *  - Query execution history log with runtime benchmarks
   *  - Curated template library & database schema presets
   */

  import { onMount } from 'svelte';
  import { dec, authFetch } from './api.js';

  // ── Types & Initial State ──────────────────────────────────────────────────

  const DB_OBJECTS_SQL =
    `SELECT name, type, sql FROM sqlite_master WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' ORDER BY type, name`;

  const EXAMPLES = [
    'SELECT * FROM expenses ORDER BY expense_date DESC LIMIT 10',
    'SELECT * FROM splits ORDER BY category',
    'SELECT * FROM split_allocations ORDER BY category, user_name',
    'SELECT * FROM users ORDER BY name',
    'SELECT * FROM view_monthly_total',
    'SELECT * FROM view_monthly_by_category',
    "SELECT name, cost_cents / 100.0 AS euros, expense_date, who_paid, category FROM expenses WHERE strftime('%Y-%m', expense_date) = strftime('%Y-%m', 'now') ORDER BY cost_cents DESC",
  ];

  const USER_PRESETS = [
    {
      label: 'Deactivated users',
      sql: 'SELECT name, color, created_at FROM users WHERE is_active = 0 ORDER BY name',
    },
    {
      label: 'Expenses by deactivated users',
      sql: "SELECT e.id, e.name, e.cost_cents / 100.0 AS euros, e.expense_date, e.who_paid, e.category\nFROM expenses e\nWHERE e.who_paid IN (SELECT name FROM users WHERE is_active = 0)\nORDER BY e.expense_date DESC LIMIT 50",
    },
    {
      label: 'All users + totals',
      sql: "SELECT u.name, u.color, u.is_active,\n  COUNT(e.id) AS expense_count,\n  ROUND(COALESCE(SUM(e.cost_cents),0)/100.0,2) AS total_euros\nFROM users u\nLEFT JOIN expenses e ON e.who_paid = u.name\nGROUP BY u.name ORDER BY total_euros DESC",
    },
  ];

  const CATEGORIZED_PRESETS = [
    {
      group: '👥 Household & Members',
      items: [
        { label: 'Active Members', sql: 'SELECT name, color, is_active, created_at FROM users WHERE is_active = 1 ORDER BY name' },
        { label: 'Deactivated Members', sql: 'SELECT name, color, created_at FROM users WHERE is_active = 0 ORDER BY name' },
        { label: 'Member Spending Totals', sql: 'SELECT who_paid, COUNT(*) as count, ROUND(SUM(cost_cents)/100.0, 2) as total_eur FROM expenses GROUP BY who_paid ORDER BY total_eur DESC' },
      ],
    },
    {
      group: '💸 Expenses & Spending',
      items: [
        { label: 'Latest 20 Expenses', sql: 'SELECT id, name, cost_cents / 100.0 AS euros, expense_date, who_paid, category, is_joint FROM expenses ORDER BY expense_date DESC, id DESC LIMIT 20' },
        { label: 'Current Month Category Totals', sql: "SELECT category, COUNT(*) as count, ROUND(SUM(cost_cents)/100.0, 2) as total_eur FROM expenses WHERE strftime('%Y-%m', expense_date) = strftime('%Y-%m', 'now') GROUP BY category ORDER BY total_eur DESC" },
        { label: 'Project Linked Expenses', sql: 'SELECT e.id, e.name, e.cost_cents / 100.0 as euros, e.expense_date, p.name as project_name FROM expenses e INNER JOIN projects p ON e.project_id = p.id ORDER BY e.expense_date DESC' },
        { label: 'Joint Account Expenses', sql: 'SELECT id, name, cost_cents / 100.0 as euros, expense_date, who_paid, category FROM expenses WHERE is_joint = 1 ORDER BY expense_date DESC' },
      ],
    },
    {
      group: '🏦 Joint Accounts & Projects',
      items: [
        { label: 'Joint Accounts Overview', sql: 'SELECT id, name, balance_cents / 100.0 as balance_eur, safety_margin_pct, deposit_split_mode FROM joint_accounts' },
        { label: 'Active Projects Summary', sql: 'SELECT id, name, target_cents / 100.0 as target_eur, target_date, is_joint FROM projects ORDER BY target_date ASC' },
        { label: 'Tag Totals View', sql: 'SELECT * FROM view_tag_totals ORDER BY total_amount DESC' },
      ],
    },
    {
      group: '🛠️ Schema & System',
      items: [
        { label: 'All Tables & Views', sql: DB_OBJECTS_SQL },
        { label: 'Table Row Counts', sql: "SELECT 'expenses' as tbl, count(*) as rows FROM expenses UNION ALL SELECT 'users', count(*) FROM users UNION ALL SELECT 'splits', count(*) FROM splits UNION ALL SELECT 'projects', count(*) FROM projects UNION ALL SELECT 'tags', count(*) FROM tags UNION ALL SELECT 'income', count(*) FROM income" },
      ],
    },
  ];

  // ── Multi-Tab Workbench State ──────────────────────────────────────────────
  let tabs = [
    {
      id: 1,
      title: 'Query 1',
      sql: 'SELECT * FROM expenses ORDER BY expense_date DESC LIMIT 10',
      results: [],
      error: null,
      loading: false,
      executionTimeMs: null,
    },
  ];
  let activeTabId = 1;
  let nextTabId = 2;

  $: activeTab = tabs.find((t) => t.id === activeTabId) || tabs[0];

  // Convenience bindings for active tab
  $: sql = activeTab?.sql ?? '';
  $: result = activeTab?.results?.[activeResultIndex] ?? (activeTab?.results?.[0] || null);
  $: loading = activeTab?.loading ?? false;
  $: error = activeTab?.error ?? null;

  let activeResultIndex = 0;

  // ── UI Workbench Panes ─────────────────────────────────────────────────────
  let showSchemaSidebar = true;
  let showHistoryDrawer = false;
  let showPresetsModal = false;
  let resultFilterText = '';
  let sortColumn = null;
  let sortDirection = 'asc'; // 'asc' | 'desc'
  let copyFeedback = '';

  // ── Schema Explorer State ──────────────────────────────────────────────────
  let schemaObjects = [];
  let schemaLoading = false;
  let schemaFilter = '';

  // ── Query Execution History ────────────────────────────────────────────────
  let queryHistory = [];

  // ── Tab Management ─────────────────────────────────────────────────────────

  function addTab(initialSql = '') {
    const newId = nextTabId++;
    const newTab = {
      id: newId,
      title: `Query ${newId}`,
      sql: initialSql || 'SELECT * FROM expenses LIMIT 10',
      results: [],
      error: null,
      loading: false,
      executionTimeMs: null,
    };
    tabs = [...tabs, newTab];
    activeTabId = newId;
    activeResultIndex = 0;
    resultFilterText = '';
  }

  function closeTab(id, e) {
    if (e) e.stopPropagation();
    if (tabs.length === 1) {
      clearActiveTab();
      return;
    }
    const idx = tabs.findIndex((t) => t.id === id);
    tabs = tabs.filter((t) => t.id !== id);
    if (activeTabId === id) {
      const nextIdx = Math.max(0, idx - 1);
      activeTabId = tabs[nextIdx].id;
      activeResultIndex = 0;
      resultFilterText = '';
    }
  }

  function renameTab(tab) {
    const newTitle = prompt('Rename Query Tab:', tab.title);
    if (newTitle && newTitle.trim()) {
      tab.title = newTitle.trim();
      tabs = [...tabs];
    }
  }

  function clearActiveTab() {
    if (!activeTab) return;
    activeTab.sql = '';
    activeTab.results = [];
    activeTab.error = null;
    activeTab.executionTimeMs = null;
    tabs = [...tabs];
    resultFilterText = '';
  }

  // ── Multi-Statement Parser & Query Executor ────────────────────────────────

  function splitStatements(script) {
    const statements = [];
    let current = '';
    let inSingleQuote = false;
    let inDoubleQuote = false;

    for (let i = 0; i < script.length; i++) {
      const char = script[i];
      if (char === "'" && !inDoubleQuote) {
        inSingleQuote = !inSingleQuote;
        current += char;
      } else if (char === '"' && !inSingleQuote) {
        inDoubleQuote = !inDoubleQuote;
        current += char;
      } else if (char === ';' && !inSingleQuote && !inDoubleQuote) {
        if (current.trim()) {
          statements.push(current.trim());
        }
        current = '';
      } else {
        current += char;
      }
    }
    if (current.trim()) {
      statements.push(current.trim());
    }
    return statements;
  }

  async function executeSingleStatement(stmt) {
    const res = await authFetch('/query', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ sql: stmt }),
    });

    if (!res.ok) {
      const body = await res.json().catch(() => ({ detail: res.statusText }));
      throw new Error(body.detail ?? `Error ${res.status}: Query failed`);
    }

    const rawResult = await res.json();

    // Decrypt string cells
    if (rawResult.rows) {
      const decryptedRows = [];
      for (const row of rawResult.rows) {
        const decRow = [];
        for (const cell of row) {
          if (typeof cell === 'string') {
            decRow.push(await dec(cell));
          } else {
            decRow.push(cell);
          }
        }
        decryptedRows.push(decRow);
      }
      rawResult.rows = decryptedRows;
    }

    return {
      sql: stmt,
      columns: rawResult.columns || [],
      rows: rawResult.rows || [],
      row_count: rawResult.row_count || (rawResult.rows ? rawResult.rows.length : 0),
      truncated: Boolean(rawResult.truncated),
    };
  }

  async function runQuery() {
    if (!activeTab || !activeTab.sql.trim()) return;
    activeTab.loading = true;
    activeTab.error = null;
    activeTab.results = [];
    activeTab.executionTimeMs = null;
    activeResultIndex = 0;
    resultFilterText = '';
    sortColumn = null;
    tabs = [...tabs];

    const script = activeTab.sql.trim();
    const statements = splitStatements(script);
    const startTime = performance.now();

    try {
      const statementResults = [];
      for (const stmt of statements) {
        const res = await executeSingleStatement(stmt);
        statementResults.push(res);
      }
      const durationMs = Math.round(performance.now() - startTime);

      activeTab.results = statementResults;
      activeTab.executionTimeMs = durationMs;

      // Add to session history
      queryHistory = [
        {
          timestamp: new Date().toLocaleTimeString(),
          sql: script,
          durationMs,
          success: true,
          rowCount: statementResults.reduce((s, r) => s + r.row_count, 0),
        },
        ...queryHistory.slice(0, 49),
      ];
    } catch (err) {
      const durationMs = Math.round(performance.now() - startTime);
      activeTab.error = err.message ?? 'Query execution failed.';
      activeTab.executionTimeMs = durationMs;

      queryHistory = [
        {
          timestamp: new Date().toLocaleTimeString(),
          sql: script,
          durationMs,
          success: false,
          error: activeTab.error,
        },
        ...queryHistory.slice(0, 49),
      ];
    } finally {
      activeTab.loading = false;
      tabs = [...tabs];
    }
  }

  function handleKeydown(e) {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
      e.preventDefault();
      runQuery();
    }
  }

  function setExample(ex) {
    if (activeTab) {
      activeTab.sql = ex;
      tabs = [...tabs];
      runQuery();
    }
  }

  function insertTableQuery(tableName) {
    if (activeTab) {
      activeTab.sql = `SELECT * FROM ${tableName} LIMIT 25;`;
      tabs = [...tabs];
      runQuery();
    }
  }

  function formatSqlKeywords() {
    if (!activeTab || !activeTab.sql) return;
    const keywords = [
      'SELECT', 'FROM', 'WHERE', 'AND', 'OR', 'ORDER BY', 'GROUP BY', 'HAVING',
      'LIMIT', 'JOIN', 'LEFT JOIN', 'INNER JOIN', 'ON', 'AS', 'INSERT INTO',
      'VALUES', 'UPDATE', 'SET', 'DELETE', 'UNION ALL', 'UNION', 'CREATE TABLE',
      'VIEW', 'PRIMARY KEY', 'NOT NULL', 'DEFAULT', 'CHECK', 'GLOB', 'IN', 'LIKE',
      'IS NULL', 'IS NOT NULL', 'DESC', 'ASC', 'COALESCE', 'ROUND', 'SUM', 'COUNT', 'AVG'
    ];
    let formatted = activeTab.sql;
    keywords.forEach((kw) => {
      const regex = new RegExp(`\\b${kw}\\b`, 'gi');
      formatted = formatted.replace(regex, kw);
    });
    activeTab.sql = formatted;
    tabs = [...tabs];
  }

  // ── Results Filtering & Sorting ────────────────────────────────────────────

  $: currentResult = activeTab?.results?.[activeResultIndex] ?? (activeTab?.results?.[0] || { columns: [], rows: [] });

  function toggleSort(columnName) {
    if (sortColumn === columnName) {
      if (sortDirection === 'asc') {
        sortDirection = 'desc';
      } else {
        sortColumn = null;
        sortDirection = 'asc';
      }
    } else {
      sortColumn = columnName;
      sortDirection = 'asc';
    }
  }

  $: filteredRows = (() => {
    if (!currentResult || !currentResult.rows) return [];
    let rows = [...currentResult.rows];
    const cols = currentResult.columns || [];

    // Filter
    if (resultFilterText.trim()) {
      const q = resultFilterText.toLowerCase();
      rows = rows.filter((r) => r.some((cell) => String(cell ?? '').toLowerCase().includes(q)));
    }

    // Sort
    if (sortColumn && cols.includes(sortColumn)) {
      const colIdx = cols.indexOf(sortColumn);
      rows.sort((a, b) => {
        const valA = a[colIdx];
        const valB = b[colIdx];
        if (valA === valB) return 0;
        if (valA === null || valA === undefined) return 1;
        if (valB === null || valB === undefined) return -1;
        if (typeof valA === 'number' && typeof valB === 'number') {
          return sortDirection === 'asc' ? valA - valB : valB - valA;
        }
        return sortDirection === 'asc'
          ? String(valA).localeCompare(String(valB))
          : String(valB).localeCompare(String(valA));
      });
    }

    return rows;
  })();

  // ── Export Tools ───────────────────────────────────────────────────────────

  function exportCSV() {
    if (!currentResult || currentResult.columns.length === 0) return;
    const headers = currentResult.columns.join(',');
    const rows = filteredRows.map((r) =>
      r.map((cell) => (cell === null ? '' : `"${String(cell).replace(/"/g, '""')}"`)).join(',')
    );
    const csvContent = [headers, ...rows].join('\n');
    downloadBlob(csvContent, 'text/csv;charset=utf-8;', 'query_export.csv');
  }

  function exportJSON() {
    if (!currentResult || currentResult.columns.length === 0) return;
    const cols = currentResult.columns;
    const data = filteredRows.map((r) => {
      const obj = {};
      cols.forEach((col, idx) => {
        obj[col] = r[idx];
      });
      return obj;
    });
    const jsonContent = JSON.stringify(data, null, 2);
    downloadBlob(jsonContent, 'application/json;charset=utf-8;', 'query_export.json');
  }

  function copyMarkdownTable() {
    if (!currentResult || currentResult.columns.length === 0) return;
    const cols = currentResult.columns;
    const headerLine = `| ${cols.join(' | ')} |`;
    const sepLine = `| ${cols.map(() => '---').join(' | ')} |`;
    const rowLines = filteredRows.map((r) => `| ${r.map((c) => String(c ?? '')).join(' | ')} |`);
    const md = [headerLine, sepLine, ...rowLines].join('\n');
    navigator.clipboard.writeText(md);
    copyFeedback = '✓ Copied Markdown to clipboard!';
    setTimeout(() => { copyFeedback = ''; }, 3000);
  }

  function downloadBlob(content, mimeType, filename) {
    const blob = new Blob([content], { type: mimeType });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }

  // ── Database Schema Inspector ──────────────────────────────────────────────

  async function loadDbSchema() {
    schemaLoading = true;
    try {
      const res = await executeSingleStatement(DB_OBJECTS_SQL);
      if (res && res.rows) {
        schemaObjects = res.rows.map((r) => ({
          name: r[0],
          type: r[1],
          sql: r[2],
        }));
      }
    } catch (err) {
      console.error('Failed to load DB schema:', err);
    } finally {
      schemaLoading = false;
    }
  }

  async function showDbObjects() {
    if (activeTab) {
      activeTab.sql = DB_OBJECTS_SQL;
      tabs = [...tabs];
      await runQuery();
    }
  }

  onMount(() => {
    loadDbSchema();
  });
</script>

<div class="space-y-6">

  <!-- ── Top Header & Action Controls ────────────────────────────────────── -->
  <header class="page-header">
    <div>
      <h1 class="page-title flex items-center gap-2.5">
        <span>⚡</span> SQL Query Console & Explorer
      </h1>
      <p class="page-subtitle">
        Direct query workbench with multi-tab execution, schema inspector, and export tools
      </p>
    </div>

    <!-- Quick Utility Bar -->
    <div class="flex items-center gap-2 flex-wrap">
      <button
        type="button"
        on:click={() => (showSchemaSidebar = !showSchemaSidebar)}
        class="btn-secondary text-xs py-1.5 px-3 flex items-center gap-1.5 cursor-pointer {showSchemaSidebar ? 'bg-indigo-50 dark:bg-indigo-950/80 border-indigo-500 text-indigo-900 dark:text-indigo-200' : ''}"
      >
        <span>🗄️</span>
        <span>{showSchemaSidebar ? 'Hide Schema' : 'Show Schema'}</span>
      </button>

      <button
        type="button"
        on:click={() => (showHistoryDrawer = !showHistoryDrawer)}
        class="btn-secondary text-xs py-1.5 px-3 flex items-center gap-1.5 cursor-pointer {showHistoryDrawer ? 'bg-indigo-50 dark:bg-indigo-950/80 border-indigo-500 text-indigo-900 dark:text-indigo-200' : ''}"
      >
        <span>⏱️</span>
        <span>History ({queryHistory.length})</span>
      </button>

      <button
        type="button"
        on:click={() => (showPresetsModal = !showPresetsModal)}
        class="btn-secondary text-xs py-1.5 px-3 flex items-center gap-1.5 cursor-pointer"
      >
        <span>📚</span>
        <span>Preset Library</span>
      </button>
    </div>
  </header>

  <!-- ── Main Workbench Grid: Schema Sidebar + Query Editor ──────────────── -->
  <div class="grid grid-cols-1 {showSchemaSidebar ? 'lg:grid-cols-4' : ''} gap-6 items-start">

    <!-- Schema & Table Explorer (Sidebar) -->
    {#if showSchemaSidebar}
      <div class="card p-4 space-y-3 lg:col-span-1 max-h-[680px] flex flex-col">
        <div class="flex items-center justify-between">
          <span class="text-xs font-bold text-neutral-700 dark:text-neutral-300 uppercase tracking-wider flex items-center gap-1.5">
            <span>🗄️</span> DB Objects
          </span>
          <button
            type="button"
            on:click={loadDbSchema}
            disabled={schemaLoading}
            class="p-1 rounded-lg text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 transition-colors"
            title="Refresh database schema"
          >
            <svg class="w-3.5 h-3.5 {schemaLoading ? 'animate-spin' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
          </button>
        </div>

        <input
          type="text"
          bind:value={schemaFilter}
          placeholder="Filter tables/views…"
          class="input-field py-1.5 px-2.5 text-xs"
        />

        <div class="space-y-1.5 overflow-y-auto flex-1 pr-1">
          {#each schemaObjects.filter((o) => o.name.toLowerCase().includes(schemaFilter.toLowerCase())) as obj}
            <div class="group flex items-center justify-between p-2 rounded-xl bg-neutral-50 dark:bg-neutral-950/80 border border-neutral-200 dark:border-neutral-800/80 hover:border-indigo-500/50 transition-colors">
              <div class="min-w-0 flex items-center gap-2">
                <span class="text-xs">{obj.type === 'view' ? '👁️' : '📑'}</span>
                <span class="text-xs font-mono font-medium text-neutral-800 dark:text-neutral-200 truncate" title={obj.name}>
                  {obj.name}
                </span>
              </div>
              <button
                type="button"
                on:click={() => insertTableQuery(obj.name)}
                class="opacity-0 group-hover:opacity-100 px-2 py-0.5 rounded text-[10px] font-semibold bg-indigo-50 dark:bg-indigo-950 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800/60 hover:bg-indigo-100 dark:hover:bg-indigo-900 transition-all cursor-pointer"
                title="Insert SELECT query for {obj.name}"
              >
                SELECT
              </button>
            </div>
          {/each}

          {#if schemaObjects.length === 0}
            <p class="text-[11px] text-neutral-500 text-center py-4">Loading database objects…</p>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Main Editor & Results Area -->
    <div class="space-y-5 {showSchemaSidebar ? 'lg:col-span-3' : ''}">

      <!-- ── Multi-Query Workbench Tabs ───────────────────────────────────── -->
      <div class="flex items-center gap-1.5 overflow-x-auto border-b border-neutral-200 dark:border-neutral-800 pb-1">
        {#each tabs as tab (tab.id)}
          <div
            class="group flex items-center gap-2 px-3.5 py-2 rounded-t-xl text-xs font-semibold transition-all cursor-pointer select-none border-t border-x
                   {activeTabId === tab.id
                     ? 'bg-white dark:bg-neutral-900 border-neutral-200 dark:border-neutral-700 text-neutral-900 dark:text-white shadow-sm'
                     : 'bg-neutral-100 dark:bg-neutral-950/60 border-transparent text-neutral-500 dark:text-neutral-400 hover:text-neutral-800 dark:hover:text-neutral-200'}"
            on:click={() => {
              activeTabId = tab.id;
              activeResultIndex = 0;
              resultFilterText = '';
            }}
            on:keydown={(e) => e.key === 'Enter' && (activeTabId = tab.id)}
            role="button"
            tabindex="0"
          >
            <span class="truncate max-w-[120px]">{tab.title}</span>
            <button
              type="button"
              on:click={(e) => { e.stopPropagation(); renameTab(tab); }}
              class="opacity-0 group-hover:opacity-100 hover:text-indigo-600 dark:hover:text-indigo-300 text-neutral-400 text-[10px] p-0.5 rounded transition-opacity cursor-pointer"
              title="Rename tab"
            >
              ✎
            </button>
            {#if tab.loading}
              <span class="w-2.5 h-2.5 rounded-full border-2 border-indigo-600 dark:border-indigo-400 border-t-transparent animate-spin"></span>
            {/if}
            {#if tab.results.length > 0}
              <span class="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
            {/if}
            <button
              type="button"
              on:click={(e) => closeTab(tab.id, e)}
              class="opacity-0 group-hover:opacity-100 hover:text-rose-600 dark:hover:text-red-400 text-neutral-400 text-xs px-1 rounded transition-opacity cursor-pointer"
              title="Close tab"
            >
              ×
            </button>
          </div>
        {/each}

        <!-- Add Tab Button -->
        <button
          type="button"
          on:click={() => addTab()}
          class="p-2 rounded-xl text-neutral-500 hover:text-neutral-900 dark:hover:text-white hover:bg-neutral-100 dark:hover:bg-neutral-800 transition-colors text-xs font-bold cursor-pointer"
          title="New Query Tab"
        >
          + New Tab
        </button>
      </div>

      <!-- ── SQL Editor Container ─────────────────────────────────────────── -->
      <div class="card p-0 overflow-hidden">

        <!-- Editor Toolbar -->
        <div class="flex items-center justify-between px-4 py-2.5 border-b border-neutral-200 dark:border-neutral-800 bg-neutral-50 dark:bg-neutral-950/60 flex-wrap gap-2">
          <div class="flex items-center gap-2">
            <span class="text-xs font-bold text-neutral-700 dark:text-neutral-300 uppercase tracking-wider">SQL Query</span>
            {#if activeTab?.executionTimeMs !== null}
              <span class="px-2 py-0.5 rounded-md bg-neutral-200 dark:bg-neutral-800 text-[10px] font-mono text-neutral-700 dark:text-neutral-400 border border-neutral-300 dark:border-neutral-700">
                ⏱ {activeTab.executionTimeMs}ms
              </span>
            {/if}
          </div>

          <div class="flex items-center gap-2 flex-wrap">
            <button
              id="show-db-objects"
              type="button"
              on:click={showDbObjects}
              class="flex items-center gap-1.5 text-[11px] px-2.5 py-1 rounded-lg bg-indigo-50 dark:bg-indigo-950/60 border border-indigo-200 dark:border-indigo-800/50 text-indigo-700 dark:text-indigo-300 hover:bg-indigo-100 dark:hover:bg-indigo-900/60 transition-colors font-medium cursor-pointer"
              title={DB_OBJECTS_SQL}
            >
              <span class="text-sm leading-none">&#9783;</span>
              Show DB objects
            </button>

            <button
              type="button"
              on:click={formatSqlKeywords}
              class="btn-secondary text-[11px] px-2.5 py-1"
              title="Format SQL Keywords uppercase"
            >
              Format
            </button>

            <span class="text-[11px] text-neutral-500 font-mono hidden sm:inline">Ctrl+Enter to run</span>
          </div>
        </div>

        <!-- Textarea Editor -->
        <div class="relative bg-white dark:bg-neutral-950/40">
          <textarea
            id="query-input"
            bind:value={activeTab.sql}
            on:keydown={handleKeydown}
            placeholder="SELECT * FROM expenses LIMIT 10;"
            rows="8"
            spellcheck="false"
            autocomplete="off"
            autocorrect="off"
            autocapitalize="off"
            class="w-full bg-transparent px-4 py-3.5 text-sm font-mono text-neutral-900 dark:text-neutral-100 placeholder-neutral-400 dark:placeholder-neutral-700 resize-none outline-none leading-relaxed"
          ></textarea>
        </div>

        <!-- Action Bar & Preset Quick Chips -->
        <div class="flex items-center justify-between px-4 py-3 border-t border-neutral-200 dark:border-neutral-800 bg-neutral-50/50 dark:bg-neutral-950/30 gap-3 flex-wrap">
          <!-- Standard Examples & Presets (Maintains Backwards Compatibility) -->
          <div class="flex flex-wrap gap-1.5 items-center">
            {#each EXAMPLES as ex, i}
              <button
                id="example-{i}"
                type="button"
                on:click={() => setExample(ex)}
                class="text-[11px] px-2.5 py-1 rounded-lg bg-neutral-100 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-100 hover:bg-neutral-200 dark:hover:bg-neutral-700 transition-colors font-mono truncate max-w-[160px] cursor-pointer border border-neutral-200 dark:border-transparent"
                title={ex}
              >
                {ex.split(' ').slice(0, 4).join(' ')}…
              </button>
            {/each}

            {#each USER_PRESETS as preset}
              <button
                type="button"
                on:click={() => {
                  if (activeTab) activeTab.sql = preset.sql;
                  runQuery();
                }}
                class="text-[11px] px-2.5 py-1 rounded-lg bg-indigo-50 dark:bg-indigo-950/60 border border-indigo-200 dark:border-indigo-800/50 text-indigo-700 dark:text-indigo-300 hover:bg-indigo-100 dark:hover:bg-indigo-900/60 transition-colors font-medium cursor-pointer"
                title={preset.sql}
              >
                {preset.label}
              </button>
            {/each}
          </div>

          <!-- Execution Controls -->
          <div class="flex items-center gap-2 flex-none ml-auto">
            {#if activeTab?.sql || activeTab?.results.length > 0 || activeTab?.error}
              <button
                id="query-clear"
                type="button"
                on:click={clearActiveTab}
                class="btn-secondary text-xs px-3 py-1.5"
              >
                Clear
              </button>
            {/if}

            <button
              id="query-run"
              type="button"
              on:click={runQuery}
              disabled={loading || !activeTab?.sql.trim()}
              class="btn-primary flex items-center gap-2 text-xs px-4 py-1.5"
            >
              {#if loading}
                <span class="w-3.5 h-3.5 rounded-full border-2 border-white/30 border-t-white animate-spin"></span>
                <span>Executing…</span>
              {:else}
                <span>▶ Run Script</span>
              {/if}
            </button>
          </div>
        </div>
      </div>

      <!-- ── Error Callout ────────────────────────────────────────────────── -->
      {#if error}
        <div class="bg-rose-50 dark:bg-red-950/60 border border-rose-200 dark:border-red-800/80 rounded-2xl px-5 py-4 animate-fadeIn">
          <div class="flex items-start gap-3">
            <span class="text-rose-600 dark:text-red-400 text-lg leading-none mt-0.5 flex-none">✕</span>
            <div class="min-w-0">
              <p class="text-rose-700 dark:text-red-300 text-sm font-semibold mb-1">Query Execution Error</p>
              <pre class="text-rose-600 dark:text-red-400 text-xs font-mono whitespace-pre-wrap break-words">{error}</pre>
            </div>
          </div>
        </div>
      {/if}

      <!-- ── Multi-Statement Results Viewer ───────────────────────────────── -->
      {#if activeTab && activeTab.results && activeTab.results.length > 0}
        <div class="card p-0 overflow-hidden space-y-0">

          <!-- Multi-Statement Result Tabs (if multiple statements executed) -->
          {#if activeTab.results.length > 1}
            <div class="flex items-center gap-1 px-4 py-2 border-b border-neutral-200 dark:border-neutral-800 bg-neutral-50 dark:bg-neutral-950/80 overflow-x-auto">
              <span class="text-[11px] font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider mr-2">Results:</span>
              {#each activeTab.results as resItem, rIdx}
                <button
                  type="button"
                  on:click={() => (activeResultIndex = rIdx)}
                  class="px-3 py-1 rounded-lg text-xs font-medium border transition-colors cursor-pointer {activeResultIndex === rIdx ? 'bg-indigo-600 border-indigo-400 text-white shadow-sm font-semibold' : 'bg-white dark:bg-neutral-900 border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
                >
                  Result #{rIdx + 1} ({resItem.row_count} rows)
                </button>
              {/each}
            </div>
          {/if}

          <!-- Result Header & Data Tools -->
          <div class="flex flex-col sm:flex-row sm:items-center justify-between px-4 py-3 border-b border-neutral-200 dark:border-neutral-800 bg-neutral-50/50 dark:bg-neutral-950/40 gap-3">
            <div class="flex items-center gap-3 flex-wrap">
              <div class="flex items-center gap-2">
                <span class="w-2 h-2 rounded-full bg-emerald-500"></span>
                <span class="text-xs font-bold text-neutral-800 dark:text-neutral-200">
                  {filteredRows.length} {filteredRows.length === 1 ? 'row' : 'rows'}
                  {#if currentResult.truncated}
                    <span class="text-amber-600 dark:text-amber-400">(truncated to 50)</span>
                  {/if}
                </span>
              </div>
              <span class="text-xs text-neutral-400">·</span>
              <span class="text-xs text-neutral-500 dark:text-neutral-400">{currentResult.columns.length} columns</span>
            </div>

            <!-- In-Memory Table Search & Export Actions -->
            <div class="flex items-center gap-2 flex-wrap">
              {#if copyFeedback}
                <span class="text-xs text-emerald-600 dark:text-emerald-400 font-semibold animate-fadeIn">{copyFeedback}</span>
              {/if}

              <input
                type="text"
                bind:value={resultFilterText}
                placeholder="Filter results…"
                class="input-field py-1 px-2.5 text-xs max-w-[160px]"
              />

              <button
                type="button"
                on:click={exportCSV}
                class="btn-secondary text-xs py-1 px-2.5"
                title="Export result as CSV"
              >
                CSV
              </button>

              <button
                type="button"
                on:click={exportJSON}
                class="btn-secondary text-xs py-1 px-2.5"
                title="Export result as JSON"
              >
                JSON
              </button>

              <button
                type="button"
                on:click={copyMarkdownTable}
                class="btn-secondary text-xs py-1 px-2.5"
                title="Copy as Markdown Table to clipboard"
              >
                Markdown
              </button>
            </div>
          </div>

          <!-- Result Table or DDL Execution Notice -->
          {#if currentResult.columns.length === 0}
            <div class="px-5 py-8 text-center">
              <p class="text-emerald-600 dark:text-emerald-400 text-sm font-semibold">Statement executed successfully.</p>
              <p class="text-neutral-500 text-xs mt-1">No tabular data returned.</p>
            </div>
          {:else}
            <div class="overflow-x-auto max-h-[500px] overflow-y-auto">
              <table class="w-full text-xs min-w-max border-collapse">
                <thead class="sticky top-0 bg-neutral-100 dark:bg-neutral-900 border-b border-neutral-200 dark:border-neutral-800 z-10">
                  <tr>
                    <th class="px-3 py-2.5 text-left font-semibold text-neutral-500 w-10 select-none">#</th>
                    {#each currentResult.columns as col}
                      <th
                        on:click={() => toggleSort(col)}
                        class="px-3 py-2.5 text-left font-semibold text-neutral-700 dark:text-neutral-300 hover:text-neutral-900 dark:hover:text-white cursor-pointer select-none whitespace-nowrap"
                        title="Click to sort by {col}"
                      >
                        <div class="inline-flex items-center gap-1">
                          <span>{col}</span>
                          {#if sortColumn === col}
                            <span class="text-indigo-600 dark:text-indigo-400 font-bold">{sortDirection === 'asc' ? '▲' : '▼'}</span>
                          {/if}
                        </div>
                      </th>
                    {/each}
                  </tr>
                </thead>
                <tbody class="divide-y divide-neutral-200/60 dark:divide-neutral-800/60 font-mono">
                  {#each filteredRows as row, i}
                    <tr class="hover:bg-neutral-50 dark:hover:bg-neutral-800/40 transition-colors">
                      <td class="px-3 py-2 text-neutral-400 select-none tabular-nums font-sans">{i + 1}</td>
                      {#each row as cell}
                        <td class="px-3 py-2 text-neutral-800 dark:text-neutral-200 whitespace-nowrap">
                          {#if cell === null}
                            <span class="text-neutral-400 dark:text-neutral-600 italic">NULL</span>
                          {:else if typeof cell === 'number'}
                            <span class="tabular-nums text-sky-600 dark:text-sky-300 font-semibold">{cell}</span>
                          {:else}
                            <span>{cell}</span>
                          {/if}
                        </td>
                      {/each}
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>

            {#if currentResult.truncated}
              <div class="px-5 py-2.5 border-t border-amber-200 dark:border-amber-900/40 bg-amber-50 dark:bg-amber-950/20">
                <p class="text-amber-800 dark:text-amber-400 text-xs font-medium">
                  ⚠ Results truncated — only first 50 rows shown. Add LIMIT to your query for custom pagination.
                </p>
              </div>
            {/if}
          {/if}
        </div>
      {/if}

    </div>
  </div>

  <!-- ── Presets Modal ─────────────────────────────────────────────────────── -->
  {#if showPresetsModal}
    <div class="fixed inset-0 z-50 bg-black/60 dark:bg-black/75 backdrop-blur-sm flex items-center justify-center p-4">
      <div class="bg-white dark:bg-neutral-900 max-w-2xl w-full p-6 space-y-5 border border-neutral-200 dark:border-neutral-700 rounded-2xl shadow-2xl max-h-[85vh] overflow-y-auto">
        <div class="flex items-center justify-between border-b border-neutral-200 dark:border-neutral-800 pb-3">
          <h3 class="text-base font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span>📚</span> SQL Query Preset Library
          </h3>
          <button
            type="button"
            on:click={() => (showPresetsModal = false)}
            class="text-neutral-400 hover:text-neutral-900 dark:hover:text-white text-base font-bold px-2 py-1 cursor-pointer"
          >
            ✕
          </button>
        </div>

        <div class="space-y-4">
          {#each CATEGORIZED_PRESETS as cat}
            <div class="space-y-2">
              <h4 class="text-xs font-bold text-neutral-700 dark:text-neutral-300 uppercase tracking-wider">{cat.group}</h4>
              <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                {#each cat.items as item}
                  <button
                    type="button"
                    on:click={() => {
                      addTab(item.sql);
                      showPresetsModal = false;
                      runQuery();
                    }}
                    class="p-3 rounded-xl bg-neutral-50 dark:bg-neutral-950 border border-neutral-200 dark:border-neutral-800 text-left hover:border-indigo-500/60 hover:bg-indigo-50/50 dark:hover:bg-neutral-900 transition-all cursor-pointer group"
                  >
                    <p class="text-xs font-bold text-neutral-800 dark:text-neutral-200 group-hover:text-indigo-600 dark:group-hover:text-indigo-300">{item.label}</p>
                    <p class="text-[10px] font-mono text-neutral-500 truncate mt-1">{item.sql}</p>
                  </button>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}

  <!-- ── Query History Drawer (Modal) ──────────────────────────────────────── -->
  {#if showHistoryDrawer}
    <div class="fixed inset-0 z-50 bg-black/60 dark:bg-black/75 backdrop-blur-sm flex items-center justify-center p-4">
      <div class="bg-white dark:bg-neutral-900 max-w-2xl w-full p-6 space-y-4 border border-neutral-200 dark:border-neutral-700 rounded-2xl shadow-2xl max-h-[85vh] overflow-y-auto">
        <div class="flex items-center justify-between border-b border-neutral-200 dark:border-neutral-800 pb-3">
          <h3 class="text-base font-bold text-neutral-900 dark:text-white flex items-center gap-2">
            <span>⏱️</span> Query Execution History
          </h3>
          <button
            type="button"
            on:click={() => (showHistoryDrawer = false)}
            class="text-neutral-400 hover:text-neutral-900 dark:hover:text-white text-base font-bold px-2 py-1 cursor-pointer"
          >
            ✕
          </button>
        </div>

        {#if queryHistory.length === 0}
          <p class="text-xs text-neutral-500 text-center py-6">No queries executed in this session yet.</p>
        {:else}
          <div class="space-y-2">
            {#each queryHistory as h}
              <div class="p-3 rounded-xl bg-neutral-50 dark:bg-neutral-950 border border-neutral-200 dark:border-neutral-800 space-y-2">
                <div class="flex items-center justify-between text-xs">
                  <div class="flex items-center gap-2">
                    <span class="w-2 h-2 rounded-full {h.success ? 'bg-emerald-500' : 'bg-rose-500'}"></span>
                    <span class="text-neutral-500 dark:text-neutral-400 font-mono text-[11px]">{h.timestamp}</span>
                    <span class="text-neutral-400">·</span>
                    <span class="text-neutral-700 dark:text-neutral-300 font-mono text-[11px]">{h.durationMs}ms</span>
                    {#if h.success}
                      <span class="text-neutral-400">·</span>
                      <span class="text-neutral-700 dark:text-neutral-300 font-mono text-[11px]">{h.rowCount} rows</span>
                    {/if}
                  </div>
                  <button
                    type="button"
                    on:click={() => {
                      addTab(h.sql);
                      showHistoryDrawer = false;
                    }}
                    class="btn-secondary text-[11px] px-2 py-0.5"
                  >
                    Open in New Tab
                  </button>
                </div>
                <pre class="text-xs font-mono text-neutral-800 dark:text-neutral-300 bg-neutral-100 dark:bg-neutral-900 p-2 rounded-lg whitespace-pre-wrap break-words">{h.sql}</pre>
                {#if h.error}
                  <p class="text-xs text-rose-600 dark:text-red-400 font-mono">{h.error}</p>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}

</div>
