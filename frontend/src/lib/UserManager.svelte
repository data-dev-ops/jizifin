<script>
  /**
   * UserManager.svelte
   *
   * Settings panel for managing household members.
   * Allows adding new users with a name and colour, toggling active/inactive
   * status, and editing colours for existing users.
   *
   * Active users appear in expense / income / recurring "who paid" dropdowns.
   * Deactivated users are hidden from those dropdowns but their historical
   * data is preserved. They are also visible in the Query tab preset.
   */

  import { users } from './stores.js';
  import { createUser, updateUser, deleteUser, fetchUsers } from './api.js';

  // ── Add-user form state ───────────────────────────────────────────────────
  let newName  = '';
  let newColor = '#6366f1';
  let addError  = '';
  let addSaving = false;

  // ── Per-user inline colour-edit state ─────────────────────────────────────
  let editingColor = {};   // { [name]: currentColor } while editing
  let colorSaving  = {};   // { [name]: bool }
  let colorError   = {};   // { [name]: string }

  $: activeUsers   = $users.filter((u) =>  u.is_active);
  $: inactiveUsers = $users.filter((u) => !u.is_active);

  /** First letter of a name, uppercased. */
  function initial(name) {
    return (name ?? '').charAt(0).toUpperCase();
  }

  async function handleAdd() {
    addError = '';
    if (!newName.trim()) { addError = 'Name is required.'; return; }
    addSaving = true;
    try {
      await createUser({ name: newName.trim(), color: newColor, is_active: true });
      // Reload all users (including inactive) so the store is up to date.
      await fetchUsers(true);
      newName  = '';
      newColor = '#6366f1';
    } catch (e) {
      addError = e.message ?? 'Failed to add user.';
    } finally {
      addSaving = false;
    }
  }

  async function toggleActive(user) {
    try {
      await updateUser(user.name, { is_active: !user.is_active });
      await fetchUsers(true);
    } catch (e) {
      // Surface error gracefully — user sees no change
      console.error(e);
    }
  }

  function startEditColor(user) {
    editingColor = { ...editingColor, [user.name]: user.color };
    colorError   = { ...colorError,   [user.name]: '' };
  }

  function cancelEditColor(name) {
    const { [name]: _, ...rest } = editingColor;
    editingColor = rest;
  }

  async function saveColor(name) {
    colorSaving = { ...colorSaving, [name]: true };
    colorError  = { ...colorError,  [name]: '' };
    try {
      await updateUser(name, { color: editingColor[name] });
      await fetchUsers(true);
      cancelEditColor(name);
    } catch (e) {
      colorError = { ...colorError, [name]: e.message ?? 'Save failed.' };
    } finally {
      colorSaving = { ...colorSaving, [name]: false };
    }
  }

  async function handleDelete(name) {
    if (!confirm(`Delete user "${name}" permanently? This only works if they have no expense or income history.`)) return;
    try {
      await deleteUser(name);
      await fetchUsers(true);
    } catch (e) {
      alert(e.message);
    }
  }
</script>

<div class="space-y-8">

  <!-- ── Active users ─────────────────────────────────────────────────────── -->
  <section>
    <h2 class="text-sm font-semibold text-neutral-300 mb-4">Active Members</h2>

    {#if activeUsers.length === 0}
      <p class="text-neutral-500 text-sm">No active users. Add one below.</p>
    {:else}
      <div class="space-y-2">
        {#each activeUsers as user (user.name)}
          <div class="flex items-center gap-3 bg-neutral-800/50 border border-neutral-700/60 rounded-xl px-4 py-3">

            <!-- Avatar -->
            <div
              class="w-9 h-9 rounded-full flex items-center justify-center text-sm font-bold text-white flex-none shadow-sm"
              style="background-color: {user.color}"
            >
              {initial(user.name)}
            </div>

            <!-- Name -->
            <span class="flex-1 text-sm font-semibold text-neutral-100">{user.name}</span>

            <!-- Colour swatch / editor -->
            {#if editingColor[user.name] !== undefined}
              <div class="flex items-center gap-2">
                <input
                  type="color"
                  bind:value={editingColor[user.name]}
                  class="w-8 h-8 rounded cursor-pointer border-0 bg-transparent p-0"
                  title="Pick colour"
                />
                <button
                  on:click={() => saveColor(user.name)}
                  disabled={colorSaving[user.name]}
                  class="text-xs px-2.5 py-1 rounded-lg bg-emerald-700 hover:bg-emerald-600 text-white font-semibold disabled:opacity-40 transition-colors"
                >
                  {colorSaving[user.name] ? '…' : 'Save'}
                </button>
                <button
                  on:click={() => cancelEditColor(user.name)}
                  class="text-xs px-2.5 py-1 rounded-lg bg-neutral-700 hover:bg-neutral-600 text-neutral-300 transition-colors"
                >
                  Cancel
                </button>
                {#if colorError[user.name]}
                  <span class="text-xs text-red-400">{colorError[user.name]}</span>
                {/if}
              </div>
            {:else}
              <button
                on:click={() => startEditColor(user)}
                title="Edit colour"
                class="w-7 h-7 rounded-full border-2 border-neutral-600 hover:border-neutral-400 transition-colors flex-none cursor-pointer"
                style="background-color: {user.color}"
              ></button>
            {/if}

            <!-- Deactivate button -->
            <button
              on:click={() => toggleActive(user)}
              title="Deactivate user"
              class="text-xs px-3 py-1.5 rounded-lg bg-neutral-700 hover:bg-amber-800/60
                     text-neutral-400 hover:text-amber-300 border border-neutral-600
                     hover:border-amber-700/60 transition-all font-medium"
            >
              Deactivate
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <!-- ── Add new user ──────────────────────────────────────────────────────── -->
  <section>
    <h2 class="text-sm font-semibold text-neutral-300 mb-4">Add New Member</h2>

    <div class="bg-neutral-800/40 border border-neutral-700/60 rounded-xl p-4 space-y-3">
      {#if addError}
        <p class="text-xs text-red-400 bg-red-950/40 border border-red-900 rounded-lg px-3 py-2">{addError}</p>
      {/if}

      <div class="flex flex-wrap gap-3 items-end">
        <!-- Name input -->
        <div class="flex-1 min-w-[160px]">
          <label for="new-user-name" class="block text-xs font-medium text-neutral-500 mb-1.5">Name</label>
          <input
            id="new-user-name"
            type="text"
            maxlength="64"
            placeholder="e.g. Alex"
            bind:value={newName}
            on:keydown={(e) => e.key === 'Enter' && handleAdd()}
            class="w-full bg-neutral-900 border border-neutral-700 rounded-lg px-3 py-2.5 text-sm
                   text-neutral-100 placeholder-neutral-600
                   focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 transition-colors"
          />
        </div>

        <!-- Colour picker -->
        <div>
          <label for="new-user-color" class="block text-xs font-medium text-neutral-500 mb-1.5">Colour</label>
          <div class="flex items-center gap-2">
            <input
              id="new-user-color"
              type="color"
              bind:value={newColor}
              class="w-10 h-10 rounded-lg cursor-pointer border border-neutral-700 bg-neutral-900 p-1"
              title="Pick avatar colour"
            />
            <!-- Live preview -->
            <div
              class="w-9 h-9 rounded-full flex items-center justify-center text-sm font-bold text-white shadow"
              style="background-color: {newColor}"
            >
              {initial(newName) || '?'}
            </div>
          </div>
        </div>

        <!-- Submit -->
        <button
          id="add-user-btn"
          on:click={handleAdd}
          disabled={addSaving}
          class="px-4 py-2.5 rounded-lg text-sm font-semibold
                 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 disabled:cursor-not-allowed
                 transition-colors shadow-sm"
        >
          {addSaving ? 'Adding…' : '+ Add Member'}
        </button>
      </div>
    </div>
  </section>

  <!-- ── Deactivated users ────────────────────────────────────────────────── -->
  {#if inactiveUsers.length > 0}
    <section>
      <h2 class="text-sm font-semibold text-neutral-500 mb-4">Deactivated Members</h2>
      <p class="text-xs text-neutral-600 mb-3">
        Deactivated members are hidden from expense/income entry forms but their
        historical data is fully preserved.
      </p>

      <div class="space-y-2">
        {#each inactiveUsers as user (user.name)}
          <div class="flex items-center gap-3 bg-neutral-900/60 border border-neutral-800 rounded-xl px-4 py-3 opacity-60">
            <div
              class="w-9 h-9 rounded-full flex items-center justify-center text-sm font-bold text-white/60 flex-none grayscale"
              style="background-color: {user.color}"
            >
              {initial(user.name)}
            </div>
            <span class="flex-1 text-sm font-medium text-neutral-500">{user.name}</span>
            <span class="text-[11px] text-neutral-600 px-2 py-0.5 rounded-full border border-neutral-700">inactive</span>

            <!-- Reactivate button -->
            <button
              on:click={() => toggleActive(user)}
              class="text-xs px-3 py-1.5 rounded-lg bg-neutral-800 hover:bg-emerald-900/50
                     text-neutral-400 hover:text-emerald-300 border border-neutral-700
                     hover:border-emerald-700/60 transition-all font-medium"
            >
              Reactivate
            </button>

            <!-- Hard delete (only shown for users with no history) -->
            <button
              on:click={() => handleDelete(user.name)}
              title="Permanently delete (only works if no history)"
              class="text-xs px-2 py-1.5 rounded-lg text-neutral-600 hover:text-red-400
                     hover:bg-red-950/30 transition-colors"
            >
              ✕
            </button>
          </div>
        {/each}
      </div>
    </section>
  {/if}

</div>
