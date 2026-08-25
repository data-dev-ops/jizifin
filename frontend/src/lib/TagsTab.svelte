<script>
  /**
   * TagsTab.svelte
   *
   * Full Tags management tab with full light/dark theme support:
   *  - Create new tag (name, color, optional description, active dates)
   *  - List all tags as cards with all-time totals
   *  - Edit any tag inline (name, color, description, dates, active toggle)
   *  - Delete with confirmation (expenses become untagged, not deleted)
   *  - Select a tag to see its full cross-month detail view:
   *      · Summary stat cards
   *      · Bar chart: spending by month (Chart.js with theme reactivity)
   *      · Doughnut chart: spending by category (Chart.js with theme reactivity)
   *      · Full expense list for this tag
   */

  import { onMount, onDestroy, tick } from 'svelte';
  import { tags, expenses, currencySymbol, theme } from './stores.js';
  import { createTag, updateTag, deleteTag, fetchTagAnalytics } from './api.js';
  import { getRandomPastelColor } from './colorUtils.js';
  import Chart from 'chart.js/auto';

  function getIsDark() {
    if (typeof document === 'undefined') return true;
    return document.documentElement.classList.contains('dark');
  }

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
  let newColor       = getRandomPastelColor();
  let newDescription = '';
  let newStartDate   = '';
  let newEndDate     = '';
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
        start_date:  newStartDate || null,
        end_date:    newEndDate || null,
      });
      addSuccess    = true;
      newName        = '';
      newColor       = getRandomPastelColor();
      newDescription = '';
      newStartDate   = '';
      newEndDate     = '';
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
  let editStartDate  = '';
  let editEndDate    = '';
  let editIsActive   = true;
  let editSubmitting = false;
  let editError      = null;

  function startEdit(tag) {
    editingId      = tag.id;
    editName       = tag.name;
    editColor      = tag.color;
    editDesc       = tag.description ?? '';
    editStartDate  = tag.start_date ?? '';
    editEndDate    = tag.end_date ?? '';
    editIsActive   = tag.is_active !== false && tag.is_active !== 0;
    editError      = null;
  }

  function cancelEdit() {
    editingId  = null;
    editError  = null;
  }

  async function handleEdit(e, tagId) {
    e.preventDefault();
    editError = null;
    if (!editName.trim()) { editError = 'Tag name is required.'; return; }
    editSubmitting = true;
    try {
      await updateTag(tagId, {
        name:        editName.trim(),
        color:       editColor,
        description: editDesc.trim() || null,
        start_date:  editStartDate || null,
        end_date:    editEndDate || null,
        is_active:   editIsActive ? 1 : 0,
      });
      editingId = null;
      if (selectedTagId === tagId) {
        await loadTagDetail(tagId);
      }
    } catch (err) {
      editError = err.message ?? 'Failed to update tag.';
    } finally {
      editSubmitting = false;
    }
  }

  // ── delete tag state ───────────────────────────────────────────────────────

  let confirmDeleteId = null;
  let deletingId      = null;
  let deleteError     = null;

  async function handleDelete(tagId) {
    deletingId  = tagId;
    deleteError = null;
    try {
      await deleteTag(tagId);
      if (selectedTagId === tagId) {
        selectedTagId = null;
        tagDetail     = null;
      }
      confirmDeleteId = null;
    } catch (err) {
      deleteError = err.message ?? 'Failed to delete tag.';
    } finally {
      deletingId = null;
    }
  }

  // ── Quick Active/Closed Toggle ─────────────────────────────────────────────
  let statusUpdatingId = null;
  async function toggleTagActive(tag) {
    statusUpdatingId = tag.id;
    try {
      const isCurrentlyActive = tag.is_active !== false && tag.is_active !== 0;
      const nextActive = !isCurrentlyActive;
      await updateTag(tag.id, { is_active: nextActive });
      if (selectedTagId === tag.id) {
        await loadTagDetail(tag.id);
      }
    } catch (err) {
      console.error("Failed to toggle tag status:", err);
    } finally {
      statusUpdatingId = null;
    }
  }

  // ── detail view state ──────────────────────────────────────────────────────

  let selectedTagId = null;
  let tagDetail     = null;
  let detailLoading = false;
  let detailError   = null;

  async function loadTagDetail(id) {
    detailLoading = true;
    detailError   = null;
    try {
      tagDetail = await fetchTagAnalytics(id);
    } catch (err) {
      detailError = err.message ?? 'Failed to load tag details.';
    } finally {
      detailLoading = false;
    }
  }

  async function selectTag(id) {
    if (selectedTagId === id) {
      selectedTagId = null;
      tagDetail     = null;
      return;
    }
    selectedTagId = id;
    await loadTagDetail(id);
  }

  // ── Chart.js instances ─────────────────────────────────────────────────────

  let barCanvas;
  let doughnutCanvas;
  let barChart;
  let doughnutChart;

  /**
   * Rebuild both charts whenever tagDetail or theme changes.
   */
  $: if (tagDetail && barCanvas && doughnutCanvas) {
    tick().then(() => renderCharts());
  }

  $: if ($theme && (barChart || doughnutChart)) {
    tick().then(() => renderCharts());
  }

  function renderCharts() {
    const isDark = getIsDark();
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
              backgroundColor: isDark ? 'rgba(15, 23, 42, 0.95)' : 'rgba(255, 255, 255, 0.95)',
              borderColor:     isDark ? 'rgba(99, 102, 241, 0.4)' : 'rgba(99, 102, 241, 0.25)',
              borderWidth:     1,
              titleColor:      isDark ? '#f1f5f9' : '#0f172a',
              bodyColor:       isDark ? '#cbd5e1' : '#475569',
              callbacks: {
                label: (ctx) => ` ${$currencySymbol}${Number(ctx.raw).toFixed(2)}`,
              },
            },
          },
          scales: {
            x: {
              grid:  { color: isDark ? 'rgba(255, 255, 255, 0.05)' : 'rgba(0, 0, 0, 0.05)' },
              ticks: { color: isDark ? '#9ca3af' : '#64748b', font: { size: 11 } },
            },
            y: {
              grid:  { color: isDark ? 'rgba(255, 255, 255, 0.05)' : 'rgba(0, 0, 0, 0.05)' },
              ticks: { color: isDark ? '#9ca3af' : '#64748b', font: { size: 11 }, callback: (v) => `${$currencySymbol}${v}` },
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
        const hue = (parseInt(color.replace('#',''), 16) % 360 + i * 37) % 360;
        return `hsl(${hue}, 70%, 55%)`;
      });
      doughnutChart = new Chart(doughnutCanvas, {
        type: 'doughnut',
        data: {
          labels: tagDetail.by_category.map((r) => r.category),
          datasets: [{
            data:            tagDetail.by_category.map((r) => r.total_amount),
            backgroundColor: palette.map((c) => c.replace('55%', '45%')),
            borderColor:     isDark ? '#080c14' : '#ffffff',
            borderWidth:     2,
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
              labels: { color: isDark ? '#d1d5db' : '#334155', font: { size: 11 }, boxWidth: 12, padding: 10 },
            },
            tooltip: {
              backgroundColor: isDark ? 'rgba(15, 23, 42, 0.95)' : 'rgba(255, 255, 255, 0.95)',
              borderColor:     isDark ? 'rgba(99, 102, 241, 0.4)' : 'rgba(99, 102, 241, 0.25)',
              borderWidth:     1,
              titleColor:      isDark ? '#f1f5f9' : '#0f172a',
              bodyColor:       isDark ? '#cbd5e1' : '#475569',
              callbacks: {
                label: (ctx) => ` ${$currencySymbol}${Number(ctx.raw).toFixed(2)}`,
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
    <div class="card">
      <h2 class="text-sm font-semibold text-neutral-800 dark:text-neutral-200 mb-5">New Tag</h2>

      <form on:submit={handleAdd} id="add-tag-form" class="space-y-4">

        <div>
          <label for="tag-name" class="block text-xs font-medium text-neutral-600 dark:text-neutral-400 mb-1.5">Tag Name</label>
          <input
            id="tag-name"
            type="text"
            maxlength="96"
            placeholder="e.g. Paris 2025, Bathroom Reno"
            bind:value={newName}
            class="input-field"
          />
        </div>

        <div>
          <label for="tag-description" class="block text-xs font-medium text-neutral-600 dark:text-neutral-400 mb-1.5">
            Description <span class="text-neutral-500">(optional)</span>
          </label>
          <input
            id="tag-description"
            type="text"
            maxlength="512"
            placeholder="e.g. Summer 2025 — flights, hotels, food"
            bind:value={newDescription}
            class="input-field"
          />
        </div>

        <div>
          <label for="tag-color" class="block text-xs font-medium text-neutral-600 dark:text-neutral-400 mb-1.5">Color</label>
          <div class="flex items-center gap-2">
            <label
              for="tag-color"
              class="relative flex items-center justify-center w-10 h-10 rounded-xl border border-neutral-300 dark:border-neutral-700/80 bg-white dark:bg-neutral-800 cursor-pointer overflow-hidden shadow-inner hover:border-neutral-400 dark:hover:border-neutral-500 transition-colors"
              title="Pick tag color"
            >
              <span class="w-6 h-6 rounded-lg shadow-sm" style="background-color: {newColor}"></span>
              <input
                id="tag-color"
                type="color"
                bind:value={newColor}
                class="sr-only"
              />
            </label>
            <button
              type="button"
              on:click={() => (newColor = getRandomPastelColor())}
              class="p-2.5 rounded-xl bg-neutral-100 dark:bg-neutral-800 hover:bg-neutral-200 dark:hover:bg-neutral-700 border border-neutral-200 dark:border-neutral-700/80 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-white transition-colors"
              title="Randomize pastel color"
              aria-label="Randomize pastel color"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
            </button>
            <span class="text-xs font-mono text-neutral-600 dark:text-neutral-400 uppercase tracking-wider">{newColor}</span>
          </div>
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div>
            <label for="tag-start-date" class="block text-xs font-medium text-neutral-600 dark:text-neutral-400 mb-1.5">
              Start Date <span class="text-neutral-500">(optional)</span>
            </label>
            <input
              id="tag-start-date"
              type="date"
              bind:value={newStartDate}
              class="input-field"
            />
          </div>
          <div>
            <label for="tag-end-date" class="block text-xs font-medium text-neutral-600 dark:text-neutral-400 mb-1.5">
              End Date <span class="text-neutral-500">(optional)</span>
            </label>
            <input
              id="tag-end-date"
              type="date"
              bind:value={newEndDate}
              class="input-field"
            />
          </div>
        </div>

        {#if addError}
          <p class="text-rose-700 dark:text-red-400 text-xs bg-rose-50 dark:bg-red-950/40 border border-rose-200 dark:border-red-800 rounded-xl px-3 py-2">{addError}</p>
        {/if}
        {#if addSuccess}
          <p class="text-emerald-700 dark:text-emerald-400 text-xs bg-emerald-50 dark:bg-emerald-950/40 border border-emerald-200 dark:border-emerald-800 rounded-xl px-3 py-2">
            ✓ Tag created successfully.
          </p>
        {/if}

        <button
          id="submit-tag"
          type="submit"
          disabled={addSubmitting}
          class="btn-primary w-full"
        >
          {addSubmitting ? 'Creating…' : 'Create Tag'}
        </button>
      </form>
    </div>

    <!-- Tag Cards List -->
    {#if $tags.length === 0}
      <div class="card empty-state-box">
        <div class="w-12 h-12 rounded-2xl bg-neutral-100 dark:bg-neutral-800 flex items-center justify-center text-2xl mb-4">🏷</div>
        <p class="text-neutral-700 dark:text-neutral-300 text-sm font-semibold">No tags yet.</p>
        <p class="text-neutral-500 text-xs mt-1">Create a tag to start grouping expenses across months.</p>
      </div>
    {:else}
      {@const activeCount = $tags.filter(t => t.is_active !== false && t.is_active !== 0).length}
      {@const closedCount = $tags.length - activeCount}
      <div class="flex items-center justify-between text-xs text-neutral-500 dark:text-neutral-400 px-1 mb-1">
        <span>Tags Overview</span>
        <span class="text-[11px] text-neutral-500">
          <strong class="text-emerald-600 dark:text-emerald-400 font-semibold">{activeCount}</strong> active
          {#if closedCount > 0}
            · <strong class="text-neutral-500 dark:text-neutral-400 font-semibold">{closedCount}</strong> closed
          {/if}
        </span>
      </div>

      {#each $tags as tag (tag.id)}
        {@const isActive = tag.is_active !== false && tag.is_active !== 0}
        <div
          id="tag-card-{tag.id}"
          class="card p-4 transition-all duration-150 cursor-pointer
                 {selectedTagId === tag.id ? 'border-amber-500 shadow-sm shadow-amber-500/10 dark:shadow-amber-900/30 ring-1 ring-amber-500/50' : 'hover:border-neutral-300 dark:hover:border-neutral-700'}
                 {!isActive ? 'opacity-75 bg-neutral-100/50 dark:bg-neutral-950/80' : ''}"
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
                  class="input-field py-2 text-sm"
                />
                <input
                  id="edit-tag-desc-{tag.id}"
                  type="text"
                  maxlength="512"
                  placeholder="Description (optional)"
                  bind:value={editDesc}
                  class="input-field py-2 text-sm"
                />
                <div class="grid grid-cols-2 gap-2">
                  <div>
                    <label for="edit-tag-start-{tag.id}" class="text-[10px] text-neutral-500 dark:text-neutral-400 block mb-0.5">Start Date</label>
                    <input
                      id="edit-tag-start-{tag.id}"
                      type="date"
                      bind:value={editStartDate}
                      class="input-field py-1 text-xs"
                    />
                  </div>
                  <div>
                    <label for="edit-tag-end-{tag.id}" class="text-[10px] text-neutral-500 dark:text-neutral-400 block mb-0.5">End Date</label>
                    <input
                      id="edit-tag-end-{tag.id}"
                      type="date"
                      bind:value={editEndDate}
                      class="input-field py-1 text-xs"
                    />
                  </div>
                </div>

                <div class="flex items-center justify-between gap-3 pt-1">
                  <div class="flex items-center gap-2">
                    <label for="edit-tag-color-{tag.id}" class="text-xs text-neutral-600 dark:text-neutral-400">Color</label>
                    <label
                      for="edit-tag-color-{tag.id}"
                      class="relative flex items-center justify-center w-8 h-8 rounded-lg border border-neutral-300 dark:border-neutral-700 bg-white dark:bg-neutral-800 cursor-pointer overflow-hidden hover:border-neutral-500 transition-colors shadow-sm"
                      title="Pick color"
                    >
                      <span class="w-5 h-5 rounded-md shadow-sm" style="background-color: {editColor}"></span>
                      <input
                        id="edit-tag-color-{tag.id}"
                        type="color"
                        bind:value={editColor}
                        class="sr-only"
                      />
                    </label>
                  </div>
                  <label class="flex items-center gap-2 text-xs text-neutral-700 dark:text-neutral-300 cursor-pointer select-none">
                    <input
                      id="edit-tag-active-{tag.id}"
                      type="checkbox"
                      bind:checked={editIsActive}
                      class="w-4 h-4 rounded border-neutral-300 dark:border-neutral-700 bg-white dark:bg-neutral-800 text-indigo-600 focus:ring-indigo-500 cursor-pointer"
                    />
                    <span>Active Tag</span>
                  </label>
                </div>
                {#if editError}
                  <p class="text-rose-600 dark:text-red-400 text-xs">{editError}</p>
                {/if}
                <div class="flex gap-2">
                  <button
                    id="save-tag-edit-{tag.id}"
                    type="submit"
                    disabled={editSubmitting}
                    class="btn-primary flex-1 py-1.5 text-xs bg-amber-600 hover:bg-amber-500"
                  >{editSubmitting ? 'Saving…' : 'Save'}</button>
                  <button
                    id="cancel-tag-edit-{tag.id}"
                    type="button"
                    on:click={cancelEdit}
                    class="btn-secondary flex-1 py-1.5 text-xs"
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
                  <div class="flex items-center gap-2 flex-wrap">
                    <h3 class="text-sm font-semibold {isActive ? 'text-neutral-900 dark:text-neutral-100' : 'text-neutral-500 dark:text-neutral-400'} truncate">{tag.name}</h3>
                    {#if tag.is_joint}
                      <span class="badge-indigo">
                        🏦 Joint Tag
                      </span>
                    {/if}

                    <!-- Active/Closed Status Toggle Badge -->
                    <!-- svelte-ignore a11y-click-events-have-key-events -->
                    <!-- svelte-ignore a11y-no-static-element-interactions -->
                    <div on:click|stopPropagation class="inline-flex">
                      <button
                        id="toggle-tag-active-{tag.id}"
                        type="button"
                        on:click={() => toggleTagActive(tag)}
                        disabled={statusUpdatingId === tag.id}
                        title={isActive ? 'Click to close off/archive this tag' : 'Click to reactivate this tag'}
                        class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold transition-colors cursor-pointer
                               {isActive
                                 ? 'badge-emerald hover:opacity-90'
                                 : 'badge-neutral hover:opacity-90'}"
                      >
                        <span class="w-1.5 h-1.5 rounded-full {isActive ? 'bg-emerald-500' : 'bg-neutral-400'}"></span>
                        {isActive ? 'Active' : 'Closed'}
                      </button>
                    </div>
                  </div>
                  {#if tag.description}
                    <p class="text-[11px] text-neutral-500 truncate mt-0.5">{tag.description}</p>
                  {/if}
                  {#if tag.start_date || tag.end_date}
                    <div class="mt-1 flex items-center gap-1 text-[10px] text-amber-600 dark:text-amber-400 font-mono">
                      <span>📅</span>
                      <span>{tag.start_date ? fmtDate(tag.start_date) : '…'} → {tag.end_date ? fmtDate(tag.end_date) : '…'}</span>
                    </div>
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
                  class="p-1.5 rounded-lg text-neutral-400 hover:text-amber-600 dark:hover:text-amber-400 hover:bg-neutral-100 dark:hover:bg-amber-950/40 transition-all duration-150"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-3.5 h-3.5">
                    <path d="M5.433 13.917l1.262-3.155A4 4 0 0 1 7.58 9.42l6.92-6.918a2.121 2.121 0 0 1 3 3l-6.92 6.918c-.383.383-.84.685-1.343.886l-3.154 1.262a.5.5 0 0 1-.65-.65Z" />
                    <path d="M3.5 5.75c0-.69.56-1.25 1.25-1.25H10A.75.75 0 0 0 10 3H4.75A2.75 2.75 0 0 0 2 5.75v9.5A2.75 2.75 0 0 0 4.75 18h9.5A2.75 2.75 0 0 0 17 15.25V10a.75.75 0 0 0-1.5 0v5.25c0 .69-.56 1.25-1.25 1.25h-9.5c-.69 0-1.25-.56-1.25-1.25v-9.5Z" />
                  </svg>
                </button>

                {#if confirmDeleteId === tag.id}
                  <span class="flex items-center gap-1">
                    <span class="text-[10px] text-neutral-500 dark:text-neutral-400">Delete?</span>
                    <button
                      id="confirm-delete-tag-{tag.id}"
                      on:click={() => handleDelete(tag.id)}
                      disabled={deletingId === tag.id}
                      class="px-2 py-0.5 rounded text-xs font-semibold bg-rose-600 hover:bg-rose-500 text-white disabled:opacity-40 transition-colors"
                    >{deletingId === tag.id ? '…' : 'Yes'}</button>
                    <button
                      id="cancel-delete-tag-{tag.id}"
                      on:click={() => { confirmDeleteId = null; deleteError = null; }}
                      class="px-2 py-0.5 rounded text-xs font-semibold bg-neutral-200 dark:bg-neutral-700 hover:bg-neutral-300 dark:hover:bg-neutral-600 text-neutral-800 dark:text-neutral-200 transition-colors"
                    >No</button>
                  </span>
                {:else}
                  <button
                    id="delete-tag-{tag.id}"
                    on:click={() => { confirmDeleteId = tag.id; deleteError = null; }}
                    title="Delete tag"
                    class="p-1.5 rounded-lg text-neutral-400 hover:text-rose-600 dark:hover:text-red-400 hover:bg-neutral-100 dark:hover:bg-red-950/40 transition-all duration-150"
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
              <span class="text-neutral-300 dark:text-neutral-600">·</span>
              <span class="text-neutral-500">{tag.expense_count} expense{tag.expense_count !== 1 ? 's' : ''}</span>
              {#if tag.first_date}
                <span class="text-neutral-300 dark:text-neutral-600">·</span>
                <span class="text-neutral-500">{fmtDate(tag.first_date)} → {fmtDate(tag.last_date)}</span>
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
      <div class="card p-14 flex flex-col items-center text-center">
        <div class="w-14 h-14 rounded-2xl bg-neutral-100 dark:bg-neutral-800 flex items-center justify-center text-3xl mb-4">🏷</div>
        <p class="text-neutral-800 dark:text-neutral-300 font-semibold text-sm">Select a tag</p>
        <p class="text-neutral-500 text-xs mt-1 max-w-xs">
          Click a tag on the left to see its full spending breakdown across all months.
        </p>
      </div>

    {:else if detailLoading}
      <div class="card p-14 flex items-center justify-center">
        <div class="w-8 h-8 rounded-full border-2 border-amber-500 border-t-transparent animate-spin"></div>
      </div>

    {:else if detailError}
      <div class="card border-rose-200 dark:border-red-900/40 p-8 text-center">
        <p class="text-rose-600 dark:text-red-400 text-sm">{detailError}</p>
      </div>

    {:else if tagDetail}
      {@const t = tagDetail.tag}
      {@const avg = avgPerMonth(t)}
      {@const detailActive = t.is_active !== false && t.is_active !== 0}

      <!-- Summary stat cards -->
      <div class="card p-4 sm:p-6 space-y-5">
        <div class="flex items-center justify-between gap-2.5 flex-wrap">
          <div class="flex items-center gap-2.5 min-w-0">
            <span class="w-3 h-3 rounded-full flex-none" style="background-color: {t.color}"></span>
            <h2 class="text-sm font-semibold text-neutral-900 dark:text-neutral-200">{t.name}</h2>
            {#if t.description}
              <span class="text-xs text-neutral-500 truncate">— {t.description}</span>
            {/if}
          </div>
          <button
            id="detail-toggle-tag-active"
            type="button"
            on:click={() => toggleTagActive(t)}
            disabled={statusUpdatingId === t.id}
            class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold transition-colors cursor-pointer
                   {detailActive
                     ? 'badge-emerald hover:opacity-90'
                     : 'badge-neutral hover:opacity-90'}"
          >
            <span class="w-1.5 h-1.5 rounded-full {detailActive ? 'bg-emerald-500' : 'bg-neutral-400'}"></span>
            {detailActive ? 'Active Group' : 'Closed Group'}
          </button>
        </div>

        <div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
          <div class="card-sub p-3">
            <p class="text-[10px] text-neutral-500 mb-1 uppercase tracking-wide">Total Spent</p>
            <p class="text-sm font-bold" style="color: {t.color}">{fmtAmt(t.total_amount)}</p>
          </div>
          <div class="card-sub p-3">
            <p class="text-[10px] text-neutral-500 mb-1 uppercase tracking-wide">Expenses</p>
            <p class="text-sm font-bold text-neutral-900 dark:text-neutral-100">{t.expense_count}</p>
          </div>
          <div class="card-sub p-3">
            <p class="text-[10px] text-neutral-500 mb-1 uppercase tracking-wide">Avg / Month</p>
            <p class="text-sm font-bold text-neutral-900 dark:text-neutral-100">{avg ? fmtAmt(avg) : '—'}</p>
          </div>
          <div class="card-sub p-3">
            <p class="text-[10px] text-neutral-500 mb-1 uppercase tracking-wide">Date Range</p>
            <p class="text-xs font-semibold text-neutral-800 dark:text-neutral-300">
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
        <div class="card p-4 sm:p-6 space-y-4">
          <h3 class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wide">Spending Over Time</h3>
          <div class="h-48">
            <canvas bind:this={barCanvas} id="tag-bar-chart-{selectedTagId}"></canvas>
          </div>
        </div>
      {/if}

      <!-- Doughnut chart: by category -->
      {#if tagDetail.by_category.length > 0}
        <div class="card p-4 sm:p-6 space-y-4">
          <h3 class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wide">By Category</h3>
          <div class="h-52">
            <canvas bind:this={doughnutCanvas} id="tag-doughnut-chart-{selectedTagId}"></canvas>
          </div>
        </div>
      {/if}

      <!-- Expense list -->
      <div class="card p-4 sm:p-6 space-y-4">
        <h3 class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wide">
          All Expenses — {t.expense_count} total
        </h3>
        {#if taggedExpenses.length === 0}
          <p class="text-neutral-500 text-xs text-center py-4">No expenses in local cache — reload or check the month filter.</p>
        {:else}
          <div class="overflow-x-auto -mx-1">
            <table class="w-full text-sm border-collapse" id="tag-expense-table-{selectedTagId}">
              <thead>
                <tr class="border-b border-neutral-200 dark:border-neutral-800">
                  <th class="text-left text-xs font-medium text-neutral-500 pb-3 pr-4 pl-1">Date</th>
                  <th class="text-left text-xs font-medium text-neutral-500 pb-3 pr-4">Description</th>
                  <th class="text-left text-xs font-medium text-neutral-500 pb-3 pr-4">Category</th>
                  <th class="text-left text-xs font-medium text-neutral-500 pb-3 pr-4">Paid by</th>
                  <th class="text-right text-xs font-medium text-neutral-500 pb-3">Amount</th>
                </tr>
              </thead>
              <tbody>
                {#each taggedExpenses as exp (exp.id)}
                  <tr class="border-b border-neutral-200/60 dark:border-neutral-800/60 hover:bg-neutral-100/60 dark:hover:bg-neutral-800/30 transition-colors">
                    <td class="py-2.5 pr-4 pl-1 text-neutral-500 tabular-nums whitespace-nowrap text-xs">
                      {fmtDate(exp.expense_date)}
                    </td>
                    <td class="py-2.5 pr-4 text-neutral-900 dark:text-neutral-200 max-w-[140px]" title={exp.name}>
                      <span class="block truncate text-xs">{exp.name}</span>
                    </td>
                    <td class="py-2.5 pr-4">
                      <span class="badge-neutral">
                        {exp.category}
                      </span>
                    </td>
                    <td class="py-2.5 pr-4 text-xs font-semibold text-neutral-800 dark:text-neutral-300 tabular-nums">
                      {exp.who_paid}
                    </td>
                    <td class="py-2.5 text-right font-semibold text-neutral-900 dark:text-neutral-100 tabular-nums text-xs whitespace-nowrap">
                      {$currencySymbol}{(exp.cost_cents / 100).toFixed(2)}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
          <p class="text-xs text-neutral-500 mt-3 text-right">
            Showing {taggedExpenses.length} of {t.expense_count} expenses (loaded in current session)
          </p>
        {/if}
      </div>
    {/if}
  </div>
</div>
