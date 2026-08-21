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

  import { projects, users, currencySymbol, showProjectsInExpense } from './stores.js';
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

  /** Look up user color */
  function userColor(name) {
    return $users.find((u) => u.name === name)?.color ?? '#6366f1';
  }

  $: activeUsers = $users.filter((u) => u.is_active !== false);

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

  let newName               = '';
  let newTargetEur          = '';
  let newTargetDate         = '';
  let newIsJoint            = false;
  let newAllowSubcategories = true;
  let newMembers            = [];
  let addSubmitting         = false;
  let addError              = null;
  let addSuccess            = false;

  function toggleNewMember(userName) {
    if (newMembers.includes(userName)) {
      newMembers = newMembers.filter((u) => u !== userName);
    } else {
      newMembers = [...newMembers, userName];
    }
  }

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
      const payload = {
        name: newName.trim(),
        target_cents: targetCents,
        target_date: newTargetDate,
        is_joint: newIsJoint,
        allow_subcategories: newAllowSubcategories,
      };
      if (newMembers.length > 0) {
        payload.user_names = newMembers;
      }
      await createProject(payload);
      addSuccess = true;
      newName = ''; newTargetEur = ''; newTargetDate = ''; newIsJoint = false; newAllowSubcategories = true; newMembers = [];
    } catch (err) {
      addError = err.message ?? 'Failed to create project.';
    } finally {
      addSubmitting = false;
    }
  }

  // ── per-card edit state ────────────────────────────────────────────────────

  let editingId              = null;
  let editName               = '';
  let editTargetEur          = '';
  let editTargetDate         = '';
  let editIsJoint            = false;
  let editAllowSubcategories = true;
  let editMembers            = [];
  let editSubmitting         = false;
  let editError              = null;

  function startEdit(p) {
    editingId              = p.id;
    editName               = p.name;
    editTargetEur          = (p.target_cents / 100).toFixed(2);
    editTargetDate         = p.target_date;
    editIsJoint            = Boolean(p.is_joint);
    editAllowSubcategories = p.allow_subcategories !== false;
    editMembers            = p.user_names ? [...p.user_names] : [];
    editError              = null;
  }

  function toggleEditMember(userName) {
    if (editMembers.includes(userName)) {
      editMembers = editMembers.filter((u) => u !== userName);
    } else {
      editMembers = [...editMembers, userName];
    }
  }

  function cancelEdit() {
    editingId = null;
  }

  async function handleEdit(e, id) {
    e.preventDefault();
    editError = null;
    if (!editName.trim()) { editError = 'Project name required.'; return; }

    const parsed = parseFloat(editTargetEur);
    if (isNaN(parsed) || parsed <= 0) { editError = 'Target must be a positive amount.'; return; }
    const targetCents = Math.round(parsed * 100);

    if (!editTargetDate || !/^\d{4}-\d{2}-\d{2}$/.test(editTargetDate)) {
      editError = 'Valid target date required (YYYY-MM-DD).'; return;
    }

    editSubmitting = true;
    try {
      const payload = {
        name: editName.trim(),
        target_cents: targetCents,
        target_date: editTargetDate,
        is_joint: editIsJoint,
        allow_subcategories: editAllowSubcategories,
      };
      if (editMembers.length > 0) {
        payload.user_names = editMembers;
      }
      await updateProject(id, payload);
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
  <div class="xl:col-span-2 card">
    <h2 class="text-sm font-semibold text-neutral-200 mb-5">New Project</h2>

    <form on:submit={handleAdd} id="add-project-form" class="space-y-4">
      <div>
        <label for="project-name" class="block text-xs font-medium text-neutral-400 mb-1.5">Project Name</label>
        <input
          id="project-name"
          type="text"
          maxlength="96"
          placeholder="e.g. New Laptop"
          bind:value={newName}
          class="input-field"
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
            class="input-field pl-7"
          />
        </div>
      </div>

      <div>
        <label for="project-date" class="block text-xs font-medium text-neutral-400 mb-1.5">Target Completion Date</label>
        <input
          id="project-date"
          type="date"
          bind:value={newTargetDate}
          class="input-field"
        />
      </div>

      <!-- Member Assignment -->
      <div>
        <label class="block text-xs font-medium text-neutral-400 mb-1.5">Assigned Members</label>
        <div class="flex flex-wrap gap-2">
          {#each activeUsers as u}
            {@const isChecked = newMembers.includes(u.name)}
            <button
              type="button"
              on:click={() => toggleNewMember(u.name)}
              class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium border transition-all cursor-pointer {isChecked ? 'bg-indigo-600/20 border-indigo-500 text-indigo-200' : 'bg-neutral-800/80 border-neutral-700 text-neutral-400 hover:text-neutral-200'}"
            >
              <span class="w-2 h-2 rounded-full flex-none" style="background-color: {u.color}"></span>
              <span>{u.name}</span>
              {#if isChecked}
                <span class="text-[10px] text-indigo-400">✓</span>
              {/if}
            </button>
          {/each}
        </div>
        <p class="text-[10px] text-neutral-500 mt-1">Select household members participating in this goal (optional, all if empty).</p>
      </div>

      <div class="space-y-2 pt-1">
        <div class="flex items-center gap-2">
          <input
            id="project-joint"
            type="checkbox"
            bind:checked={newIsJoint}
            class="w-4 h-4 rounded border-neutral-700 bg-neutral-800 text-indigo-600 focus:ring-indigo-500 cursor-pointer"
          />
          <label for="project-joint" class="text-xs text-neutral-300 font-medium select-none cursor-pointer">
            🏦 Joint Project (Paid by Joint Account)
          </label>
        </div>

        <div class="flex items-center gap-2">
          <input
            id="project-allow-subcategories"
            type="checkbox"
            bind:checked={newAllowSubcategories}
            class="w-4 h-4 rounded border-neutral-700 bg-neutral-800 text-indigo-600 focus:ring-indigo-500 cursor-pointer"
          />
          <label for="project-allow-subcategories" class="text-xs text-neutral-300 font-medium select-none cursor-pointer">
            📁 Allow subcategories for expenses
          </label>
        </div>
      </div>

      {#if addError}
        <p class="text-red-400 text-xs bg-red-950/40 border border-red-800 rounded-xl px-3 py-2">{addError}</p>
      {/if}
      {#if addSuccess}
        <p class="text-emerald-400 text-xs bg-emerald-950/40 border border-emerald-800 rounded-xl px-3 py-2">
          ✓ Project created successfully.
        </p>
      {/if}

      <button
        id="submit-project"
        type="submit"
        disabled={addSubmitting}
        class="btn-primary w-full"
      >
        {addSubmitting ? 'Creating…' : 'Create Project'}
      </button>
    </form>
  </div>

  <!-- Project Cards -->
  <div class="xl:col-span-3 space-y-4">
    <!-- Quick Integration Preference -->
    <div class="card p-4 flex items-center justify-between gap-4">
      <div>
        <p class="text-xs font-semibold text-neutral-200">Show Project Dropdown in Expense Form</p>
        <p class="text-[11px] text-neutral-500 mt-0.5">Toggle whether the project selector is shown when logging new expenses.</p>
      </div>
      <button
        id="toggle-projects-in-expense"
        role="switch"
        aria-checked={$showProjectsInExpense}
        on:click={() => showProjectsInExpense.update((v) => !v)}
        class="relative inline-flex h-6 w-11 flex-none cursor-pointer rounded-full border-2 border-transparent
               transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:ring-offset-neutral-900
               {$showProjectsInExpense ? 'bg-indigo-600' : 'bg-neutral-700'}"
      >
        <span
          class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow
                 transition duration-200 ease-in-out
                 {$showProjectsInExpense ? 'translate-x-5' : 'translate-x-0'}"
        ></span>
      </button>
    </div>

    {#if $projects.length === 0}
      <div class="card empty-state-box">
        <div class="w-12 h-12 rounded-2xl bg-neutral-800 flex items-center justify-center text-2xl mb-4">▰</div>
        <p class="text-neutral-300 text-sm font-semibold">No projects yet.</p>
        <p class="text-neutral-500 text-xs mt-1">Use the form to create your first savings goal.</p>
      </div>

    {:else}
      {#each $projects as project (project.id)}
        {@const progress = pct(project.total_spent_cents, project.target_cents)}
        <div
          id="project-card-{project.id}"
          class="card p-5 hover:border-neutral-700/80 transition-all"
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

              <!-- Edit Member Assignment -->
              <div>
                <label class="block text-xs font-medium text-neutral-400 mb-1.5">Assigned Members</label>
                <div class="flex flex-wrap gap-2">
                  {#each activeUsers as u}
                    {@const isChecked = editMembers.includes(u.name)}
                    <button
                      type="button"
                      on:click={() => toggleEditMember(u.name)}
                      class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium border transition-all cursor-pointer {isChecked ? 'bg-indigo-600/20 border-indigo-500 text-indigo-200' : 'bg-neutral-800/80 border-neutral-700 text-neutral-400 hover:text-neutral-200'}"
                    >
                      <span class="w-2 h-2 rounded-full flex-none" style="background-color: {u.color}"></span>
                      <span>{u.name}</span>
                      {#if isChecked}
                        <span class="text-[10px] text-indigo-400">✓</span>
                      {/if}
                    </button>
                  {/each}
                </div>
              </div>

              <div class="space-y-2 pt-1">
                <div class="flex items-center gap-2">
                  <input
                    id="edit-joint-{project.id}"
                    type="checkbox"
                    bind:checked={editIsJoint}
                    class="w-4 h-4 rounded border-neutral-700 bg-neutral-800 text-indigo-600 focus:ring-indigo-500"
                  />
                  <label for="edit-joint-{project.id}" class="text-xs text-neutral-300 font-medium select-none cursor-pointer">
                    🏦 Joint Project
                  </label>
                </div>

                <div class="flex items-center gap-2">
                  <input
                    id="edit-allow-subcategories-{project.id}"
                    type="checkbox"
                    bind:checked={editAllowSubcategories}
                    class="w-4 h-4 rounded border-neutral-700 bg-neutral-800 text-indigo-600 focus:ring-indigo-500"
                  />
                  <label for="edit-allow-subcategories-{project.id}" class="text-xs text-neutral-300 font-medium select-none cursor-pointer">
                    📁 Allow subcategories for expenses
                  </label>
                </div>
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
                <div class="flex items-center gap-2 flex-wrap">
                  <h3 class="text-sm font-semibold text-neutral-100">{project.name}</h3>
                  {#if project.is_joint}
                    <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-semibold bg-indigo-950/80 text-indigo-300 border border-indigo-800/60">
                      🏦 Joint Project
                    </span>
                  {/if}
                  {#if project.user_names && project.user_names.length > 0}
                    <div class="flex items-center gap-1">
                      {#each project.user_names as uName}
                        <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium bg-neutral-800 border border-neutral-700 text-neutral-300">
                          <span class="w-1.5 h-1.5 rounded-full" style="background-color: {userColor(uName)}"></span>
                          <span>{uName}</span>
                        </span>
                      {/each}
                    </div>
                  {/if}
                  {#if project.allow_subcategories === false}
                    <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-semibold bg-neutral-800 text-neutral-400 border border-neutral-700">
                      🚫 No Subcategories
                    </span>
                  {/if}
                </div>
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
