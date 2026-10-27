<script>
  import { onMount } from 'svelte';
  import { auth, backup, categoryApi } from './api.js';

  /** Categories created on first boot: [kind, name, pay_mode]. */
  const SEED_CATEGORIES = [
    ['expense', 'GROCERIES', 'shared'], ['expense', 'UTILITIES', 'shared'], ['expense', 'RENT', 'shared'],
    ['expense', 'OTHER', 'shared'], ['expense', 'FIXED COSTS', 'shared'], ['expense', 'DATING', 'shared'],
    ['expense', 'PET', 'shared'], ['expense', 'LEISURE', 'personal'], ['expense', 'GIFT', 'personal'],
    ['expense', 'PERSONAL COST', 'personal'],
    ['income', 'SALARY', 'shared'], ['income', 'BONUS', 'shared'], ['income', 'GIFT', 'shared'],
  ];

  let passphrase = '';
  let showPassword = false;
  let error = '';
  let loading = false;
  let isFirstBoot = false;
  let backupFile = null;

  onMount(async () => {
    try {
      isFirstBoot = !(await auth.status()).initialized;
    } catch (err) {
      error = `Backend unreachable: ${err.message}`;
    }
  });

  async function seedCategories() {
    for (const [kind, name, pay_mode] of SEED_CATEGORIES) {
      await categoryApi.create({ kind, name, pay_mode });
    }
  }

  async function firstBoot() {
    await auth.init(passphrase);
    if (backupFile?.[0]) {
      await backup.restore(backupFile[0]);
      await auth.login(passphrase);
    } else {
      await seedCategories();
    }
  }

  async function handleSubmit() {
    if (!passphrase.trim()) {
      error = 'Master password is required';
      return;
    }
    loading = true;
    error = '';
    try {
      await (isFirstBoot ? firstBoot() : auth.login(passphrase));
      passphrase = '';
    } catch (err) {
      error = err.status === 401 ? 'Incorrect master password' : err.message;
    } finally {
      loading = false;
    }
  }
</script>

<div class="min-h-screen flex items-center justify-center bg-slate-50 dark:bg-[#080c14] p-4 transition-colors duration-200">
  <div class="w-full max-w-md card p-6 sm:p-8 shadow-2xl space-y-6">
    <div class="text-center space-y-2">
      <div class="w-12 h-12 rounded-2xl bg-gradient-to-br from-indigo-500 to-violet-600 flex items-center justify-center text-xl font-bold shadow-lg shadow-indigo-600/30 text-white mx-auto mb-3">
        🔐
      </div>
      <h1 class="text-2xl sm:text-3xl font-bold tracking-tight text-neutral-900 dark:text-white">
        Jizifin Finance
      </h1>
      <p class="text-xs sm:text-sm text-neutral-500 dark:text-neutral-400">
        {#if isFirstBoot}
          Welcome! Create a master passphrase to encrypt your household database.
        {:else}
          Enter your master passphrase to decrypt your household database.
        {/if}
      </p>
    </div>

    <form on:submit|preventDefault={handleSubmit} class="space-y-4">
      <div>
        <label for="salt" class="block text-xs font-semibold uppercase tracking-wider text-neutral-700 dark:text-neutral-300 mb-2">
          Master Password
        </label>
        <div class="relative">
          {#if showPassword}
            <input
              id="salt"
              name="passphrase"
              type="text"
              bind:value={passphrase}
              placeholder={isFirstBoot ? "Create master passphrase..." : "Enter master passphrase..."}
              autocomplete={isFirstBoot ? "new-password" : "current-password"}
              class="input-field py-3 text-sm pr-11 font-mono"
              disabled={loading}
            />
          {:else}
            <input
              id="salt"
              name="passphrase"
              type="password"
              bind:value={passphrase}
              placeholder={isFirstBoot ? "Create master passphrase..." : "Enter master passphrase..."}
              autocomplete={isFirstBoot ? "new-password" : "current-password"}
              class="input-field py-3 text-sm pr-11"
              disabled={loading}
            />
          {/if}
          <button
            id="toggle-password-btn"
            type="button"
            on:click={() => (showPassword = !showPassword)}
            class="absolute right-3 top-1/2 -translate-y-1/2 text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200 transition-colors p-1 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500 cursor-pointer"
            aria-label={showPassword ? "Hide password" : "Show password"}
            title={showPassword ? "Hide password" : "Show password"}
          >
            {#if showPassword}
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l18 18" />
              </svg>
            {:else}
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
              </svg>
            {/if}
          </button>
        </div>
      </div>

      {#if isFirstBoot}
        <div>
          <label for="dbUpload" class="block text-xs font-semibold uppercase tracking-wider text-neutral-700 dark:text-neutral-300 mb-2">
            Restore Backup (Optional)
          </label>
          <input
            id="dbUpload"
            type="file"
            accept=".duckdb"
            bind:files={backupFile}
            class="w-full text-xs text-neutral-600 dark:text-neutral-400 file:mr-4 file:py-2 file:px-4 file:rounded-xl file:border-0 file:text-xs file:font-semibold file:bg-indigo-100 dark:file:bg-indigo-900/50 file:text-indigo-700 dark:file:text-indigo-300 hover:file:opacity-90 transition cursor-pointer"
            disabled={loading}
          />
          <p class="text-[10px] text-neutral-500 mt-1.5">A Jizifin .duckdb backup. It must have been made with this same passphrase.</p>
        </div>
      {/if}

      {#if error}
        <div class="text-rose-700 dark:text-rose-300 text-xs bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-800/60 rounded-xl p-3">
          {error}
        </div>
      {/if}

      <div class="flex flex-col gap-3 pt-2">
        <button
          type="submit"
          class="btn-primary w-full py-3 text-sm"
          disabled={loading}
        >
          {#if loading}
            <svg class="animate-spin -ml-1 mr-3 h-5 w-5 text-white" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
            </svg>
            {#if isFirstBoot}Initializing...{:else}Decrypting...{/if}
          {:else}
            {#if isFirstBoot}Set Password & Enter{:else}Decrypt & Open{/if}
          {/if}
        </button>
      </div>
    </form>
  </div>
</div>

