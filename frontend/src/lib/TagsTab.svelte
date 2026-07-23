<script>
  /**
   * TagsTab.svelte
   *
   * Full Tags management tab:
   *  - Create new tag (name, color, optional description)
   *  - List all tags as cards with all-time totals
   *  - Edit any tag inline (name, color, description)
   *  - Delete with confirmation (expenses become untagged, not deleted)
   *  - Select a tag to see its full cross-month detail view:
   *      · Summary stat cards
   *      · Bar chart: spending by month (Chart.js)
   *      · Doughnut chart: spending by category (Chart.js)
   *      · Full expense list for this tag
   */

  import { onMount, onDestroy, tick } from 'svelte';
  import { tags, expenses, currencySymbol } from './stores.js';
  import { createTag, updateTag, deleteTag, fetchTagAnalytics } from './api.js';
  import Chart from 'chart.js/auto';

  // ── helpers ────────────────────────────────────────────────────────────────

  /** Integer cents → formatted amount */
  function fmtAmt(euros) {
    const val = Number(euros) || 0;
    return `${$currencySymbol}${val.toLocaleString('en-GB', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  }

  /** YYYY-MM-DD → DD/MM/YYYY */
  function fmtDate(iso) {
    if (!iso) return '—';
    const [y, m, d] = iso.split('-');
    return `${d}/${m}/${y}`;
  }

  /** YYYY-MM → e.g. "Jul 2025" */
  function fmtMonth(ym) {
    if (!ym) return '—';
    const [y, m] = ym.split('-');
    return new Date(Number(y), Number(m) - 1, 1).toLocaleDateString('en-GB', { month: 'short', year: 'numeric' });
  }

  /** Compute avg monthly spend from total and date range */
  function avgPerMonth(tag) {
    if (!tag.first_date || !tag.last_date || tag.expense_count === 0) return null;
    const first = new Date(tag.first_date);
    const last  = new Date(tag.last_date);
    const months = Math.max(1, (last.getFullYear() - first.getFullYear()) * 12 + (last.getMonth() - first.getMonth()) + 1);
    return tag.total_amount / months;
  }

  // ── add-tag form ───────────────────────────────────────────────────────────

  let newName        = '';
  let newColor       = '#f59e0b';
  let newDescription = '';
  let addSubmitting  = false;
  let addError       = null;
  let addSuccess     = false;

  async function handleAdd(e) {
    e.preventDefault();
    addError   = null;
    addSuccess = false;
    if (!newName.trim()) { addError = 'Tag name is required.'; return; }
    addSubmitting = true;
    try {
      await createTag({
        name:        newName.trim(),
        color:       newColor,
        description: newDescription.trim() || null,
      });
      addSuccess    = true;
      newName        = '';
      newColor       = '#f59e0b';
      newDescription = '';
    } catch (err) {
      addError = err.message ?? 'Failed to create tag.';
    } finally {
      addSubmitting = false;
    }
  }

  // ── per-card edit state ────────────────────────────────────────────────────

  let editingId      = null;
  let editName       = '';
  let editColor      = '';
  let editDesc       = '';
  let editSubmitting = false;
  let editError      = null;

  function startEdit(t) {
    editingId  = t.id;
    editName   = t.name;
    editColor  = t.color;
    editDesc   = t.description ?? '';
    editError  = null;
  }

  function cancelEdit() { editingId = null; editError = null; }

  async function handleEdit(e, id) {
    e.preventDefault();
    editError = null;
    if (!editName.trim()) { editError = 'Tag name required.'; return; }
    editSubmitting = true;
    try {
      await updateTag(id, {
        name:        editName.trim(),
        color:       editColor,
        description: editDesc.trim() || null,
      });
      // If this is the selected tag, reload its detail
      if (selectedTagId === id) await loadTagDetail(id);
      editingId = null;
    } catch (err) {
      editError = err.message ?? 'Failed to update tag.';
    } finally {
      editSubmitting = false;
    }
  }

  // ── per-card delete state ──────────────────────────────────────────────────

  let confirmDeleteId = null;
  let deletingId      = null;
  let deleteError     = null;

  async function handleDelete(id) {
    deletingId  = id;
    deleteError = null;
    try {
      await deleteTag(id);
      confirmDeleteId = null;
      if (selectedTagId === id) {
        selectedTagId = null;
        tagDetail     = null;
      }
    } catch (err) {
      deleteError = err.message ?? 'Delete failed.';
    } finally {
      deletingId = null;
    }
  }

  // ── tag detail (right panel) ───────────────────────────────────────────────

  let selectedTagId = null;
  let tagDetail     = null;   // TagDetailResponse from /analytics/tags/{id}
  let detailLoading = false;
  let detailError   = null;

  async function loadTagDetail(id) {
    detailLoading = true;
    detailError   = null;
    try {
      tagDetail = await fetchTagAnalytics(id);
    } catch (err) {
      detailError = err.message ?? 'Failed to load tag detail.';
      tagDetail   = null;
    } finally {
      detailLoading = false;
    }
  }

  async function selectTag(id) {
    selectedTagId = id;
    await loadTagDetail(id);
  }

  // ── Chart.js instances ─────────────────────────────────────────────────────

  let barCanvas;
  let doughnutCanvas;
  let barChart;
  let doughnutChart;

  /**
   * Rebuild both charts whenever tagDetail changes.
   * Charts are destroyed and recreated to avoid stale reference bugs.
   */
  $: if (tagDetail && barCanvas && doughnutCanvas) {
    tick().then(() => renderCharts());
  }

  function renderCharts() {
    const color = tagDetail?.tag?.color ?? '#f59e0b';

    // ── Bar chart: spending by month ─────────────────────────────────────
    if (barChart) { barChart.destroy(); barChart = null; }
    if (barCanvas && tagDetail?.by_month?.length > 0) {
      barChart = new Chart(barCanvas, {
        type: 'bar',
        data: {
          labels: tagDetail.by_month.map((r) => fmtMonth(r.month)),
          datasets: [{
            label: 'Spending',
            data: tagDetail.by_month.map((r) => r.total_amount),
            backgroundColor: color + 'aa',
            borderColor:     color,
            borderWidth:     1.5,
            borderRadius:    6,
            borderSkipped:   false,
          }],
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: {
            legend: { display: false },
            tooltip: {
              callbacks: {
                label: (ctx) => ` ${$currencySymbol}${ctx.raw.toFixed(2)}`,
              },
            },
          },
          scales: {
            x: {
              grid:  { color: 'rgba(255,255,255,0.05)' },
              ticks: { color: '#9ca3af', font: { size: 11 } },
            },
            y: {
              grid:  { color: 'rgba(255,255,255,0.05)' },
              ticks: { color: '#9ca3af', font: { size: 11 }, callback: (v) => `${$currencySymbol}${v}` },
              beginAtZero: true,
            },
          },
        },
      });
    }

    // ── Doughnut chart: spending by category ──────────────────────────────
    if (doughnutChart) { doughnutChart.destroy(); doughnutChart = null; }
    if (doughnutCanvas && tagDetail?.by_category?.length > 0) {
      // Generate palette: base color + shifted hues
      const palette = tagDetail.by_category.map((_, i) => {
        const hue = (parseInt(color.slice(1), 16) % 360 + i * 37) % 360;
        return `hsl(${hue}, 70%, 55%)`;
      });
      doughnutChart = new Chart(doughnutCanvas, {
        type: 'doughnut',
        data: {
          labels: tagDetail.by_category.map((r) => r.category),
          datasets: [{
            data:            tagDetail.by_category.map((r) => r.total_amount),
            backgroundColor: palette.map((c) => c.replace('55%', '45%') + ''),
            borderColor:     palette,
            borderWidth:     1.5,
            hoverOffset:     6,
          }],
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          cutout: '60%',
          plugins: {
            legend: {
              position: 'right',
              labels: { color: '#d1d5db', font: { size: 11 }, boxWidth: 12, padding: 10 },
            },
            tooltip: {
              callbacks: {
                label: (ctx) => ` ${$currencySymbol}${ctx.raw.toFixed(2)}`,
              },
            },
          },
        },
      });
    }
  }

  onDestroy(() => {
    barChart?.destroy();
    doughnutChart?.destroy();
  });

  // ── Expenses for the selected tag ──────────────────────────────────────────
  /** Filter the global expenses store to only those tagged with selectedTagId */
  $: taggedExpenses = selectedTagId
    ? $expenses.filter((e) => e.tag_id === selectedTagId).sort((a, b) =>
        b.expense_date.localeCompare(a.expense_date)
      )
    : [];
</script>


<div class="grid grid-cols-1 xl:grid-cols-5 gap-6">

  <!-- ── LEFT PANEL: Create + Tag list ──────────────────────────────────── -->
  <div class="xl:col-span-2 space-y-4">

    <!-- Add Tag Form -->
    <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6">
      <h2 class="text-sm font-semibold text-neutral-300 mb-5">New Tag</h2>

      <form on:submit={handleAdd} id="add-tag-form" class="space-y-4">

        <div>
          <label for="tag-name" class="block text-xs font-medium text-neutral-400 mb-1.5">Tag Name</label>
          <input
            id="tag-name"
            type="text"
            maxlength="96"
            placeholder="e.g. Paris 2025, Bathroom Reno"
            bind:value={newName}
            class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2.5 text-sm
                   text-neutral-100 placeholder-neutral-600
                   focus:outline-none focus:border-amber-500 focus:ring-1 focus:ring-amber-500 transition-colors"
          />
        </div>

        <div class="flex items-center gap-3">
          <div class="flex-1">
            <label for="tag-description" class="block text-xs font-medium text-neutral-400 mb-1.5">
              Description <span class="text-neutral-600">(optional)</span>
            </label>
            <input
              id="tag-description"
              type="text"
              maxlength="512"
              placeholder="e.g. Summer 2025 — flights, hotels, food"
              bind:value={newDescription}
              class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2.5 text-sm
                     text-neutral-100 placeholder-neutral-600
                     focus:outline-none focus:border-amber-500 focus:ring-1 focus:ring-amber-500 transition-colors"
            />
          </div>
          <div>
            <label for="tag-color" class="block text-xs font-medium text-neutral-400 mb-1.5">Color</label>
            <div class="relative">
              <input
                id="tag-color"
                type="color"
                bind:value={newColor}
                class="w-12 h-10 rounded-lg cursor-pointer border border-neutral-700 bg-neutral-800
                       [color-scheme:dark] p-0.5"
              />
            </div>
          </div>
        </div>

        {#if addError}
          <p class="text-red-400 text-xs bg-red-950/40 border border-red-900 rounded-lg px-3 py-2">{addError}</p>
        {/if}
        {#if addSuccess}
          <p class="text-emerald-400 text-xs bg-emerald-950/40 border border-emerald-900 rounded-lg px-3 py-2">
            ✓ Tag created successfully.
          </p>
        {/if}

        <button
          id="submit-tag"
          type="submit"
          disabled={addSubmitting}
          class="w-full py-2.5 rounded-lg text-sm font-semibold
                 bg-gradient-to-r from-amber-600 to-orange-600
                 hover:from-amber-500 hover:to-orange-500
                 disabled:opacity-50 disabled:cursor-not-allowed
                 transition-all duration-150 shadow-md shadow-amber-900/30 active:scale-[0.98]"
        >
          {addSubmitting ? 'Creating…' : 'Create Tag'}
        </button>
      </form>
    </div>

    <!-- Tag Cards List -->
    {#if $tags.length === 0}
      <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-10 flex flex-col items-center text-center">
        <div class="w-12 h-12 rounded-2xl bg-neutral-800 flex items-center justify-center text-2xl mb-4">🏷</div>
        <p class="text-neutral-400 text-sm">No tags yet.</p>
        <p class="text-neutral-600 text-xs mt-1">Create a tag to start grouping expenses across months.</p>
      </div>
    {:else}
      {#each $tags as tag (tag.id)}
        <div
          id="tag-card-{tag.id}"
          class="bg-neutral-900 rounded-2xl border transition-all duration-150 p-4 cursor-pointer
                 {selectedTagId === tag.id ? 'border-amber-500/60 shadow-sm shadow-amber-900/30' : 'border-neutral-800 hover:border-neutral-700'}"
          on:click={() => selectTag(tag.id)}
          on:keydown={(e) => e.key === 'Enter' && selectTag(tag.id)}
          role="button"
          tabindex="0"
          aria-pressed={selectedTagId === tag.id}
        >
          {#if editingId === tag.id}
            <!-- svelte-ignore a11y-click-events-have-key-events -->
            <!-- svelte-ignore a11y-no-static-element-interactions -->
            <div on:click|stopPropagation>
              <form on:submit={(e) => handleEdit(e, tag.id)} class="space-y-3">
                <input
                  id="edit-tag-name-{tag.id}"
                  type="text"
                  maxlength="96"
                  bind:value={editName}
                  class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm
                         text-neutral-100 focus:outline-none focus:border-amber-500 focus:ring-1 focus:ring-amber-500"
                />
                <input
                  id="edit-tag-desc-{tag.id}"
                  type="text"
                  maxlength="512"
                  placeholder="Description (optional)"
                  bind:value={editDesc}
                  class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm
                         text-neutral-100 placeholder-neutral-600 focus:outline-none focus:border-amber-500 focus:ring-1 focus:ring-amber-500"
                />
                <div class="flex items-center gap-3">
                  <label class="text-xs text-neutral-500">Color</label>
                  <input
                    id="edit-tag-color-{tag.id}"
                    type="color"
                    bind:value={editColor}
                    class="w-10 h-8 rounded cursor-pointer border border-neutral-700 bg-neutral-800 [color-scheme:dark] p-0.5"
                  />
                </div>
                {#if editError}
                  <p class="text-red-400 text-xs">{editError}</p>
                {/if}
                <div class="flex gap-2">
                  <button
                    id="save-tag-edit-{tag.id}"
                    type="submit"
                    disabled={editSubmitting}
                    class="flex-1 py-1.5 rounded-lg text-xs font-semibold bg-amber-600 hover:bg-amber-500
                           disabled:opacity-40 transition-colors"
                  >{editSubmitting ? 'Saving…' : 'Save'}</button>
                  <button
                    id="cancel-tag-edit-{tag.id}"
                    type="button"
                    on:click={cancelEdit}
                    class="flex-1 py-1.5 rounded-lg text-xs font-semibold bg-neutral-700 hover:bg-neutral-600 transition-colors"
                  >Cancel</button>
                </div>
              </form>
            </div>
          {:else}
            <!-- View mode -->
            <div class="flex items-start justify-between gap-2">
              <div class="flex items-center gap-2.5 min-w-0">
                <!-- Color dot -->
                <span class="w-3 h-3 rounded-full flex-none mt-0.5" style="background-color: {tag.color}"></span>
                <div class="min-w-0">
                  <h3 class="text-sm font-semibold text-neutral-100 truncate">{tag.name}</h3>
                  {#if tag.description}
                    <p class="text-[11px] text-neutral-500 truncate mt-0.5">{tag.description}</p>
                  {/if}
                </div>
              </div>

              <!-- Action buttons -->
              <!-- svelte-ignore a11y-click-events-have-key-events -->
              <!-- svelte-ignore a11y-no-static-element-interactions -->
              <div class="flex items-center gap-1 flex-none" on:click|stopPropagation>
                <button
                  id="edit-tag-{tag.id}"
                  on:click={() => startEdit(tag)}
                  title="Edit tag"
                  class="p-1.5 rounded-lg text-neutral-500 hover:text-amber-400 hover:bg-amber-950/40 transition-all duration-150"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-3.5 h-3.5">
                    <path d="M5.433 13.917l1.262-3.155A4 4 0 0 1 7.58 9.42l6.92-6.918a2.121 2.121 0 0 1 3 3l-6.92 6.918c-.383.383-.84.685-1.343.886l-3.154 1.262a.5.5 0 0 1-.65-.65Z" />
                    <path d="M3.5 5.75c0-.69.56-1.25 1.25-1.25H10A.75.75 0 0 0 10 3H4.75A2.75 2.75 0 0 0 2 5.75v9.5A2.75 2.75 0 0 0 4.75 18h9.5A2.75 2.75 0 0 0 17 15.25V10a.75.75 0 0 0-1.5 0v5.25c0 .69-.56 1.25-1.25 1.25h-9.5c-.69 0-1.25-.56-1.25-1.25v-9.5Z" />
                  </svg>
                </button>

                {#if confirmDeleteId === tag.id}
                  <span class="flex items-center gap-1">
                    <span class="text-[10px] text-neutral-400">Delete?</span>
                    <button
                      id="confirm-delete-tag-{tag.id}"
                      on:click={() => handleDelete(tag.id)}
                      disabled={deletingId === tag.id}
                      class="px-2 py-0.5 rounded text-xs font-semibold bg-red-600 hover:bg-red-500 disabled:opacity-40 transition-colors"
                    >{deletingId === tag.id ? '…' : 'Yes'}</button>
                    <button
                      id="cancel-delete-tag-{tag.id}"
                      on:click={() => { confirmDeleteId = null; deleteError = null; }}
                      class="px-2 py-0.5 rounded text-xs font-semibold bg-neutral-700 hover:bg-neutral-600 transition-colors"
                    >No</button>
                  </span>
                {:else}
                  <button
                    id="delete-tag-{tag.id}"
                    on:click={() => { confirmDeleteId = tag.id; deleteError = null; }}
                    title="Delete tag"
                    class="p-1.5 rounded-lg text-neutral-500 hover:text-red-400 hover:bg-red-950/40 transition-all duration-150"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-3.5 h-3.5">
                      <path fill-rule="evenodd" d="M8.75 1A2.75 2.75 0 0 0 6 3.75v.443c-.795.077-1.584.176-2.365.298a.75.75 0 1 0 .23 1.482l.149-.022.841 10.518A2.75 2.75 0 0 0 7.596 19h4.807a2.75 2.75 0 0 0 2.742-2.53l.841-10.52.149.023a.75.75 0 0 0 .23-1.482A41.03 41.03 0 0 0 14 4.193V3.75A2.75 2.75 0 0 0 11.25 1h-2.5ZM10 4c.84 0 1.673.025 2.5.075V3.75c0-.69-.56-1.25-1.25-1.25h-2.5c-.69 0-1.25.56-1.25 1.25v.325C8.327 4.025 9.16 4 10 4ZM8.58 7.72a.75.75 0 0 0-1.5.06l.3 7.5a.75.75 0 1 0 1.5-.06l-.3-7.5Zm4.34.06a.75.75 0 1 0-1.5-.06l-.3 7.5a.75.75 0 1 0 1.5.06l.3-7.5Z" clip-rule="evenodd" />
                    </svg>
                  </button>
                {/if}
              </div>
            </div>

            <!-- Totals row -->
            <div class="mt-3 flex items-center gap-4 text-xs">
              <span class="font-semibold tabular-nums" style="color: {tag.color}">
                {fmtAmt(tag.total_amount)}
              </span>
              <span class="text-neutral-600">·</span>
              <span class="text-neutral-500">{tag.expense_count} expense{tag.expense_count !== 1 ? 's' : ''}</span>
              {#if tag.first_date}
                <span class="text-neutral-600">·</span>
                <span class="text-neutral-600">{fmtDate(tag.first_date)} → {fmtDate(tag.last_date)}</span>
              {/if}
            </div>
          {/if}
        </div>
      {/each}
    {/if}
  </div>

  <!-- ── RIGHT PANEL: Tag detail ────────────────────────────────────────── -->
  <div class="xl:col-span-3 space-y-4">

    {#if !selectedTagId}
      <!-- Empty state -->
      <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-14 flex flex-col items-center text-center">
        <div class="w-14 h-14 rounded-2xl bg-neutral-800 flex items-center justify-center text-3xl mb-4">🏷</div>
        <p class="text-neutral-300 font-semibold text-sm">Select a tag</p>
        <p class="text-neutral-600 text-xs mt-1 max-w-xs">
          Click a tag on the left to see its full spending breakdown across all months.
        </p>
      </div>

    {:else if detailLoading}
      <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-14 flex items-center justify-center">
        <div class="w-8 h-8 rounded-full border-2 border-amber-500 border-t-transparent animate-spin"></div>
      </div>

    {:else if detailError}
      <div class="bg-neutral-900 rounded-2xl border border-red-900/40 p-8 text-center">
        <p class="text-red-400 text-sm">{detailError}</p>
      </div>

    {:else if tagDetail}
      {@const t = tagDetail.tag}
      {@const avg = avgPerMonth(t)}

      <!-- Summary stat cards -->
      <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6">
        <div class="flex items-center gap-2.5 mb-5">
          <span class="w-3 h-3 rounded-full flex-none" style="background-color: {t.color}"></span>
          <h2 class="text-sm font-semibold text-neutral-200">{t.name}</h2>
          {#if t.description}
            <span class="text-xs text-neutral-500 truncate">— {t.description}</span>
          {/if}
        </div>

        <div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
          <div class="bg-neutral-800/60 rounded-xl p-3">
            <p class="text-[10px] text-neutral-500 mb-1 uppercase tracking-wide">Total Spent</p>
            <p class="text-sm font-bold" style="color: {t.color}">{fmtAmt(t.total_amount)}</p>
          </div>
          <div class="bg-neutral-800/60 rounded-xl p-3">
            <p class="text-[10px] text-neutral-500 mb-1 uppercase tracking-wide">Expenses</p>
            <p class="text-sm font-bold text-neutral-100">{t.expense_count}</p>
          </div>
          <div class="bg-neutral-800/60 rounded-xl p-3">
            <p class="text-[10px] text-neutral-500 mb-1 uppercase tracking-wide">Avg / Month</p>
            <p class="text-sm font-bold text-neutral-100">{avg ? fmtAmt(avg) : '—'}</p>
          </div>
          <div class="bg-neutral-800/60 rounded-xl p-3">
            <p class="text-[10px] text-neutral-500 mb-1 uppercase tracking-wide">Date Range</p>
            <p class="text-xs font-semibold text-neutral-300">
              {t.first_date ? `${fmtDate(t.first_date)}` : '—'}
            </p>
            {#if t.last_date && t.last_date !== t.first_date}
              <p class="text-[10px] text-neutral-500">→ {fmtDate(t.last_date)}</p>
            {/if}
          </div>
        </div>
      </div>

      <!-- Bar chart: spending by month -->
      {#if tagDetail.by_month.length > 0}
        <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6">
          <h3 class="text-xs font-semibold text-neutral-400 uppercase tracking-wide mb-4">Spending Over Time</h3>
          <div class="h-48">
            <canvas bind:this={barCanvas} id="tag-bar-chart-{selectedTagId}"></canvas>
          </div>
        </div>
      {/if}

      <!-- Doughnut chart: by category -->
      {#if tagDetail.by_category.length > 0}
        <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6">
          <h3 class="text-xs font-semibold text-neutral-400 uppercase tracking-wide mb-4">By Category</h3>
          <div class="h-52">
            <canvas bind:this={doughnutCanvas} id="tag-doughnut-chart-{selectedTagId}"></canvas>
          </div>
        </div>
      {/if}

      <!-- Expense list -->
      <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6">
        <h3 class="text-xs font-semibold text-neutral-400 uppercase tracking-wide mb-4">
          All Expenses — {t.expense_count} total
        </h3>
        {#if taggedExpenses.length === 0}
          <p class="text-neutral-600 text-xs text-center py-4">No expenses in local cache — reload or check the month filter.</p>
        {:else}
          <div class="overflow-x-auto -mx-1">
            <table class="w-full text-sm border-collapse" id="tag-expense-table-{selectedTagId}">
              <thead>
                <tr class="border-b border-neutral-800">
                  <th class="text-left text-xs font-medium text-neutral-500 pb-3 pr-4 pl-1">Date</th>
                  <th class="text-left text-xs font-medium text-neutral-500 pb-3 pr-4">Description</th>
                  <th class="text-left text-xs font-medium text-neutral-500 pb-3 pr-4">Category</th>
                  <th class="text-left text-xs font-medium text-neutral-500 pb-3 pr-4">Paid by</th>
                  <th class="text-right text-xs font-medium text-neutral-500 pb-3">Amount</th>
                </tr>
              </thead>
              <tbody>
                {#each taggedExpenses as exp (exp.id)}
                  <tr class="border-b border-neutral-800/60 hover:bg-neutral-800/30 transition-colors">
                    <td class="py-2.5 pr-4 pl-1 text-neutral-500 tabular-nums whitespace-nowrap text-xs">
                      {fmtDate(exp.expense_date)}
                    </td>
                    <td class="py-2.5 pr-4 text-neutral-200 max-w-[140px]" title={exp.name}>
                      <span class="block truncate text-xs">{exp.name}</span>
                    </td>
                    <td class="py-2.5 pr-4">
                      <span class="inline-flex items-center px-2 py-0.5 rounded-md bg-neutral-800 text-xs text-neutral-400 border border-neutral-700">
                        {exp.category}
                      </span>
                    </td>
                    <td class="py-2.5 pr-4 text-xs font-semibold text-neutral-300 tabular-nums">
                      {exp.who_paid}
                    </td>
                    <td class="py-2.5 text-right font-semibold text-neutral-100 tabular-nums text-xs whitespace-nowrap">
                      {$currencySymbol}{(exp.cost_cents / 100).toFixed(2)}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
          <p class="text-xs text-neutral-600 mt-3 text-right">
            Showing {taggedExpenses.length} of {t.expense_count} expenses (loaded in current session)
          </p>
        {/if}
      </div>
    {/if}
  </div>
</div>
