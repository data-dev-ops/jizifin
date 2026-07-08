<script>
  /**
   * ProjectsTab.svelte
   *
   * Full Projects management tab:
   *  - Create new project (name, target €, target date)
   *  - List all projects as rich stat cards
   *  - Edit any project inline
   *  - Delete with confirmation (expenses retain history)
   */

  import { projects, currencySymbol } from './stores.js';
  import { createProject, updateProject, deleteProject } from './api.js';

  // ── helpers ────────────────────────────────────────────────────────────────

  /** Integer cents → formatted amount */
  function fmtEur(cents) {
    return `${$currencySymbol}${(cents / 100).toLocaleString('en-GB', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  }

  /** YYYY-MM-DD → DD/MM/YYYY */
  function fmtDate(iso) {
    if (!iso) return '—';
    const [y, m, d] = iso.split('-');
    return `${d}/${m}/${y}`;
  }

  /** Clamp 0–100 */
  function pct(spent, target) {
    if (!target) return 0;
    return Math.min(100, Math.round((spent / target) * 100));
  }

  /** Human-readable remaining months estimate from est completion date string */
  function estLabel(estStr) {
    if (!estStr || estStr === 'Indefinite') return 'No payments yet';
    if (estStr === 'Completed') return '✓ Completed';
    const now = new Date();
    const est = new Date(estStr);
    const diffMs = est - now;
    if (diffMs <= 0) return '✓ Completed';
    const months = Math.ceil(diffMs / (1000 * 60 * 60 * 24 * 30.44));
    return `~${months} mo left (${fmtDate(estStr)})`;
  }

  // ── add-project form ───────────────────────────────────────────────────────

  let newName       = '';
  let newTargetEur  = '';
  let newTargetDate = '';
  let addSubmitting = false;
  let addError      = null;
  let addSuccess    = false;

  async function handleAdd(e) {
    e.preventDefault();
    addError = null;
    addSuccess = false;

    if (!newName.trim()) { addError = 'Project name required.'; return; }

    const parsed = parseFloat(newTargetEur);
    if (isNaN(parsed) || parsed <= 0) { addError = 'Target must be a positive amount.'; return; }
    const targetCents = Math.round(parsed * 100);

    if (!newTargetDate || !/^\d{4}-\d{2}-\d{2}$/.test(newTargetDate)) {
      addError = 'Valid target date required (YYYY-MM-DD).'; return;
    }

    addSubmitting = true;
    try {
      await createProject({ name: newName.trim(), target_cents: targetCents, target_date: newTargetDate });
      addSuccess = true;
      newName = ''; newTargetEur = ''; newTargetDate = '';
    } catch (err) {
      addError = err.message ?? 'Failed to create project.';
    } finally {
      addSubmitting = false;
    }
  }

  // ── per-card edit state ────────────────────────────────────────────────────

  let editingId      = null;
  let editName       = '';
  let editTargetEur  = '';
  let editTargetDate = '';
  let editSubmitting = false;
  let editError      = null;

  function startEdit(p) {
    editingId      = p.id;
    editName       = p.name;
    editTargetEur  = (p.target_cents / 100).toFixed(2);
    editTargetDate = p.target_date;
    editError      = null;
  }

  function cancelEdit() { editingId = null; editError = null; }

  async function handleEdit(e, id) {
    e.preventDefault();
    editError = null;
    const parsed = parseFloat(editTargetEur);
    if (isNaN(parsed) || parsed <= 0) { editError = 'Target must be positive.'; return; }
    if (!editTargetDate || !/^\d{4}-\d{2}-\d{2}$/.test(editTargetDate)) {
      editError = 'Valid target date required.'; return;
    }
    editSubmitting = true;
    try {
      await updateProject(id, {
        name: editName.trim(),
        target_cents: Math.round(parsed * 100),
        target_date: editTargetDate,
      });
      editingId = null;
    } catch (err) {
      editError = err.message ?? 'Failed to update project.';
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
      await deleteProject(id);
      confirmDeleteId = null;
    } catch (err) {
      deleteError = err.message ?? 'Delete failed.';
    } finally {
      deletingId = null;
    }
  }

  function barColor(p) {
    if (p >= 100) return 'from-emerald-500 to-emerald-400';
    if (p >= 60)  return 'from-indigo-500 to-violet-500';
    if (p >= 30)  return 'from-sky-600 to-indigo-500';
    return 'from-sky-700 to-sky-500';
  }
</script>

<div class="grid grid-cols-1 xl:grid-cols-5 gap-6">

  <!-- Add Project Form -->
  <div class="xl:col-span-2 bg-neutral-900 rounded-2xl border border-neutral-800 p-4 sm:p-6">
    <h2 class="text-sm font-semibold text-neutral-300 mb-5">New Project</h2>

    <form on:submit={handleAdd} id="add-project-form" class="space-y-4">
      <div>
        <label for="project-name" class="block text-xs font-medium text-neutral-400 mb-1.5">Project Name</label>
        <input
          id="project-name"
          type="text"
          maxlength="96"
          placeholder="e.g. New Laptop"
          bind:value={newName}
          class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2.5 text-sm
                 text-neutral-100 placeholder-neutral-600
                 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 transition-colors"
        />
      </div>

      <div>
        <label for="project-target" class="block text-xs font-medium text-neutral-400 mb-1.5">Target Amount ({$currencySymbol})</label>
        <div class="relative">
          <span class="absolute left-3 top-1/2 -translate-y-1/2 text-neutral-500 text-sm">{$currencySymbol}</span>
          <input
            id="project-target"
            type="number"
            min="0.01"
            step="0.01"
            placeholder="0.00"
            bind:value={newTargetEur}
            class="w-full bg-neutral-800 border border-neutral-700 rounded-lg pl-7 pr-3 py-2.5 text-sm
                   text-neutral-100 placeholder-neutral-600
                   focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 transition-colors"
          />
        </div>
      </div>

      <div>
        <label for="project-date" class="block text-xs font-medium text-neutral-400 mb-1.5">Target Completion Date</label>
        <input
          id="project-date"
          type="date"
          bind:value={newTargetDate}
          class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2.5 text-sm
                 text-neutral-100 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500
                 transition-colors [color-scheme:dark]"
        />
      </div>

      {#if addError}
        <p class="text-red-400 text-xs bg-red-950/40 border border-red-900 rounded-lg px-3 py-2">{addError}</p>
      {/if}
      {#if addSuccess}
        <p class="text-emerald-400 text-xs bg-emerald-950/40 border border-emerald-900 rounded-lg px-3 py-2">
          ✓ Project created successfully.
        </p>
      {/if}

      <button
        id="submit-project"
        type="submit"
        disabled={addSubmitting}
        class="w-full py-2.5 rounded-lg text-sm font-semibold
               bg-gradient-to-r from-indigo-600 to-violet-600
               hover:from-indigo-500 hover:to-violet-500
               disabled:opacity-50 disabled:cursor-not-allowed
               transition-all duration-150 shadow-md shadow-indigo-900/30 active:scale-[0.98]"
      >
        {addSubmitting ? 'Creating…' : 'Create Project'}
      </button>
    </form>
  </div>

  <!-- Project Cards -->
  <div class="xl:col-span-3 space-y-4">
    {#if $projects.length === 0}
      <div class="bg-neutral-900 rounded-2xl border border-neutral-800 p-10 flex flex-col items-center text-center">
        <div class="w-12 h-12 rounded-2xl bg-neutral-800 flex items-center justify-center text-2xl mb-4">▰</div>
        <p class="text-neutral-400 text-sm">No projects yet.</p>
        <p class="text-neutral-600 text-xs mt-1">Use the form to create your first savings goal.</p>
      </div>

    {:else}
      {#each $projects as project (project.id)}
        {@const progress = pct(project.total_spent_cents, project.target_cents)}
        <div
          id="project-card-{project.id}"
          class="bg-neutral-900 rounded-2xl border border-neutral-800 p-5 transition-all duration-150 hover:border-neutral-700"
        >
          {#if editingId === project.id}
            <!-- Edit Mode -->
            <form on:submit={(e) => handleEdit(e, project.id)} class="space-y-3">
              <input
                id="edit-name-{project.id}"
                type="text"
                maxlength="96"
                bind:value={editName}
                class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm
                       text-neutral-100 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500"
              />
              <div class="grid grid-cols-2 gap-3">
                <div class="relative">
                  <span class="absolute left-3 top-1/2 -translate-y-1/2 text-neutral-500 text-xs">{$currencySymbol}</span>
                  <input
                    id="edit-target-{project.id}"
                    type="number"
                    min="0.01"
                    step="0.01"
                    bind:value={editTargetEur}
                    class="w-full bg-neutral-800 border border-neutral-700 rounded-lg pl-6 pr-2 py-2 text-sm
                           text-neutral-100 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500"
                  />
                </div>
                <input
                  id="edit-date-{project.id}"
                  type="date"
                  bind:value={editTargetDate}
                  class="w-full bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm
                         text-neutral-100 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500
                         [color-scheme:dark]"
                />
              </div>
              {#if editError}
                <p class="text-red-400 text-xs">{editError}</p>
              {/if}
              <div class="flex gap-2">
                <button
                  id="save-edit-{project.id}"
                  type="submit"
                  disabled={editSubmitting}
                  class="flex-1 py-1.5 rounded-lg text-xs font-semibold bg-indigo-600 hover:bg-indigo-500
                         disabled:opacity-40 transition-colors"
                >{editSubmitting ? 'Saving…' : 'Save'}</button>
                <button
                  id="cancel-edit-{project.id}"
                  type="button"
                  on:click={cancelEdit}
                  class="flex-1 py-1.5 rounded-lg text-xs font-semibold bg-neutral-700 hover:bg-neutral-600 transition-colors"
                >Cancel</button>
              </div>
            </form>

          {:else}
            <!-- View Mode -->
            <div class="flex items-start justify-between gap-2 mb-4">
              <div>
                <h3 class="text-sm font-semibold text-neutral-100">{project.name}</h3>
                <p class="text-xs text-neutral-500 mt-0.5">Target: {fmtDate(project.target_date)}</p>
              </div>
              <div class="flex items-center gap-1.5 flex-none">
                <button
                  id="edit-project-{project.id}"
                  on:click={() => startEdit(project)}
                  title="Edit project"
                  class="p-1.5 rounded-lg text-neutral-500 hover:text-indigo-400 hover:bg-indigo-950/40 transition-all duration-150"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-3.5 h-3.5">
                    <path d="M5.433 13.917l1.262-3.155A4 4 0 0 1 7.58 9.42l6.92-6.918a2.121 2.121 0 0 1 3 3l-6.92 6.918c-.383.383-.84.685-1.343.886l-3.154 1.262a.5.5 0 0 1-.65-.65Z" />
                    <path d="M3.5 5.75c0-.69.56-1.25 1.25-1.25H10A.75.75 0 0 0 10 3H4.75A2.75 2.75 0 0 0 2 5.75v9.5A2.75 2.75 0 0 0 4.75 18h9.5A2.75 2.75 0 0 0 17 15.25V10a.75.75 0 0 0-1.5 0v5.25c0 .69-.56 1.25-1.25 1.25h-9.5c-.69 0-1.25-.56-1.25-1.25v-9.5Z" />
                  </svg>
                </button>

                {#if confirmDeleteId === project.id}
                  <span class="flex items-center gap-1.5">
                    <span class="text-[10px] text-neutral-400">Delete?</span>
                    <button
                      id="confirm-delete-project-{project.id}"
                      on:click={() => handleDelete(project.id)}
                      disabled={deletingId === project.id}
                      class="px-2 py-0.5 rounded text-xs font-semibold bg-red-600 hover:bg-red-500 disabled:opacity-40 transition-colors"
                    >{deletingId === project.id ? '…' : 'Yes'}</button>
                    <button
                      id="cancel-delete-project-{project.id}"
                      on:click={() => { confirmDeleteId = null; deleteError = null; }}
                      class="px-2 py-0.5 rounded text-xs font-semibold bg-neutral-700 hover:bg-neutral-600 transition-colors"
                    >No</button>
                  </span>
                {:else}
                  <button
                    id="delete-project-{project.id}"
                    on:click={() => { confirmDeleteId = project.id; deleteError = null; }}
                    title="Delete project"
                    class="p-1.5 rounded-lg text-neutral-500 hover:text-red-400 hover:bg-red-950/40 transition-all duration-150"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-3.5 h-3.5">
                      <path fill-rule="evenodd" d="M8.75 1A2.75 2.75 0 0 0 6 3.75v.443c-.795.077-1.584.176-2.365.298a.75.75 0 1 0 .23 1.482l.149-.022.841 10.518A2.75 2.75 0 0 0 7.596 19h4.807a2.75 2.75 0 0 0 2.742-2.53l.841-10.52.149.023a.75.75 0 0 0 .23-1.482A41.03 41.03 0 0 0 14 4.193V3.75A2.75 2.75 0 0 0 11.25 1h-2.5ZM10 4c.84 0 1.673.025 2.5.075V3.75c0-.69-.56-1.25-1.25-1.25h-2.5c-.69 0-1.25.56-1.25 1.25v.325C8.327 4.025 9.16 4 10 4ZM8.58 7.72a.75.75 0 0 0-1.5.06l.3 7.5a.75.75 0 1 0 1.5-.06l-.3-7.5Zm4.34.06a.75.75 0 1 0-1.5-.06l-.3 7.5a.75.75 0 1 0 1.5.06l.3-7.5Z" clip-rule="evenodd" />
                    </svg>
                  </button>
                {/if}
              </div>
            </div>

            <!-- Amount + progress bar -->
            <div class="mb-3">
              <div class="flex justify-between text-xs mb-1.5">
                <span class="text-neutral-400">
                  {fmtEur(project.total_spent_cents)}
                  <span class="text-neutral-600">/ {fmtEur(project.target_cents)}</span>
                </span>
                <span class="{progress >= 100 ? 'text-emerald-400' : 'text-neutral-300'} font-semibold tabular-nums">
                  {progress}%
                </span>
              </div>
              <div class="w-full h-2 bg-neutral-800 rounded-full overflow-hidden">
                <div
                  class="h-full rounded-full bg-gradient-to-r {barColor(progress)} transition-all duration-700"
                  style="width: {progress}%"
                ></div>
              </div>
            </div>

            <!-- Metrics grid -->
            <div class="grid grid-cols-3 gap-3 mt-4">
              <div class="bg-neutral-800/60 rounded-xl p-3">
                <p class="text-[10px] text-neutral-500 mb-1 uppercase tracking-wide">Last Payment</p>
                {#if project.last_payment}
                  <p class="text-xs font-semibold text-neutral-200">{fmtEur(project.last_payment.cost_cents)}</p>
                  <p class="text-[10px] text-neutral-500 mt-0.5">{fmtDate(project.last_payment.expense_date)}</p>
                  <p class="text-[10px] text-sky-400 mt-0.5">{project.last_payment.who_paid}</p>
                {:else}
                  <p class="text-xs text-neutral-600">None yet</p>
                {/if}
              </div>

              <div class="bg-neutral-800/60 rounded-xl p-3">
                <p class="text-[10px] text-neutral-500 mb-1 uppercase tracking-wide">Avg / Month</p>
                {#if project.avg_monthly_payment_cents > 0}
                  <p class="text-xs font-semibold text-neutral-200">{fmtEur(project.avg_monthly_payment_cents)}</p>
                  <p class="text-[10px] text-neutral-500 mt-0.5">per month</p>
                {:else}
                  <p class="text-xs text-neutral-600">No data</p>
                {/if}
              </div>

              <div class="bg-neutral-800/60 rounded-xl p-3">
                <p class="text-[10px] text-neutral-500 mb-1 uppercase tracking-wide">Est. Done</p>
                <p class="text-xs font-semibold
                  {project.estimated_completion_date === 'Completed'
                    ? 'text-emerald-400'
                    : project.estimated_completion_date === 'Indefinite'
                    ? 'text-neutral-600'
                    : 'text-indigo-300'}">
                  {estLabel(project.estimated_completion_date)}
                </p>
              </div>
            </div>

            {#if deleteError && confirmDeleteId === project.id}
              <p class="text-xs text-red-400 mt-2">{deleteError}</p>
            {/if}
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</div>
