<script>
  /**
   * ProjectsTab.svelte
   *
   * Full Projects management tab with complete Light/Dark mode theming:
   *  - Create new project with configurable funding sources (Personal/Household vs Joint Accounts)
   *  - Multi-joint account support with auto-member matching and subgroup presets
   *  - Rich project cards with progress metrics and equity balance sheets
   *  - Inline edit mode and delete with confirmation
   */

  import { projects, users, currencySymbol, showProjectsInExpense, jointAccounts, projectDisplayFilter } from './stores.js';
  import { createProject, updateProject, deleteProject, fetchProjectSettlement } from './api.js';

  $: filteredProjects = $projects.filter((p) => {
    const isCompleted = p.target_cents > 0 && (p.total_spent_cents || 0) >= p.target_cents;
    const hasStarted = (p.total_spent_cents || 0) > 0;
    if ($projectDisplayFilter === 'active') {
      return !isCompleted;
    }
    if ($projectDisplayFilter === 'in_progress') {
      return hasStarted && !isCompleted;
    }
    return true; // 'all'
  });

  // ── helpers ────────────────────────────────────────────────────────────────

  /** Integer cents → formatted amount */
  function fmtEur(cents) {
    return `${$currencySymbol}${((cents || 0) / 100).toLocaleString('en-GB', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
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

  // Quick date helper presets
  function setTargetDateOffset(months) {
    const d = new Date();
    d.setMonth(d.getMonth() + months);
    newTargetDate = d.toISOString().slice(0, 10);
  }

  function setTargetDateEndOfYear() {
    const y = new Date().getFullYear();
    newTargetDate = `${y}-12-31`;
  }

  function addTargetAmount(eur) {
    const cur = parseFloat(newTargetEur) || 0;
    newTargetEur = (cur + eur).toFixed(2);
  }

  // ── add-project form ───────────────────────────────────────────────────────

  let newName = '';
  let newTargetEur = '';
  let newTargetDate = '';
  let newIsJoint = false;
  let newSelectedJointAccountId = null;
  let newAllowSubcategories = true;
  let newMembers = [];
  let addSubmitting = false;
  let addError = null;
  let addSuccess = false;

  // Reactively track joint account selection
  $: if (newIsJoint && !newSelectedJointAccountId && ($jointAccounts || []).length > 0) {
    newSelectedJointAccountId = $jointAccounts[0].id;
  }

  function selectJointAccountFunding(acc) {
    newIsJoint = true;
    newSelectedJointAccountId = acc.id;
    if (acc.member_names && acc.member_names.length > 0) {
      newMembers = [...acc.member_names];
    }
  }

  function setAllHouseholdMembers() {
    newMembers = [];
  }

  function setSubgroupMembers(memberNames) {
    newMembers = [...memberNames];
  }

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

    if (!newName.trim()) {
      addError = 'Project name required.';
      return;
    }

    const parsed = parseFloat(newTargetEur);
    if (isNaN(parsed) || parsed <= 0) {
      addError = 'Target must be a positive amount.';
      return;
    }
    const targetCents = Math.round(parsed * 100);

    if (!newTargetDate || !/^\d{4}-\d{2}-\d{2}$/.test(newTargetDate)) {
      addError = 'Valid target date required (YYYY-MM-DD).';
      return;
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
      newName = '';
      newTargetEur = '';
      newTargetDate = '';
      newIsJoint = false;
      newSelectedJointAccountId = null;
      newAllowSubcategories = true;
      newMembers = [];
    } catch (err) {
      addError = err.message ?? 'Failed to create project.';
    } finally {
      addSubmitting = false;
    }
  }

  // ── per-card edit state ────────────────────────────────────────────────────

  let editingId = null;
  let editName = '';
  let editTargetEur = '';
  let editTargetDate = '';
  let editIsJoint = false;
  let editSelectedJointAccountId = null;
  let editAllowSubcategories = true;
  let editMembers = [];
  let editSubmitting = false;
  let editError = null;

  function startEdit(p) {
    editingId = p.id;
    editName = p.name;
    editTargetEur = (p.target_cents / 100).toFixed(2);
    editTargetDate = p.target_date;
    editIsJoint = Boolean(p.is_joint);
    editAllowSubcategories = p.allow_subcategories !== false;
    editMembers = p.user_names ? [...p.user_names] : [];
    editSelectedJointAccountId = ($jointAccounts || [])[0]?.id || null;
    editError = null;
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
    if (!editName.trim()) {
      editError = 'Project name required.';
      return;
    }

    const parsed = parseFloat(editTargetEur);
    if (isNaN(parsed) || parsed <= 0) {
      editError = 'Target must be a positive amount.';
      return;
    }
    const targetCents = Math.round(parsed * 100);

    if (!editTargetDate || !/^\d{4}-\d{2}-\d{2}$/.test(editTargetDate)) {
      editError = 'Valid target date required (YYYY-MM-DD).';
      return;
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
  let deletingId = null;
  let deleteError = null;

  async function handleDelete(id) {
    deletingId = id;
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

  // ── Project Settlement Drawer State ────────────────────────────────────────

  let settlementsData = {};
  let settlementsLoading = {};
  let settlementsOpen = {};

  async function toggleSettlement(projId) {
    settlementsOpen[projId] = !settlementsOpen[projId];
    if (settlementsOpen[projId] && !settlementsData[projId]) {
      settlementsLoading[projId] = true;
      try {
        settlementsData[projId] = await fetchProjectSettlement(projId);
      } catch (err) {
        console.error('Failed to load project settlement:', err);
      } finally {
        settlementsLoading[projId] = false;
      }
    }
    settlementsOpen = { ...settlementsOpen };
    settlementsData = { ...settlementsData };
    settlementsLoading = { ...settlementsLoading };
  }

  function barColor(p) {
    if (p >= 100) return 'from-emerald-500 to-emerald-400';
    if (p >= 60) return 'from-indigo-500 to-violet-500';
    if (p >= 30) return 'from-sky-600 to-indigo-500';
    return 'from-sky-700 to-sky-500';
  }
</script>

<div class="grid grid-cols-1 xl:grid-cols-5 gap-6">

  <!-- ── LEFT PANEL: Configurable New Project Form ───────────────────────── -->
  <div class="xl:col-span-2 card space-y-5">
    <div class="border-b border-neutral-200 dark:border-neutral-800 pb-3">
      <h2 class="text-sm font-bold text-neutral-900 dark:text-white flex items-center gap-2">
        <span>🎯</span> New Savings / Project Goal
      </h2>
      <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">
        Define shared or individual milestone budgets with customizable funding rules
      </p>
    </div>

    <form on:submit={handleAdd} id="add-project-form" class="space-y-4">

      <!-- Project Name -->
      <div>
        <label for="project-name" class="block text-xs font-semibold text-neutral-700 dark:text-neutral-300 uppercase tracking-wider mb-1.5">
          Project Name
        </label>
        <input
          id="project-name"
          type="text"
          maxlength="96"
          placeholder="e.g. Bathroom Renovation, Summer Vacation, New Car"
          bind:value={newName}
          class="input-field"
        />
      </div>

      <!-- Target Amount & Quick Add Chips -->
      <div>
        <label for="project-target" class="block text-xs font-semibold text-neutral-700 dark:text-neutral-300 uppercase tracking-wider mb-1.5">
          Target Amount ({$currencySymbol})
        </label>
        <div class="relative">
          <span class="absolute left-3.5 top-1/2 -translate-y-1/2 text-neutral-500 text-sm font-bold">{$currencySymbol}</span>
          <input
            id="project-target"
            type="number"
            min="0.01"
            step="0.01"
            placeholder="0.00"
            bind:value={newTargetEur}
            class="input-field pl-8 font-semibold tabular-nums"
          />
        </div>
        <div class="flex items-center gap-1.5 mt-2 flex-wrap">
          <span class="text-[10px] text-neutral-500 uppercase font-semibold mr-1">Quick Add:</span>
          {#each [100, 500, 1000, 5000] as eur}
            <button
              type="button"
              on:click={() => addTargetAmount(eur)}
              class="px-2 py-0.5 rounded-lg bg-neutral-100 dark:bg-neutral-800 hover:bg-neutral-200 dark:hover:bg-neutral-700 border border-neutral-200 dark:border-neutral-700 text-[11px] text-neutral-700 dark:text-neutral-300 transition-colors cursor-pointer"
            >
              +{$currencySymbol}{eur}
            </button>
          {/each}
        </div>
      </div>

      <!-- Target Completion Date & Presets -->
      <div>
        <label for="project-date" class="block text-xs font-semibold text-neutral-700 dark:text-neutral-300 uppercase tracking-wider mb-1.5">
          Target Completion Date
        </label>
        <input
          id="project-date"
          type="date"
          bind:value={newTargetDate}
          class="input-field"
        />
        <div class="flex items-center gap-1.5 mt-2 flex-wrap">
          <span class="text-[10px] text-neutral-500 uppercase font-semibold mr-1">Presets:</span>
          <button
            type="button"
            on:click={() => setTargetDateOffset(3)}
            class="px-2 py-0.5 rounded-lg bg-neutral-100 dark:bg-neutral-800 hover:bg-neutral-200 dark:hover:bg-neutral-700 border border-neutral-200 dark:border-neutral-700 text-[11px] text-neutral-700 dark:text-neutral-300 transition-colors cursor-pointer"
          >
            +3 Mo
          </button>
          <button
            type="button"
            on:click={() => setTargetDateOffset(6)}
            class="px-2 py-0.5 rounded-lg bg-neutral-100 dark:bg-neutral-800 hover:bg-neutral-200 dark:hover:bg-neutral-700 border border-neutral-200 dark:border-neutral-700 text-[11px] text-neutral-700 dark:text-neutral-300 transition-colors cursor-pointer"
          >
            +6 Mo
          </button>
          <button
            type="button"
            on:click={setTargetDateEndOfYear}
            class="px-2 py-0.5 rounded-lg bg-neutral-100 dark:bg-neutral-800 hover:bg-neutral-200 dark:hover:bg-neutral-700 border border-neutral-200 dark:border-neutral-700 text-[11px] text-neutral-700 dark:text-neutral-300 transition-colors cursor-pointer"
          >
            End of Year
          </button>
          <button
            type="button"
            on:click={() => setTargetDateOffset(12)}
            class="px-2 py-0.5 rounded-lg bg-neutral-100 dark:bg-neutral-800 hover:bg-neutral-200 dark:hover:bg-neutral-700 border border-neutral-200 dark:border-neutral-700 text-[11px] text-neutral-700 dark:text-neutral-300 transition-colors cursor-pointer"
          >
            +1 Year
          </button>
        </div>
      </div>

      <!-- ── Funding Source & Joint Account Configuration ───────────────────── -->
      <div class="card-sub p-3.5 space-y-3">
        <label class="block text-xs font-semibold text-neutral-700 dark:text-neutral-300 uppercase tracking-wider">
          Funding & Payment Source
        </label>

        <!-- Hidden checkbox for programmatic/legacy backward compatibility -->
        <input
          id="project-joint"
          type="checkbox"
          bind:checked={newIsJoint}
          class="sr-only"
        />

        <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
          <!-- Personal / Household Option -->
          <button
            type="button"
            on:click={() => (newIsJoint = false)}
            class="p-2.5 rounded-xl border text-left transition-all cursor-pointer {!newIsJoint ? 'bg-indigo-50 dark:bg-indigo-950/80 border-indigo-500 text-indigo-900 dark:text-white shadow-sm font-semibold' : 'bg-white dark:bg-neutral-900 border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
          >
            <div class="flex items-center gap-1.5 text-xs font-bold">
              <span>👥</span>
              <span>Personal / Household</span>
            </div>
            <p class="text-[10px] text-neutral-500 mt-1">Paid directly by participating members</p>
          </button>

          <!-- Joint Account Option -->
          {#if ($jointAccounts || []).length > 0}
            <button
              type="button"
              on:click={() => {
                newIsJoint = true;
                if (!newSelectedJointAccountId && $jointAccounts.length > 0) {
                  selectJointAccountFunding($jointAccounts[0]);
                }
              }}
              class="p-2.5 rounded-xl border text-left transition-all cursor-pointer {newIsJoint ? 'bg-indigo-50 dark:bg-indigo-950/80 border-indigo-500 text-indigo-900 dark:text-white shadow-sm font-semibold' : 'bg-white dark:bg-neutral-900 border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
            >
              <div class="flex items-center gap-1.5 text-xs font-bold">
                <span>🏦</span>
                <span>Joint Account Funded</span>
              </div>
              <p class="text-[10px] text-neutral-500 mt-1">Paid from shared joint account fund</p>
            </button>
          {:else}
            <div class="p-2.5 rounded-xl border border-neutral-200 dark:border-neutral-800/60 bg-neutral-50 dark:bg-neutral-900/40 text-neutral-400 text-left">
              <div class="flex items-center gap-1.5 text-xs font-semibold">
                <span>🏦</span>
                <span>No Joint Accounts</span>
              </div>
              <p class="text-[10px] mt-1 text-neutral-500">Configure one in Joint Accounts tab</p>
            </div>
          {/if}
        </div>

        <!-- If Joint Account is selected and multiple exist, show account picker -->
        {#if newIsJoint && ($jointAccounts || []).length > 0}
          <div class="pt-2 border-t border-neutral-200 dark:border-neutral-800/80 space-y-2">
            <span class="text-[11px] font-semibold text-neutral-600 dark:text-neutral-400">Select Funding Joint Account:</span>
            <div class="flex flex-wrap gap-1.5">
              {#each $jointAccounts as acc}
                <button
                  type="button"
                  on:click={() => selectJointAccountFunding(acc)}
                  class="px-2.5 py-1 rounded-lg text-xs font-medium border transition-all cursor-pointer {newSelectedJointAccountId === acc.id ? 'bg-indigo-600 border-indigo-400 text-white shadow-sm font-bold' : 'bg-white dark:bg-neutral-900 border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
                >
                  <span>🏦 {acc.name}</span>
                  {#if acc.member_names && acc.member_names.length > 0}
                    <span class="text-[10px] opacity-75">({acc.member_names.join(' & ')})</span>
                  {/if}
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>

      <!-- ── Participating Members Assignment ───────────────────────────────── -->
      <div class="space-y-2">
        <div class="flex items-center justify-between">
          <label class="block text-xs font-semibold text-neutral-700 dark:text-neutral-300 uppercase tracking-wider">
            Assigned Household Members
          </label>
          <span class="text-[11px] text-neutral-500">
            {newMembers.length === 0 ? 'All Household Members' : `${newMembers.length} of ${activeUsers.length} selected`}
          </span>
        </div>

        <!-- Subgroup quick selectors -->
        <div class="flex items-center gap-1.5 flex-wrap">
          <button
            type="button"
            on:click={setAllHouseholdMembers}
            class="px-2 py-0.5 rounded-lg text-[10px] font-medium border transition-colors cursor-pointer {newMembers.length === 0 ? 'bg-indigo-50 dark:bg-indigo-950 border-indigo-500 text-indigo-700 dark:text-indigo-300 font-semibold' : 'bg-neutral-100 dark:bg-neutral-800 border-neutral-200 dark:border-neutral-700 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'}"
          >
            All Members ({activeUsers.length})
          </button>
          {#each $jointAccounts || [] as acc}
            {#if acc.member_names && acc.member_names.length > 0}
              <button
                type="button"
                on:click={() => setSubgroupMembers(acc.member_names)}
                class="px-2 py-0.5 rounded-lg text-[10px] font-medium border transition-colors cursor-pointer bg-neutral-100 dark:bg-neutral-800 border-neutral-200 dark:border-neutral-700 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200"
              >
                🏦 {acc.name} ({acc.member_names.join('+')})
              </button>
            {/if}
          {/each}
        </div>

        <!-- Individual member toggle chips -->
        <div class="flex flex-wrap gap-2 pt-1">
          {#each activeUsers as u}
            {@const isChecked = newMembers.includes(u.name)}
            <button
              type="button"
              on:click={() => toggleNewMember(u.name)}
              class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-semibold border transition-all cursor-pointer {isChecked ? 'bg-indigo-50 dark:bg-indigo-950 border-indigo-500 text-indigo-900 dark:text-white shadow-sm font-bold' : 'bg-white dark:bg-neutral-900 border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-500 hover:text-neutral-900 dark:hover:text-neutral-300'}"
            >
              <span class="w-2.5 h-2.5 rounded-full flex-none" style="background-color: {u.color}"></span>
              <span>{u.name}</span>
              {#if isChecked}
                <span class="text-[10px] text-indigo-600 dark:text-indigo-400 font-bold">✓</span>
              {/if}
            </button>
          {/each}
        </div>
        <p class="text-[10px] text-neutral-500 mt-1">
          Assigned members share liability & equity in project settlement balance sheets.
        </p>
      </div>

      <!-- ── Subcategories Option Toggle ────────────────────────────────────── -->
      <div class="card-sub p-3 flex items-center justify-between gap-3">
        <div>
          <label for="project-allow-subcategories" class="text-xs text-neutral-800 dark:text-neutral-200 font-semibold cursor-pointer select-none">
            📁 Allow Expense Subcategories
          </label>
          <p class="text-[10px] text-neutral-500 mt-0.5">
            When enabled, project expenses can also be tagged under specific split categories (e.g. Materials, Travel).
          </p>
        </div>
        <input
          id="project-allow-subcategories"
          type="checkbox"
          bind:checked={newAllowSubcategories}
          class="w-4 h-4 rounded border-neutral-300 dark:border-neutral-700 bg-white dark:bg-neutral-800 text-indigo-600 focus:ring-indigo-500 cursor-pointer shrink-0"
        />
      </div>

      {#if addError}
        <p class="text-rose-700 dark:text-red-400 text-xs bg-rose-50 dark:bg-red-950/40 border border-rose-200 dark:border-red-800 rounded-xl px-3 py-2">{addError}</p>
      {/if}
      {#if addSuccess}
        <p class="text-emerald-700 dark:text-emerald-400 text-xs bg-emerald-50 dark:bg-emerald-950/40 border border-emerald-200 dark:border-emerald-800 rounded-xl px-3 py-2">
          ✓ Project created successfully.
        </p>
      {/if}

      <button
        id="submit-project"
        type="submit"
        disabled={addSubmitting}
        class="btn-primary w-full py-2.5"
      >
        {addSubmitting ? 'Creating…' : 'Create Project'}
      </button>
    </form>
  </div>

  <!-- ── RIGHT PANEL: Project Cards List & Settlement Balance Sheets ─────── -->
  <div class="xl:col-span-3 space-y-4">

    <!-- Quick Integration Toggle -->
    <div class="card p-4 flex items-center justify-between gap-4">
      <div>
        <p class="text-xs font-semibold text-neutral-800 dark:text-neutral-200">Show Project Dropdown in Expense Form</p>
        <p class="text-[11px] text-neutral-500 mt-0.5">Toggle whether the project selector is shown when logging new expenses.</p>
      </div>
      <button
        id="toggle-projects-in-expense"
        role="switch"
        aria-checked={$showProjectsInExpense}
        on:click={() => showProjectsInExpense.update((v) => !v)}
        class="relative inline-flex h-6 w-11 flex-none cursor-pointer rounded-full border-2 border-transparent
               transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2
               {$showProjectsInExpense ? 'bg-indigo-600' : 'bg-neutral-300 dark:bg-neutral-700'}"
      >
        <span
          class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow
                 transition duration-200 ease-in-out
                 {$showProjectsInExpense ? 'translate-x-5' : 'translate-x-0'}"
        ></span>
      </button>
    </div>

    <!-- ── Project Lifecycle Filters ── -->
    {#if $projects.length > 0}
      <div class="flex items-center gap-1.5 p-1 bg-neutral-100 dark:bg-neutral-900/80 rounded-xl border border-neutral-200 dark:border-neutral-800 text-xs self-start">
        <button
          id="filter-proj-active"
          type="button"
          on:click={() => projectDisplayFilter.set('active')}
          class="px-3 py-1.5 rounded-lg font-medium transition-all {$projectDisplayFilter === 'active' ? 'bg-white dark:bg-neutral-800 text-indigo-600 dark:text-indigo-400 shadow-xs font-semibold' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-white'}"
        >
          Active ({$projects.filter(p => (p.total_spent_cents || 0) < p.target_cents).length})
        </button>
        <button
          id="filter-proj-in-progress"
          type="button"
          on:click={() => projectDisplayFilter.set('in_progress')}
          class="px-3 py-1.5 rounded-lg font-medium transition-all {$projectDisplayFilter === 'in_progress' ? 'bg-white dark:bg-neutral-800 text-indigo-600 dark:text-indigo-400 shadow-xs font-semibold' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-white'}"
        >
          In Progress ({$projects.filter(p => (p.total_spent_cents || 0) > 0 && (p.total_spent_cents || 0) < p.target_cents).length})
        </button>
        <button
          id="filter-proj-all"
          type="button"
          on:click={() => projectDisplayFilter.set('all')}
          class="px-3 py-1.5 rounded-lg font-medium transition-all {$projectDisplayFilter === 'all' ? 'bg-white dark:bg-neutral-800 text-indigo-600 dark:text-indigo-400 shadow-xs font-semibold' : 'text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-white'}"
        >
          All ({$projects.length})
        </button>
      </div>
    {/if}

    {#if $projects.length === 0}
      <div class="card empty-state-box">
        <div class="w-12 h-12 rounded-2xl bg-neutral-100 dark:bg-neutral-800 flex items-center justify-center text-2xl mb-4">🎯</div>
        <p class="text-neutral-700 dark:text-neutral-300 text-sm font-semibold">No projects yet.</p>
        <p class="text-neutral-500 text-xs mt-1">Use the form on the left to create your first savings goal or project.</p>
      </div>

    {:else if filteredProjects.length === 0}
      <div class="card p-6 text-center rounded-2xl border border-neutral-200 dark:border-neutral-800 bg-neutral-50/50 dark:bg-neutral-950/40">
        <p class="text-sm font-medium text-neutral-700 dark:text-neutral-300">No projects matching filter "{$projectDisplayFilter}".</p>
        <button
          type="button"
          on:click={() => projectDisplayFilter.set('all')}
          class="mt-2 text-xs text-indigo-600 dark:text-indigo-400 hover:underline font-semibold"
        >
          Show all {$projects.length} projects →
        </button>
      </div>

    {:else}
      {#each filteredProjects as project (project.id)}
        {@const progress = pct(project.total_spent_cents, project.target_cents)}
        <div
          id="project-card-{project.id}"
          class="card p-5 hover:border-neutral-300 dark:hover:border-neutral-700/80 transition-all space-y-4"
        >
          {#if editingId === project.id}
            <!-- ── Inline Edit Mode ── -->
            <form on:submit={(e) => handleEdit(e, project.id)} class="space-y-4">
              <h4 class="text-xs font-bold text-neutral-800 dark:text-neutral-300 uppercase tracking-wider">Edit Project Details</h4>

              <div>
                <label for="edit-name-{project.id}" class="text-[11px] text-neutral-600 dark:text-neutral-400 block mb-1">Project Name</label>
                <input
                  id="edit-name-{project.id}"
                  type="text"
                  maxlength="96"
                  bind:value={editName}
                  class="input-field"
                />
              </div>

              <div class="grid grid-cols-2 gap-3">
                <div>
                  <label for="edit-target-{project.id}" class="text-[11px] text-neutral-600 dark:text-neutral-400 block mb-1">Target Amount ({$currencySymbol})</label>
                  <div class="relative">
                    <span class="absolute left-3 top-1/2 -translate-y-1/2 text-neutral-500 text-xs font-bold">{$currencySymbol}</span>
                    <input
                      id="edit-target-{project.id}"
                      type="number"
                      min="0.01"
                      step="0.01"
                      bind:value={editTargetEur}
                      class="input-field pl-7 font-semibold"
                    />
                  </div>
                </div>
                <div>
                  <label for="edit-date-{project.id}" class="text-[11px] text-neutral-600 dark:text-neutral-400 block mb-1">Target Date</label>
                  <input
                    id="edit-date-{project.id}"
                    type="date"
                    bind:value={editTargetDate}
                    class="input-field"
                  />
                </div>
              </div>

              <!-- Edit Member Assignment -->
              <div>
                <label class="block text-[11px] text-neutral-600 dark:text-neutral-400 mb-1.5 font-medium">Assigned Members</label>
                <div class="flex flex-wrap gap-2">
                  {#each activeUsers as u}
                    {@const isChecked = editMembers.includes(u.name)}
                    <button
                      type="button"
                      on:click={() => toggleEditMember(u.name)}
                      class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium border transition-all cursor-pointer {isChecked ? 'bg-indigo-50 dark:bg-indigo-950 border-indigo-500 text-indigo-900 dark:text-white font-bold' : 'bg-white dark:bg-neutral-900 border-neutral-200 dark:border-neutral-800 text-neutral-600 dark:text-neutral-500'}"
                    >
                      <span class="w-2 h-2 rounded-full flex-none" style="background-color: {u.color}"></span>
                      <span>{u.name}</span>
                      {#if isChecked}
                        <span class="text-[10px] text-indigo-600 dark:text-indigo-400 font-bold">✓</span>
                      {/if}
                    </button>
                  {/each}
                </div>
              </div>

              <!-- Funding Source Toggle in Edit Mode -->
              <div class="card-sub p-3 space-y-2">
                <input
                  id="edit-joint-{project.id}"
                  type="checkbox"
                  bind:checked={editIsJoint}
                  class="sr-only"
                />
                <div class="flex items-center gap-2">
                  <label for="edit-joint-{project.id}" class="flex items-center gap-2 text-xs text-neutral-700 dark:text-neutral-300 font-medium select-none cursor-pointer">
                    <input
                      type="checkbox"
                      bind:checked={editIsJoint}
                      class="w-4 h-4 rounded border-neutral-300 dark:border-neutral-700 bg-white dark:bg-neutral-800 text-indigo-600 focus:ring-indigo-500 cursor-pointer"
                    />
                    <span>🏦 Joint Account Project</span>
                  </label>
                </div>

                <div class="flex items-center gap-2 pt-1 border-t border-neutral-200 dark:border-neutral-800">
                  <label for="edit-allow-subcategories-{project.id}" class="flex items-center gap-2 text-xs text-neutral-700 dark:text-neutral-300 font-medium select-none cursor-pointer">
                    <input
                      id="edit-allow-subcategories-{project.id}"
                      type="checkbox"
                      bind:checked={editAllowSubcategories}
                      class="w-4 h-4 rounded border-neutral-300 dark:border-neutral-700 bg-white dark:bg-neutral-800 text-indigo-600 focus:ring-indigo-500 cursor-pointer"
                    />
                    <span>📁 Allow subcategories for expenses</span>
                  </label>
                </div>
              </div>

              {#if editError}
                <p class="text-rose-600 dark:text-red-400 text-xs">{editError}</p>
              {/if}
              <div class="flex gap-2">
                <button
                  id="save-edit-{project.id}"
                  type="submit"
                  disabled={editSubmitting}
                  class="btn-primary flex-1 py-2 text-xs"
                >
                  {editSubmitting ? 'Saving…' : 'Save Changes'}
                </button>
                <button
                  id="cancel-edit-{project.id}"
                  type="button"
                  on:click={cancelEdit}
                  class="btn-secondary flex-1 py-2 text-xs"
                >
                  Cancel
                </button>
              </div>
            </form>

          {:else}
            <!-- ── View Mode ── -->
            <div class="flex items-start justify-between gap-2">
              <div>
                <div class="flex items-center gap-2 flex-wrap">
                  <h3 class="text-base font-bold text-neutral-900 dark:text-white">{project.name}</h3>

                  {#if project.is_joint}
                    <span class="badge-indigo">
                      🏦 Joint Project
                    </span>
                  {:else}
                    <span class="badge-neutral">
                      👥 Shared Household
                    </span>
                  {/if}

                  {#if project.user_names && project.user_names.length > 0}
                    <div class="flex items-center gap-1">
                      {#each project.user_names as uName}
                        <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-lg text-xs font-medium bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 text-neutral-700 dark:text-neutral-300">
                          <span class="w-2 h-2 rounded-full" style="background-color: {userColor(uName)}"></span>
                          <span>{uName}</span>
                        </span>
                      {/each}
                    </div>
                  {/if}

                  {#if project.allow_subcategories === false}
                    <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-lg text-[11px] font-medium bg-neutral-100 dark:bg-neutral-900 text-neutral-500 border border-neutral-200 dark:border-neutral-800">
                      🚫 Direct Only
                    </span>
                  {/if}
                </div>
                <p class="text-xs text-neutral-500 mt-1">Target Completion: <strong class="text-neutral-800 dark:text-neutral-300">{fmtDate(project.target_date)}</strong></p>
              </div>

              <!-- Action buttons -->
              <div class="flex items-center gap-1.5 flex-none">
                <button
                  id="edit-project-{project.id}"
                  type="button"
                  on:click={() => startEdit(project)}
                  title="Edit project"
                  class="p-1.5 rounded-lg text-neutral-400 hover:text-indigo-600 dark:hover:text-indigo-300 hover:bg-indigo-50 dark:hover:bg-indigo-950/40 transition-all duration-150 cursor-pointer"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4">
                    <path d="M5.433 13.917l1.262-3.155A4 4 0 0 1 7.58 9.42l6.92-6.918a2.121 2.121 0 0 1 3 3l-6.92 6.918c-.383.383-.84.685-1.343.886l-3.154 1.262a.5.5 0 0 1-.65-.65Z" />
                    <path d="M3.5 5.75c0-.69.56-1.25 1.25-1.25H10A.75.75 0 0 0 10 3H4.75A2.75 2.75 0 0 0 2 5.75v9.5A2.75 2.75 0 0 0 4.75 18h9.5A2.75 2.75 0 0 0 17 15.25V10a.75.75 0 0 0-1.5 0v5.25c0 .69-.56 1.25-1.25 1.25h-9.5c-.69 0-1.25-.56-1.25-1.25v-9.5Z" />
                  </svg>
                </button>

                {#if confirmDeleteId === project.id}
                  <span class="flex items-center gap-1.5">
                    <span class="text-xs text-neutral-500 dark:text-neutral-400">Delete?</span>
                    <button
                      id="confirm-delete-project-{project.id}"
                      type="button"
                      on:click={() => handleDelete(project.id)}
                      disabled={deletingId === project.id}
                      class="px-2.5 py-1 rounded-lg text-xs font-semibold bg-rose-600 hover:bg-rose-500 text-white disabled:opacity-40 transition-colors cursor-pointer"
                    >
                      {deletingId === project.id ? '…' : 'Yes'}
                    </button>
                    <button
                      id="cancel-delete-project-{project.id}"
                      type="button"
                      on:click={() => { confirmDeleteId = null; deleteError = null; }}
                      class="px-2.5 py-1 rounded-lg text-xs font-semibold bg-neutral-200 dark:bg-neutral-700 hover:bg-neutral-300 dark:hover:bg-neutral-600 text-neutral-800 dark:text-neutral-200 transition-colors cursor-pointer"
                    >
                      No
                    </button>
                  </span>
                {:else}
                  <button
                    id="delete-project-{project.id}"
                    type="button"
                    on:click={() => { confirmDeleteId = project.id; deleteError = null; }}
                    title="Delete project"
                    class="p-1.5 rounded-lg text-neutral-400 hover:text-rose-600 dark:hover:text-red-400 hover:bg-rose-50 dark:hover:bg-red-950/40 transition-all duration-150 cursor-pointer"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4">
                      <path fill-rule="evenodd" d="M8.75 1A2.75 2.75 0 0 0 6 3.75v.443c-.795.077-1.584.176-2.365.298a.75.75 0 1 0 .23 1.482l.149-.022.841 10.518A2.75 2.75 0 0 0 7.596 19h4.807a2.75 2.75 0 0 0 2.742-2.53l.841-10.52.149.023a.75.75 0 0 0 .23-1.482A41.03 41.03 0 0 0 14 4.193V3.75A2.75 2.75 0 0 0 11.25 1h-2.5ZM10 4c.84 0 1.673.025 2.5.075V3.75c0-.69-.56-1.25-1.25-1.25h-2.5c-.69 0-1.25.56-1.25 1.25v.325C8.327 4.025 9.16 4 10 4ZM8.58 7.72a.75.75 0 0 0-1.5.06l.3 7.5a.75.75 0 1 0 1.5-.06l-.3-7.5Zm4.34.06a.75.75 0 1 0-1.5-.06l-.3 7.5a.75.75 0 1 0 1.5.06l.3-7.5Z" clip-rule="evenodd" />
                    </svg>
                  </button>
                {/if}
              </div>
            </div>

            <!-- Amount + Visual Progress Bar -->
            <div class="space-y-1.5">
              <div class="flex justify-between text-xs font-semibold">
                <span class="text-neutral-800 dark:text-neutral-300">
                  {fmtEur(project.total_spent_cents)}
                  <span class="text-neutral-500 font-normal">/ {fmtEur(project.target_cents)}</span>
                </span>
                <span class="{progress >= 100 ? 'text-emerald-600 dark:text-emerald-400 font-bold' : 'text-neutral-800 dark:text-neutral-300'} tabular-nums">
                  {progress}%
                </span>
              </div>
              <div class="w-full h-2.5 bg-neutral-200 dark:bg-neutral-950 rounded-full overflow-hidden shadow-inner flex">
                <div
                  class="h-full rounded-full bg-gradient-to-r {barColor(progress)} transition-all duration-700"
                  style="width: {progress}%"
                ></div>
              </div>
            </div>

            <!-- Metrics Grid -->
            <div class="grid grid-cols-3 gap-3">
              <div class="card-sub p-3">
                <p class="text-[10px] text-neutral-500 mb-1 uppercase tracking-wide font-semibold">Last Payment</p>
                {#if project.last_payment}
                  <p class="text-xs font-bold text-neutral-900 dark:text-neutral-200">{fmtEur(project.last_payment.cost_cents)}</p>
                  <p class="text-[10px] text-neutral-500 mt-0.5">{fmtDate(project.last_payment.expense_date)}</p>
                  <p class="text-[10px] text-indigo-600 dark:text-indigo-400 mt-0.5 truncate">{project.last_payment.who_paid}</p>
                {:else}
                  <p class="text-xs text-neutral-400">None yet</p>
                {/if}
              </div>

              <div class="card-sub p-3">
                <p class="text-[10px] text-neutral-500 mb-1 uppercase tracking-wide font-semibold">Avg / Month</p>
                {#if project.avg_monthly_payment_cents > 0}
                  <p class="text-xs font-bold text-neutral-900 dark:text-neutral-200">{fmtEur(project.avg_monthly_payment_cents)}</p>
                  <p class="text-[10px] text-neutral-500 mt-0.5">per month</p>
                {:else}
                  <p class="text-xs text-neutral-400">No data</p>
                {/if}
              </div>

              <div class="card-sub p-3">
                <p class="text-[10px] text-neutral-500 mb-1 uppercase tracking-wide font-semibold">Est. Completion</p>
                <p class="text-xs font-bold
                  {project.estimated_completion_date === 'Completed'
                    ? 'text-emerald-600 dark:text-emerald-400'
                    : project.estimated_completion_date === 'Indefinite'
                    ? 'text-neutral-400'
                    : 'text-indigo-600 dark:text-indigo-300'}">
                  {estLabel(project.estimated_completion_date)}
                </p>
              </div>
            </div>

            <!-- Project Settlement & Equity Breakdown Toggle -->
            <div class="pt-2 border-t border-neutral-200 dark:border-neutral-800/80">
              <button
                type="button"
                id="toggle-settlement-{project.id}"
                on:click={() => toggleSettlement(project.id)}
                class="inline-flex items-center gap-1.5 text-xs font-semibold text-indigo-600 dark:text-indigo-400 hover:underline transition-colors cursor-pointer"
              >
                <span>⚖️</span>
                <span>{settlementsOpen[project.id] ? 'Hide Settlement & Equity Balance Sheet' : 'View Settlement & Equity Balance Sheet'}</span>
                <span class="text-[10px] transform transition-transform duration-200 {settlementsOpen[project.id] ? 'rotate-180' : ''}">▼</span>
              </button>

              {#if settlementsOpen[project.id]}
                <div class="mt-3 p-4 bg-neutral-50 dark:bg-neutral-950/90 border border-indigo-200 dark:border-indigo-900/40 rounded-xl space-y-3 animate-fadeIn">
                  {#if settlementsLoading[project.id]}
                    <p class="text-xs text-neutral-500 py-2">Loading project balance sheet…</p>
                  {:else if settlementsData[project.id]}
                    {@const sData = settlementsData[project.id]}
                    <div>
                      <div class="flex items-center justify-between text-xs mb-2">
                        <span class="font-semibold text-neutral-700 dark:text-neutral-300 uppercase tracking-wider text-[10px]">Participant Equity Positions</span>
                        <span class="font-mono text-neutral-500 dark:text-neutral-400 text-[11px]">Total Spent: {fmtEur(sData.total_spent_cents)}</span>
                      </div>

                      <div class="space-y-2">
                        {#each sData.participants as p}
                          <div class="flex items-center justify-between p-2.5 rounded-xl bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 text-xs">
                            <div class="flex items-center gap-2">
                              <span class="w-2.5 h-2.5 rounded-full flex-none" style="background-color: {userColor(p.user_name)}"></span>
                              <span class="font-semibold text-neutral-900 dark:text-neutral-200">{p.user_name}</span>
                            </div>
                            <div class="flex items-center gap-3 font-mono text-[11px]">
                              <span class="text-neutral-500 dark:text-neutral-400">Paid: {fmtEur(p.effective_funding_cents)}</span>
                              <span class="text-neutral-300 dark:text-neutral-600">|</span>
                              <span class="text-neutral-500 dark:text-neutral-400">Share: {fmtEur(p.assigned_liability_cents)}</span>
                              <span class="px-2 py-0.5 rounded font-bold {p.net_balance_cents > 0 ? 'bg-emerald-50 dark:bg-emerald-950/80 text-emerald-700 dark:text-emerald-300 border border-emerald-200 dark:border-emerald-800/60' : p.net_balance_cents < 0 ? 'bg-rose-50 dark:bg-rose-950/80 text-rose-700 dark:text-rose-300 border border-rose-200 dark:border-rose-800/60' : 'bg-neutral-100 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-400'}">
                                {p.net_balance_cents > 0 ? `+${fmtEur(p.net_balance_cents)}` : p.net_balance_cents < 0 ? `-${fmtEur(Math.abs(p.net_balance_cents))}` : '€0.00'}
                              </span>
                            </div>
                          </div>
                        {/each}
                      </div>

                      {#if sData.debts && sData.debts.length > 0}
                        <div class="mt-3 pt-2.5 border-t border-neutral-200 dark:border-neutral-800">
                          <p class="text-[10px] font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-1.5">Project Settlement Reimbursements</p>
                          <div class="space-y-1.5">
                            {#each sData.debts as d}
                              <div class="flex items-center justify-between text-xs py-1.5 px-3 rounded-lg bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800/80 font-mono">
                                <div class="flex items-center gap-1.5 font-sans text-neutral-800 dark:text-neutral-300">
                                  <span class="font-semibold" style="color: {userColor(d.from_user)}">{d.from_user}</span>
                                  <span class="text-neutral-400 dark:text-neutral-500">→ pays →</span>
                                  <span class="font-semibold" style="color: {userColor(d.to_user)}">{d.to_user}</span>
                                </div>
                                <span class="font-bold text-amber-600 dark:text-amber-300">{fmtEur(Math.round(d.amount * 100))}</span>
                              </div>
                            {/each}
                          </div>
                        </div>
                      {/if}
                    </div>
                  {/if}
                </div>
              {/if}
            </div>

            {#if deleteError && confirmDeleteId === project.id}
              <p class="text-xs text-rose-600 dark:text-red-400 mt-2">{deleteError}</p>
            {/if}
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</div>
