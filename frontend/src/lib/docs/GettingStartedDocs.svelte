<script>
  import { createEventDispatcher } from 'svelte';
  import { theme, currencySymbol } from '../stores.js';

  const dispatch = createEventDispatcher();

  let activeStep = 'step-1';

  const steps = [
    { id: 'step-1', title: '1. Master Passphrase & Crypto', icon: '🔑' },
    { id: 'step-2', title: '2. Household Members & Colors', icon: '👥' },
    { id: 'step-3', title: '3. Categories & Split Ratios', icon: '⚖️' },
    { id: 'step-4', title: '4. Joint Household Account', icon: '🏦' },
    { id: 'step-5', title: '5. Income & Employment Jobs', icon: '💼' },
    { id: 'step-6', title: '6. Budgets & Savings Goals', icon: '🎯' },
    { id: 'step-7', title: '7. Expenses, Overrides & Tags', icon: '📝' },
    { id: 'step-8', title: '8. Month-End Settlements', icon: '🔒' },
  ];

  function navigateTo(path) {
    dispatch('navigate', { path });
  }

  function scrollToStep(id) {
    activeStep = id;
    const el = document.getElementById(id);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  }

  function cycleTheme() {
    if ($theme === 'dark') {
      theme.set('light');
    } else if ($theme === 'light') {
      theme.set('system');
    } else {
      theme.set('dark');
    }
  }
</script>

<div class="min-h-screen bg-slate-50 dark:bg-neutral-950 text-neutral-900 dark:text-white font-inter">
  <!-- Top Navigation Bar -->
  <header class="sticky top-0 z-30 bg-white/85 dark:bg-neutral-900/85 backdrop-blur-md border-b border-neutral-200 dark:border-neutral-800">
    <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between gap-4">
      <div class="flex items-center gap-3">
        <button
          on:click={() => navigateTo('/docs')}
          class="flex items-center gap-2 text-xs font-semibold px-2.5 py-1.5 rounded-lg border border-neutral-200 dark:border-neutral-700 bg-neutral-100 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300 hover:bg-neutral-200 dark:hover:bg-neutral-700 transition-colors"
        >
          ← Docs Index
        </button>

        <div class="hidden sm:flex items-center gap-2 text-xs text-neutral-400">
          <span>/</span>
          <span class="font-semibold text-neutral-800 dark:text-neutral-200">Getting Started Guide</span>
        </div>
      </div>

      <div class="flex items-center gap-3">
        <button
          on:click={() => navigateTo('/docs/frontend')}
          class="text-xs text-neutral-500 hover:text-neutral-900 dark:hover:text-white transition-colors hidden md:block"
        >
          Frontend Docs →
        </button>

        <button
          on:click={() => navigateTo('/')}
          class="text-xs font-medium px-3.5 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white transition-colors shadow-xs"
        >
          Open App
        </button>

        <button
          on:click={cycleTheme}
          aria-label="Toggle theme"
          class="w-8 h-8 flex items-center justify-center rounded-lg border border-neutral-200 dark:border-neutral-700 bg-neutral-100 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300 hover:bg-neutral-200 dark:hover:bg-neutral-700 transition-colors text-xs"
        >
          {#if $theme === 'dark'}🌙{:else if $theme === 'light'}☀️{:else}💻{/if}
        </button>
      </div>
    </div>
  </header>

  <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
    <div class="flex gap-8">
      <!-- Sticky Sidebar Table of Contents -->
      <aside class="hidden lg:block w-64 flex-none sticky top-24 self-start space-y-4 max-h-[calc(100vh-7rem)] overflow-y-auto pr-2">
        <div class="p-3 bg-neutral-100 dark:bg-neutral-900 rounded-xl border border-neutral-200 dark:border-neutral-800">
          <p class="text-[11px] font-bold uppercase tracking-wider text-neutral-500 dark:text-neutral-400 mb-2 px-2">
            Setup Checklist
          </p>
          <nav class="space-y-0.5">
            {#each steps as step}
              <button
                on:click={() => scrollToStep(step.id)}
                class="w-full text-left text-xs px-2 py-1.5 rounded-lg transition-all flex items-center gap-2
                       {activeStep === step.id
                         ? 'bg-indigo-600 text-white font-semibold'
                         : 'text-neutral-600 dark:text-neutral-400 hover:bg-neutral-200 dark:hover:bg-neutral-800 hover:text-neutral-900 dark:hover:text-white'}"
              >
                <span class="truncate">{step.title}</span>
              </button>
            {/each}
          </nav>
        </div>

        <div class="p-3 bg-indigo-50/50 dark:bg-indigo-950/20 rounded-xl border border-indigo-200 dark:border-indigo-900/60 text-xs">
          <p class="font-bold text-indigo-900 dark:text-indigo-200 mb-1">Zero-Knowledge</p>
          <p class="text-indigo-700 dark:text-indigo-300 text-[11px] leading-relaxed">
            All data is encrypted in the browser using PBKDF2/AES-GCM before writing to the database.
          </p>
        </div>
      </aside>

      <!-- Main Step-by-Step Content -->
      <main class="flex-1 min-w-0 space-y-10">
        <!-- Title Banner -->
        <div class="border-b border-neutral-200 dark:border-neutral-800 pb-5">
          <span class="text-xs font-semibold text-emerald-600 dark:text-emerald-400">Step-by-Step Walkthrough</span>
          <h1 class="text-2xl sm:text-3xl font-extrabold tracking-tight text-neutral-900 dark:text-white mt-1">
            Getting Started with Jizifin
          </h1>
          <p class="text-sm text-neutral-600 dark:text-neutral-400 mt-2 leading-relaxed">
            Configure your household finance instance from initial encryption setup to daily expense logging, joint accounts, and month settlements.
          </p>
        </div>

        <!-- Step 1 -->
        <section id="step-1" class="p-5 rounded-2xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 space-y-3 scroll-mt-24">
          <div class="flex items-center gap-2.5">
            <div class="w-7 h-7 rounded-lg bg-indigo-100 dark:bg-indigo-950/70 text-indigo-600 dark:text-indigo-400 flex items-center justify-center font-bold text-xs">
              1
            </div>
            <h2 class="text-base font-bold text-neutral-900 dark:text-white">
              Initialize Setup and Set Master Passphrase
            </h2>
          </div>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            When you first launch Jizifin, create a <strong>Household Master Passphrase</strong>. The browser uses WebCrypto with PBKDF2 (100,000 SHA-256 iterations) to derive a 256-bit AES-GCM encryption key.
          </p>
          <div class="p-3 bg-neutral-50 dark:bg-neutral-800/50 rounded-xl border border-neutral-200 dark:border-neutral-700/60 text-xs text-neutral-600 dark:text-neutral-300">
            <strong>Security Tip:</strong> All household members use the same master passphrase. Store it in a shared password manager. The database never stores your passphrase in plaintext.
          </div>
        </section>

        <!-- Step 2 -->
        <section id="step-2" class="p-5 rounded-2xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 space-y-3 scroll-mt-24">
          <div class="flex items-center gap-2.5">
            <div class="w-7 h-7 rounded-lg bg-indigo-100 dark:bg-indigo-950/70 text-indigo-600 dark:text-indigo-400 flex items-center justify-center font-bold text-xs">
              2
            </div>
            <h2 class="text-base font-bold text-neutral-900 dark:text-white">
              Add Household Members & Customize Colors
            </h2>
          </div>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            In <strong>Settings &rarr; Household Members</strong>, add each tenant or partner in your household.
          </p>
          <ul class="list-disc list-inside text-xs text-neutral-600 dark:text-neutral-400 space-y-1">
            <li><strong>Name:</strong> Name or nickname (encrypted in database).</li>
            <li><strong>Color:</strong> Pick a distinct color swatch (e.g., `#6366f1` or `#10b981`). Used across charts, payer badges, and debt flows.</li>
          </ul>
        </section>

        <!-- Step 3 -->
        <section id="step-3" class="p-5 rounded-2xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 space-y-3 scroll-mt-24">
          <div class="flex items-center gap-2.5">
            <div class="w-7 h-7 rounded-lg bg-indigo-100 dark:bg-indigo-950/70 text-indigo-600 dark:text-indigo-400 flex items-center justify-center font-bold text-xs">
              3
            </div>
            <h2 class="text-base font-bold text-neutral-900 dark:text-white">
              Configure Expense Categories & Default Split Ratios
            </h2>
          </div>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            In the <strong>Splits</strong> tab, configure the default split ratio for each category:
          </p>
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs">
            <div class="p-3 bg-neutral-50 dark:bg-neutral-800/50 rounded-xl border border-neutral-200 dark:border-neutral-700/60">
              <p class="font-bold text-neutral-900 dark:text-white">Even Split (50% / 50%)</p>
              <p class="text-neutral-500 mt-0.5">Shared groceries, electricity, internet bills.</p>
            </div>
            <div class="p-3 bg-neutral-50 dark:bg-neutral-800/50 rounded-xl border border-neutral-200 dark:border-neutral-700/60">
              <p class="font-bold text-neutral-900 dark:text-white">Income Proportional (e.g. 60% / 40%)</p>
              <p class="text-neutral-500 mt-0.5">Rent or larger household costs split by income.</p>
            </div>
          </div>
          <p class="text-[11px] text-neutral-500">
            Constraint: Total allocations across active members must equal exactly 100.0%.
          </p>
        </section>

        <!-- Step 4 -->
        <section id="step-4" class="p-5 rounded-2xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 space-y-3 scroll-mt-24">
          <div class="flex items-center gap-2.5">
            <div class="w-7 h-7 rounded-lg bg-indigo-100 dark:bg-indigo-950/70 text-indigo-600 dark:text-indigo-400 flex items-center justify-center font-bold text-xs">
              4
            </div>
            <h2 class="text-base font-bold text-neutral-900 dark:text-white">
              Set Up Joint Household Account
            </h2>
          </div>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            In the <strong>Joint Account</strong> tab, manage shared bank funds:
          </p>
          <ul class="list-disc list-inside text-xs text-neutral-600 dark:text-neutral-400 space-y-1">
            <li><strong>Safety Margin Buffer:</strong> Add a percentage (e.g. 10% or 15%) above expected costs to prevent overdrafts.</li>
            <li><strong>Deposit Split Mode:</strong> Choose <em>Salary Proportional</em>, <em>Even</em>, or <em>Manual</em>.</li>
            <li><strong>Expected Costs:</strong> Assign joint categories (e.g., Rent, Water, Groceries) and monthly estimates.</li>
            <li><strong>Balance Corrections:</strong> Log bank transfers into or out of the account.</li>
          </ul>
        </section>

        <!-- Step 5 -->
        <section id="step-5" class="p-5 rounded-2xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 space-y-3 scroll-mt-24">
          <div class="flex items-center gap-2.5">
            <div class="w-7 h-7 rounded-lg bg-indigo-100 dark:bg-indigo-950/70 text-indigo-600 dark:text-indigo-400 flex items-center justify-center font-bold text-xs">
              5
            </div>
            <h2 class="text-base font-bold text-neutral-900 dark:text-white">
              Log Income Streams and Employment Contracts
            </h2>
          </div>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            In the <strong>Income</strong> tab, record salaries and one-off funds:
          </p>
          <ul class="list-disc list-inside text-xs text-neutral-600 dark:text-neutral-400 space-y-1">
            <li><strong>Jobs:</strong> Add employment contracts (monthly, weekly, biweekly, annual) with start and end dates.</li>
            <li><strong>One-Off Income:</strong> Record bonuses, tax refunds, or gifts.</li>
            <li><strong>Salary Overrides:</strong> Adjust a specific month for unpaid leave or overtime.</li>
          </ul>
        </section>

        <!-- Step 6 -->
        <section id="step-6" class="p-5 rounded-2xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 space-y-3 scroll-mt-24">
          <div class="flex items-center gap-2.5">
            <div class="w-7 h-7 rounded-lg bg-indigo-100 dark:bg-indigo-950/70 text-indigo-600 dark:text-indigo-400 flex items-center justify-center font-bold text-xs">
              6
            </div>
            <h2 class="text-base font-bold text-neutral-900 dark:text-white">
              Define Monthly Budgets & Savings Goals (Projects)
            </h2>
          </div>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            In the <strong>Budgets</strong> and <strong>Projects</strong> tabs:
          </p>
          <ul class="list-disc list-inside text-xs text-neutral-600 dark:text-neutral-400 space-y-1">
            <li><strong>Budgets:</strong> Set monthly caps per category to track spending burndown gauges on the dashboard.</li>
            <li><strong>Projects:</strong> Set target goals (e.g., Vacation, Renovation) with target amounts and completion dates.</li>
          </ul>
        </section>

        <!-- Step 7 -->
        <section id="step-7" class="p-5 rounded-2xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 space-y-3 scroll-mt-24">
          <div class="flex items-center gap-2.5">
            <div class="w-7 h-7 rounded-lg bg-indigo-100 dark:bg-indigo-950/70 text-indigo-600 dark:text-indigo-400 flex items-center justify-center font-bold text-xs">
              7
            </div>
            <h2 class="text-base font-bold text-neutral-900 dark:text-white">
              Log Daily Expenses, Split Overrides & Recurring Templates
            </h2>
          </div>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            Use the <strong>Quick Add Expense</strong> form on the dashboard:
          </p>
          <ul class="list-disc list-inside text-xs text-neutral-600 dark:text-neutral-400 space-y-1">
            <li>Select payer, category, amount, and date.</li>
            <li>Check <em>Paid from Joint Account</em> if funded by the joint pool.</li>
            <li>Expand <em>Split Override</em> to customize percentage splits for individual receipts.</li>
            <li>Set up recurring expenses in the <strong>Recurring</strong> tab for automated monthly posting.</li>
          </ul>
        </section>

        <!-- Step 8 -->
        <section id="step-8" class="p-5 rounded-2xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 space-y-3 scroll-mt-24 pb-6">
          <div class="flex items-center gap-2.5">
            <div class="w-7 h-7 rounded-lg bg-indigo-100 dark:bg-indigo-950/70 text-indigo-600 dark:text-indigo-400 flex items-center justify-center font-bold text-xs">
              8
            </div>
            <h2 class="text-base font-bold text-neutral-900 dark:text-white">
              Month-End Settlement and Debt Simplification
            </h2>
          </div>
          <p class="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
            At month end, navigate to the <strong>Paybacks</strong> tab:
          </p>
          <ul class="list-disc list-inside text-xs text-neutral-600 dark:text-neutral-400 space-y-1">
            <li>View simplified reimbursement transfers computed via graph reduction.</li>
            <li>Once bank transfers are completed, click <strong>Lock and Settle Month</strong> to freeze the historical record.</li>
          </ul>
        </section>
      </main>
    </div>
  </div>
</div>
