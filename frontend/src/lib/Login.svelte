<script>
  import { authSalt, cryptoKey, sessionToken } from './stores.js';
  import { deriveKey, encryptText, decryptText } from './crypto.js';

  let saltText = '';
  let showPassword = false;
  let error = '';
  let loading = false;
  let isFirstBoot = false;
  let dbFile = null;

  // Check if first boot when component mounts
  import { onMount } from 'svelte';
  onMount(async () => {
    try {
      const res = await fetch(`/api/auth/salt`);
      if (res.status === 404) {
        isFirstBoot = true;
      }
    } catch (err) {
      console.error("Failed to check auth status", err);
    }
  });

  async function handleSubmit() {
    if (!saltText.trim()) {
      error = 'Master password is required';
      return;
    }
    loading = true;
    error = '';
    try {
      const key = await deriveKey(saltText);
      const baseUrl = `/api`;

      if (isFirstBoot) {
        if (dbFile && dbFile[0]) {
          const formData = new FormData();
          formData.append('file', dbFile[0]);
          formData.append('saltText', saltText);
          const res = await fetch(`${baseUrl}/auth/import`, {
            method: 'POST',
            body: formData
          });
          if (!res.ok) throw new Error("Failed to import database: " + await res.text());
          const importData = await res.json().catch(() => ({}));
          if (importData.token) sessionToken.set(importData.token);
        } else {
          // Encrypt the magic word and store it
          const magicEncrypted = await encryptText("FinanceTrackerAuth", key);
          const res = await fetch(`${baseUrl}/auth/salt`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ value: magicEncrypted })
          });
          if (!res.ok) throw new Error("Failed to initialize master password: " + await res.text());
          const saltData = await res.json().catch(() => ({}));
          if (saltData.token) sessionToken.set(saltData.token);

          // Seed default categories using the key
          const defaultCategories = [
            "GROCERIES", "UTILITIES", "RENT", "OTHER", "FIXED COSTS",
            "DATING", "LEISURE", "GIFT", "PET", "PERSONAL COST"
          ];
          const authHeaders = {
            'Content-Type': 'application/json',
            ...(saltData.token ? { Authorization: `Bearer ${saltData.token}` } : {})
          };
          for (const cat of defaultCategories) {
            const encCat = await encryptText(cat, key);
            await fetch(`${baseUrl}/splits`, {
              method: 'POST',
              headers: authHeaders,
              body: JSON.stringify({ category: encCat, allocations: [] })
            });
          }
        }
      } else {
        // Fetch the magic word and try to decrypt it
        const res = await fetch(`${baseUrl}/auth/salt`);
        if (!res.ok) throw new Error("Database not initialized or unreachable");
        const data = await res.json();
        
        const magicDecrypted = await decryptText(data.value, key);
        if (magicDecrypted !== "FinanceTrackerAuth") {
          throw new Error("Incorrect master password");
        }

        // Establish authenticated session with server
        const loginRes = await fetch(`${baseUrl}/auth/login`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ proof: data.value })
        });
        if (!loginRes.ok) {
          throw new Error("Failed to authenticate session: " + await loginRes.text());
        }
        const loginData = await loginRes.json();
        if (loginData.token) {
          sessionToken.set(loginData.token);
        }
      }

      // Password is correct or initialized
      authSalt.set(saltText);
      cryptoKey.set(key);
    } catch (err) {
      error = err.message || 'Failed to authenticate';
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
              type="text"
              bind:value={saltText}
              placeholder={isFirstBoot ? "Create master passphrase..." : "Enter master passphrase..."}
              class="input-field py-3 text-sm pr-11 font-mono"
              disabled={loading}
            />
          {:else}
            <input
              id="salt"
              type="password"
              bind:value={saltText}
              placeholder={isFirstBoot ? "Create master passphrase..." : "Enter master passphrase..."}
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
            Import Existing Database (Optional)
          </label>
          <input
            id="dbUpload"
            type="file"
            accept=".db,.sqlite"
            bind:files={dbFile}
            class="w-full text-xs text-neutral-600 dark:text-neutral-400 file:mr-4 file:py-2 file:px-4 file:rounded-xl file:border-0 file:text-xs file:font-semibold file:bg-indigo-100 dark:file:bg-indigo-900/50 file:text-indigo-700 dark:file:text-indigo-300 hover:file:opacity-90 transition cursor-pointer"
            disabled={loading}
          />
          <p class="text-[10px] text-neutral-500 mt-1.5">Provide an unencrypted finance.db to load your data. It will be encrypted upon import.</p>
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

