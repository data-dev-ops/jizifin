<script>
  /**
   * TagsTab.svelte — event tags with optional date windows. Lifetime totals and the
   * per-month / per-category breakdown come from `/views/tags`; the server rejects
   * tags outside their window and windows that would orphan tagged entries.
   */

  import { onDestroy, tick } from 'svelte';
  import Chart from 'chart.js/auto';
  import { entryApi, tagApi, views } from './api.js';
  import { money, fmtDate, fmtMonth } from './format.js';
  import { chartColors, PALETTE } from './chartTheme.js';
  import { getRandomPastelColor } from './colorUtils.js';
  import { ledgerVersion, tags, memberById, categoryName, theme } from './stores.js';

  let totals = new Map();
  let error = '';

  $: loadTotals($ledgerVersion);

  async function loadTotals() {
    try {
      totals = new Map((await views.tags()).map((t) => [t.tag, t]));
      if (selectedId != null) await loadDetail(selectedId);
    } catch (err) {
      error = err.message;
    }
  }

  // ── Form ───────────────────────────────────────────────────────────────────
  const blank = () => ({ id: null, name: '', description: '', color: getRandomPastelColor(), valid_from: '', valid_to: '' });
  let form = blank();
  let formError = '';
  let confirmDeleteId = null;

  function edit(t) {
    form = { ...t, description: t.description ?? '', valid_from: t.valid_from ?? '', valid_to: t.valid_to ?? '' };
    formError = '';
  }

  async function save() {
    formError = '';
    const payload = { name: form.name, description: form.description, color: form.color, valid_from: form.valid_from || null, valid_to: form.valid_to || null };
    try {
      if (form.id) await tagApi.update(form.id, payload);
      else await tagApi.create(payload);
      form = blank();
    } catch (err) {
      formError = err.message;
    }
  }

  async function remove(id) {
    try {
      await tagApi.remove(id);
      confirmDeleteId = null;
      if (selectedId === id) selectedId = null;
    } catch (err) {
      error = err.message;
    }
  }

  // ── Detail ─────────────────────────────────────────────────────────────────
  let selectedId = null;
  let detail = null;
  let entries = [];
  let barCanvas;
  let doughnutCanvas;
  let barChart = null;
  let doughnutChart = null;

  $: selected = $tags.find((t) => t.id === selectedId);
  $: if (detail && $theme) tick().then(renderCharts);

  async function select(id) {
    selectedId = selectedId === id ? null : id;
    detail = null;
    if (selectedId != null) await loadDetail(selectedId);
  }

  async function loadDetail(id) {
    const [d, page] = await Promise.all([views.tag(id), entryApi.page({ tag: id })]);
    detail = d;
    entries = page.entries;
    await tick();
    renderCharts();
  }

  function renderCharts() {
    barChart?.destroy();
    doughnutChart?.destroy();
    if (!detail) return;
    const c = chartColors();
    const color = selected?.color ?? '#f59e0b';
    const tooltip = { ...c.tooltip, callbacks: { label: (ctx) => ` ${$money(Math.round(ctx.raw * 100))}` } };
    if (barCanvas && detail.by_month.length) {
      barChart = new Chart(barCanvas, {
        type: 'bar',
        data: { labels: detail.by_month.map((b) => fmtMonth(b.key)), datasets: [{ data: detail.by_month.map((b) => b.total / 100), backgroundColor: `${color}aa`, borderColor: color, borderWidth: 1.5, borderRadius: 6 }] },
        options: { responsive: true, maintainAspectRatio: false, plugins: { legend: { display: false }, tooltip }, scales: { x: { grid: { color: c.grid }, ticks: { color: c.tick } }, y: { grid: { color: c.grid }, ticks: { color: c.tick }, beginAtZero: true } } },
      });
    }
    if (doughnutCanvas && detail.by_category.length) {
      doughnutChart = new Chart(doughnutCanvas, {
        type: 'doughnut',
        data: { labels: detail.by_category.map((b) => $categoryName(b.key)), datasets: [{ data: detail.by_category.map((b) => b.total / 100), backgroundColor: detail.by_category.map((_, i) => PALETTE[i % PALETTE.length]), borderWidth: 2 }] },
        options: { responsive: true, maintainAspectRatio: false, cutout: '60%', plugins: { legend: { position: 'right', labels: { color: c.tick, boxWidth: 12 } }, tooltip } },
      });
    }
  }

  onDestroy(() => {
    barChart?.destroy();
    doughnutChart?.destroy();
  });
</script>

<div class="space-y-6">
  <header class="page-header">
    <div>
      <h1 class="page-title flex items-center gap-2.5"><span>🏷️</span> Tags & Labels</h1>
      <p class="page-subtitle">Event tags across categories, with optional active windows.</p>
    </div>
  </header>

  {#if error}<p class="text-xs text-rose-600 dark:text-red-400">{error}</p>{/if}

  <div class="grid grid-cols-1 xl:grid-cols-5 gap-6">
    <div class="xl:col-span-2 space-y-4">
      <form class="card space-y-3" on:submit|preventDefault={save} id="tag-form">
        <h2 class="text-sm font-bold text-neutral-900 dark:text-white">{form.id ? 'Edit Tag' : 'New Tag'}</h2>
        <div class="flex gap-2">
          <input id="tag-name" type="text" bind:value={form.name} placeholder="e.g. Summer Trip 2026" class="input-field flex-1" />
          <input type="color" bind:value={form.color} class="w-10 h-10 rounded-lg" />
        </div>
        <input id="tag-description" type="text" bind:value={form.description} placeholder="Description (optional)" class="input-field" />
        <div class="grid grid-cols-2 gap-3">
          <label class="block text-xs text-neutral-600 dark:text-neutral-400">Active from <input id="tag-from" type="date" bind:value={form.valid_from} class="input-field mt-1" /></label>
          <label class="block text-xs text-neutral-600 dark:text-neutral-400">Until <input id="tag-to" type="date" bind:value={form.valid_to} class="input-field mt-1" /></label>
        </div>
        {#if formError}<p class="text-xs text-rose-600 dark:text-red-400">{formError}</p>{/if}
        <div class="flex gap-2">
          {#if form.id}<button type="button" on:click={() => (form = blank())} class="btn-secondary">Cancel</button>{/if}
          <button id="tag-save" type="submit" class="btn-primary flex-1">{form.id ? 'Save Tag' : '+ Create Tag'}</button>
        </div>
      </form>

      <div class="space-y-2">
        {#each $tags as t (t.id)}
          {@const tv = totals.get(t.id)}
          <div class="card p-3.5 space-y-2 cursor-pointer {selectedId === t.id ? 'ring-2 ring-indigo-500' : ''}" role="button" tabindex="0" on:click={() => select(t.id)} on:keydown={(e) => e.key === 'Enter' && select(t.id)}>
            <div class="flex items-center justify-between gap-2">
              <span class="flex items-center gap-2 min-w-0"><span class="w-3 h-3 rounded-full flex-none" style="background-color: {t.color}"></span><span class="text-sm font-semibold text-neutral-900 dark:text-white truncate">{t.name}</span></span>
              <span class="text-sm font-bold tabular-nums text-neutral-900 dark:text-white">{$money(tv?.total ?? 0)}</span>
            </div>
            <p class="text-[11px] text-neutral-500">{tv?.count ?? 0} expenses{t.valid_from || t.valid_to ? ` · ${fmtDate(t.valid_from)} → ${fmtDate(t.valid_to)}` : ''}</p>
            <div class="flex gap-3 text-xs" on:click|stopPropagation on:keydown|stopPropagation role="presentation">
              <button type="button" on:click={() => edit(t)} class="text-indigo-600 dark:text-indigo-400">Edit</button>
              {#if confirmDeleteId === t.id}
                <button type="button" on:click={() => remove(t.id)} class="px-2 py-0.5 rounded bg-rose-600 text-white">Confirm (entries stay, untagged)</button>
                <button type="button" on:click={() => (confirmDeleteId = null)} class="text-neutral-500">✕</button>
              {:else}
                <button type="button" on:click={() => (confirmDeleteId = t.id)} class="text-rose-600 dark:text-red-400">Delete</button>
              {/if}
            </div>
          </div>
        {:else}
          <div class="card empty-state-box py-8"><p class="text-xs text-neutral-500">No tags yet.</p></div>
        {/each}
      </div>
    </div>

    <div class="xl:col-span-3 space-y-4">
      {#if selected && detail}
        {@const tv = totals.get(selected.id)}
        <div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
          <div class="card-sub p-3.5"><p class="text-[11px] text-neutral-500 uppercase">Total</p><p class="text-lg font-bold tabular-nums text-neutral-900 dark:text-white">{$money(tv?.total ?? 0)}</p></div>
          <div class="card-sub p-3.5"><p class="text-[11px] text-neutral-500 uppercase">Expenses</p><p class="text-lg font-bold tabular-nums text-neutral-900 dark:text-white">{tv?.count ?? 0}</p></div>
          <div class="card-sub p-3.5"><p class="text-[11px] text-neutral-500 uppercase">Avg / month</p><p class="text-lg font-bold tabular-nums text-neutral-900 dark:text-white">{$money(tv?.monthly_avg ?? 0)}</p></div>
          <div class="card-sub p-3.5"><p class="text-[11px] text-neutral-500 uppercase">Span</p><p class="text-xs font-semibold text-neutral-900 dark:text-white">{fmtDate(tv?.first)} → {fmtDate(tv?.last)}</p></div>
        </div>
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <div class="card p-4"><h3 class="text-xs font-bold uppercase text-neutral-600 dark:text-neutral-400 mb-2">By month</h3><div class="relative h-56"><canvas bind:this={barCanvas}></canvas></div></div>
          <div class="card p-4"><h3 class="text-xs font-bold uppercase text-neutral-600 dark:text-neutral-400 mb-2">By category</h3><div class="relative h-56"><canvas bind:this={doughnutCanvas}></canvas></div></div>
        </div>
        <div class="card p-4 space-y-2">
          <h3 class="text-xs font-bold uppercase text-neutral-600 dark:text-neutral-400">Tagged expenses</h3>
          {#each entries as e (e.id)}
            <div class="card-sub p-2.5 flex items-center justify-between gap-3 text-xs">
              <span class="min-w-0"><span class="font-mono text-neutral-400 mr-2">{fmtDate(e.date)}</span><span class="font-semibold text-neutral-900 dark:text-white">{e.name}</span>
                <span class="text-neutral-500"> · {$memberById.get(e.member)?.name} · {$categoryName(e.category)}</span></span>
              <span class="font-bold tabular-nums text-neutral-900 dark:text-white">{$money(e.amount)}</span>
            </div>
          {/each}
        </div>
      {:else}
        <div class="card empty-state-box py-16"><p class="text-sm text-neutral-500">Select a tag to see its breakdown.</p></div>
      {/if}
    </div>
  </div>
</div>
