<script>
  import { authSalt, cryptoKey } from './stores.js';
  import { deriveKey, encryptText, decryptText } from './crypto.js';

  let saltText = '';
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
        } else {
          // Encrypt the magic word and store it
          const magicEncrypted = await encryptText("FinanceTrackerAuth", key);
          const res = await fetch(`${baseUrl}/auth/salt`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ value: magicEncrypted })
          });
          if (!res.ok) throw new Error("Failed to initialize master password: " + await res.text());

          // Seed default categories using the key
          const defaultCategories = [
            "GROCERIES", "UTILITIES", "RENT", "OTHER", "FIXED COSTS",
            "DATING", "LEISURE", "GIFT", "PET", "PERSONAL COST"
          ];
          for (const cat of defaultCategories) {
            const encCat = await encryptText(cat, key);
            await fetch(`${baseUrl}/splits`, {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
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

<div class="min-h-screen flex items-center justify-center bg-slate-950 p-4">
  <div class="w-full max-w-md bg-slate-900 border border-slate-800 rounded-2xl p-8 shadow-2xl space-y-6">
    <div class="text-center space-y-2">
      <h1 class="text-3xl font-bold tracking-tight text-white bg-gradient-to-r from-indigo-400 to-cyan-400 bg-clip-text text-transparent">
        Jizifin Finance
      </h1>
      <p class="text-sm text-slate-400">
        {#if isFirstBoot}
          Welcome! Create a master password to encrypt your database.
        {:else}
          Enter your master password to decrypt the database.
        {/if}
      </p>
    </div>

    <form on:submit|preventDefault={handleSubmit} class="space-y-4">
      <div>
        <label for="salt" class="block text-xs font-semibold uppercase tracking-wider text-slate-400 mb-2">
          Master Password
        </label>
        <input
          id="salt"
          type="password"
          bind:value={saltText}
          placeholder={isFirstBoot ? "Create master password..." : "Enter master password..."}
          class="w-full bg-slate-950 border border-slate-800 text-white rounded-xl px-4 py-3 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-transparent placeholder-slate-600 transition"
          disabled={loading}
        />
      </div>

      {#if isFirstBoot}
        <div>
          <label for="dbUpload" class="block text-xs font-semibold uppercase tracking-wider text-slate-400 mb-2">
            Import Existing Database (Optional)
          </label>
          <input
            id="dbUpload"
            type="file"
            accept=".db,.sqlite"
            bind:files={dbFile}
            class="w-full text-sm text-slate-400 file:mr-4 file:py-2 file:px-4 file:rounded-xl file:border-0 file:text-sm file:font-semibold file:bg-indigo-900/50 file:text-indigo-300 hover:file:bg-indigo-900 transition"
            disabled={loading}
          />
          <p class="text-[10px] text-slate-500 mt-1.5">Provide an unencrypted finance.db to load your data. It will be encrypted upon import.</p>
        </div>
      {/if}

      {#if error}
        <div class="text-red-400 text-xs bg-red-950/30 border border-red-900/50 rounded-xl p-3">
          {error}
        </div>
      {/if}

      <div class="flex flex-col gap-3">
        <button
          type="submit"
          class="w-full bg-indigo-600 hover:bg-indigo-500 text-white font-medium py-3 rounded-xl transition flex items-center justify-center text-sm shadow-lg shadow-indigo-600/25 disabled:opacity-50"
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
