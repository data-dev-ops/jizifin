<script>
  /**
   * IncomeTab.svelte
   *
   * Complete, unified Income & Employment hub with full Light/Dark theming:
   *   1. Monthly Income Summary cards (Job Base Salary + One-Off Bonuses = Total Month Income)
   *   2. Jobs & Employment Streams (CRUD, timeline, weekly/monthly frequencies, 1-click raise/leave adjustments)
   *   3. One-off Income & Bonus Ledger for the active month
   *   4. Category management link
   *
   * All encryption/decryption is handled transparently by api.js.
   */

  import { createEventDispatcher, onMount } from "svelte";
  import {
    selectedMonth,
    users,
    incomeEntries,
    incomeCategories,
    jobs,
    currencySymbol,
    incomeAnalytics,
    privacyShield
  } from "./stores.js";

  const dispatch = createEventDispatcher();
  import {
    fetchIncome,
    deleteIncome,
    createIncome,
    fetchIncomeCategories,
    createIncomeCategory,
    deleteIncomeCategory,
    fetchJobs,
    createJob,
    updateJob,
    deleteJob,
    fetchIncomeByPerson,
    saveSalaryOverride,
    deleteSalaryOverride
  } from "./api.js";

  // ── Derived Users ──────────────────────────────────────────────────────────
  $: activeUsers = $users.filter((u) => u.is_active);

  // ── Salary Override (Period-Specific Adjustment) State ─────────────────────
  let showSalaryOverrideModal = false;
  let salaryOverrideUser = null;
  let salaryOverrideAmount = "";
  let salaryOverrideNote = "";
  let salaryOverrideContractCents = 0;
  let salaryOverrideHasExisting = false;
  let salaryOverrideError = "";
  let salaryOverrideSaving = false;

  // ── Job Manager State ──────────────────────────────────────────────────────
  let showJobModal = false; // false | "add" | "edit" | "adjust"
  let jobModalTitle = "Add Employment Stream";
  let jobForm = {
    id: null,
    name: "",
    who: "",
    amount: "",
    frequency: "monthly",
    start_date: new Date().toISOString().slice(0, 10),
    end_date: "",
    notes: "",
    is_active: true,
  };

  // Adjust / Raise sub-form state
  let adjustSourceJob = null;
  let adjustEffectiveDate = new Date().toISOString().slice(0, 10);
  let adjustNewAmount = "";
  let adjustNewNotes = "";
  let adjustError = "";
  let adjustSaving = false;

  let jobError = "";
  let jobSaving = false;
  let deletingJobId = null;
  let confirmJobDeleteId = null;

  // ── One-off Income Form State ──────────────────────────────────────────────
  let formName = "";
  let formAmountEur = "";
  let formWho = "";
  let formCategory = "";
  let formDate = new Date().toISOString().slice(0, 10);
  let formIsJoint = false;
  let submitting = false;
  let formError = "";
  let formSuccess = false;

  // ── One-off Ledger State ───────────────────────────────────────────────────
  let deletingId = null;
  let confirmId = null;

  // Default selects
  $: if (activeUsers.length && !formWho) formWho = activeUsers[0]?.name ?? "";
  $: if (activeUsers.length && !jobForm.who) jobForm.who = activeUsers[0]?.name ?? "";
  $: if ($incomeCategories.length && !formCategory) formCategory = $incomeCategories[0]?.category ?? "";

  // ── Formatting Helpers ─────────────────────────────────────────────────────
  function fmt(cents) {
    return `${$currencySymbol}${(cents / 100).toFixed(2)}`;
  }

  function fmtDate(iso) {
    if (!iso) return "Ongoing";
    try {
      return new Date(iso + "T00:00:00").toLocaleDateString(undefined, {
        day: "numeric",
        month: "short",
        year: "numeric",
      });
    } catch {
      return iso;
    }
  }

  function userColor(name) {
    return $users.find((u) => u.name === name)?.color ?? "#6366f1";
  }

  function userInitial(name) {
    return name ? name[0].toUpperCase() : "?";
  }

  const CAT_COLOURS = [
    "#6366f1", "#8b5cf6", "#ec4899", "#f59e0b", "#10b981", "#3b82f6", "#ef4444"
  ];
  function catColour(cat) {
    const idx = $incomeCategories.findIndex((c) => c.category === cat);
    return CAT_COLOURS[idx % CAT_COLOURS.length] ?? "#6366f1";
  }

  function toMonthlyEquivalent(amountCents, freq) {
    if (freq === "weekly") return Math.round((amountCents * 52) / 12);
    if (freq === "biweekly") return Math.round((amountCents * 26) / 12);
    if (freq === "annual") return Math.round(amountCents / 12);
    return amountCents;
  }

  function isJobActiveInMonth(job, monthStr) {
    if (!monthStr || !job.is_active) return false;
    const startMonth = fMonthStart(monthStr);
    const endMonth = fMonthEnd(monthStr);
    return job.start_date <= endMonth && (!job.end_date || job.end_date >= startMonth);
  }

  function fMonthStart(m) {
    return `${m}-01`;
  }

  function fMonthEnd(m) {
    return `${m}-31`;
  }

  function getJobStatusBadge(job, targetMonth) {
    if (!job.is_active) return { label: "Inactive", cls: "bg-neutral-100 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-400 border-neutral-200 dark:border-neutral-700" };
    const today = new Date().toISOString().slice(0, 10);
    if (job.start_date > today) return { label: "Upcoming", cls: "bg-sky-50 dark:bg-sky-950/80 text-sky-700 dark:text-sky-300 border-sky-200 dark:border-sky-800" };
    if (job.end_date && job.end_date < (targetMonth ? fMonthStart(targetMonth) : today)) {
      return { label: "Ended", cls: "bg-neutral-100 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-400 border-neutral-200 dark:border-neutral-700" };
    }
    if (job.notes && /leave|sick|sabbatical|parental/i.test(job.notes)) {
      return { label: "On Leave", cls: "bg-amber-50 dark:bg-amber-950/80 text-amber-700 dark:text-amber-300 border-amber-200 dark:border-amber-800" };
    }
    return { label: "Active", cls: "bg-emerald-50 dark:bg-emerald-950/80 text-emerald-700 dark:text-emerald-300 border-emerald-200 dark:border-emerald-800" };
  }

  // ── Month breakdown calculations ───────────────────────────────────────────
  $: userSummaryList = activeUsers.map((u) => {
    // 1. Contract base salary from active jobs in $selectedMonth
    const userJobs = $jobs.filter((j) => j.who === u.name && isJobActiveInMonth(j, $selectedMonth));
    const contractSalaryCents = userJobs.reduce((sum, j) => sum + toMonthlyEquivalent(j.amount_cents, j.frequency), 0);

    const row = $incomeAnalytics.find((r) => r.who === u.name);
    let baseSalaryCents = contractSalaryCents;
    let hasOverride = false;
    let overrideNote = "";

    if (row) {
      baseSalaryCents = row.base_salary_cents;
      hasOverride = Boolean(row.has_override);
      overrideNote = row.override_note || "";
    } else if (userJobs.length === 0) {
      baseSalaryCents = 0;
    }

    // 2. One-off income this month
    const oneOffCents = $incomeEntries
      .filter((e) => e.who === u.name && e.category !== "SALARY")
      .reduce((sum, e) => sum + e.amount_cents, 0);

    const totalCents = baseSalaryCents + oneOffCents;
    return {
      name: u.name,
      color: u.color,
      activeJobs: userJobs,
      baseSalaryCents,
      contractSalaryCents: row?.contract_salary_cents ?? contractSalaryCents,
      hasOverride,
      overrideNote,
      oneOffCents,
      totalCents,
    };
  });

  $: totalHouseholdIncome = userSummaryList.reduce((sum, u) => sum + u.totalCents, 0);

  onMount(async () => {
    try {
      await Promise.all([fetchIncomeCategories(), fetchJobs(), fetchIncome($selectedMonth), fetchIncomeByPerson($selectedMonth)]);
    } catch (e) {
      console.error(e);
    }
  });

  // ── Month change ────────────────────────────────────────────────────────────
  $: if ($selectedMonth) {
    fetchIncome($selectedMonth);
    fetchIncomeByPerson($selectedMonth);
  }

  // ── Salary Override Actions ────────────────────────────────────────────────
  function openSalaryOverrideModal(u) {
    salaryOverrideUser = u;
    salaryOverrideContractCents = u.contractSalaryCents;
    salaryOverrideHasExisting = u.hasOverride;
    salaryOverrideAmount = (u.baseSalaryCents / 100).toFixed(2);
    salaryOverrideNote = u.overrideNote || "";
    salaryOverrideError = "";
    showSalaryOverrideModal = true;
  }

  function closeSalaryOverrideModal() {
    showSalaryOverrideModal = false;
    salaryOverrideError = "";
    salaryOverrideSaving = false;
  }

  async function handleSaveSalaryOverride() {
    salaryOverrideError = "";
    const amountCents = Math.round(parseFloat(salaryOverrideAmount) * 100);
    if (isNaN(amountCents) || amountCents < 0) {
      salaryOverrideError = "Please enter a valid salary amount (0 or greater).";
      return;
    }

    salaryOverrideSaving = true;
    try {
      await saveSalaryOverride({
        user_name: salaryOverrideUser.name,
        month: $selectedMonth,
        amount_cents: amountCents,
        note: salaryOverrideNote.trim() || null,
      });
      await fetchIncomeByPerson($selectedMonth);
      closeSalaryOverrideModal();
    } catch (err) {
      salaryOverrideError = err.message;
    } finally {
      salaryOverrideSaving = false;
    }
  }

  async function handleResetSalaryOverride() {
    salaryOverrideError = "";
    salaryOverrideSaving = true;
    try {
      await deleteSalaryOverride(salaryOverrideUser.name, $selectedMonth);
      await fetchIncomeByPerson($selectedMonth);
      closeSalaryOverrideModal();
    } catch (err) {
      salaryOverrideError = err.message;
    } finally {
      salaryOverrideSaving = false;
    }
  }

  // ── Job Actions ────────────────────────────────────────────────────────────
  function openAddJobModal(prefillUser = "") {
    jobModalTitle = "Add Employment Stream";
    jobError = "";
    jobForm = {
      id: null,
      name: "",
      who: prefillUser || activeUsers[0]?.name || "",
      amount: "",
      frequency: "monthly",
      start_date: `${$selectedMonth || new Date().toISOString().slice(0, 7)}-01`,
      end_date: "",
      notes: "",
      is_active: true,
    };
    showJobModal = "add";
  }

  function openEditJobModal(job) {
    jobModalTitle = "Edit Employment Stream";
    jobError = "";
    jobForm = {
      id: job.id,
      name: job.name,
      who: job.who,
      amount: (job.amount_cents / 100).toFixed(2),
      frequency: job.frequency || "monthly",
      start_date: job.start_date,
      end_date: job.end_date || "",
      notes: job.notes || "",
      is_active: job.is_active,
    };
    showJobModal = "edit";
  }

  function openAdjustJobModal(job) {
    adjustSourceJob = job;
    adjustEffectiveDate = `${$selectedMonth || new Date().toISOString().slice(0, 7)}-01`;
    adjustNewAmount = (job.amount_cents / 100).toFixed(2);
    adjustNewNotes = "Promotion / Rate Adjustment";
    adjustError = "";
    showJobModal = "adjust";
  }

  function closeJobModal() {
    showJobModal = false;
    jobError = "";
    adjustError = "";
  }

  async function handleSaveJob() {
    jobError = "";
    const amountCents = Math.round(parseFloat(jobForm.amount) * 100);
    if (!jobForm.name.trim()) { jobError = "Job or Employer title is required."; return; }
    if (!amountCents || amountCents <= 0) { jobError = "Please enter a valid pay amount."; return; }
    if (!jobForm.who) { jobError = "Select a household member."; return; }
    if (!jobForm.start_date) { jobError = "Start date is required."; return; }
    if (jobForm.end_date && jobForm.end_date < jobForm.start_date) {
      jobError = "End date cannot be earlier than start date."; return;
    }

    jobSaving = true;
    try {
      const payload = {
        name: jobForm.name.trim(),
        who: jobForm.who,
        amount_cents: amountCents,
        frequency: jobForm.frequency,
        start_date: jobForm.start_date,
        end_date: jobForm.end_date ? jobForm.end_date : null,
        notes: jobForm.notes.trim() || null,
        is_active: jobForm.is_active,
      };

      if (showJobModal === "edit" && jobForm.id) {
        await updateJob(jobForm.id, payload);
      } else {
        await createJob(payload);
      }
      await fetchJobs();
      await fetchIncomeByPerson($selectedMonth);
      closeJobModal();
    } catch (err) {
      jobError = err.message;
    } finally {
      jobSaving = false;
    }
  }

  async function handleSaveAdjustment() {
    adjustError = "";
    const newAmountCents = Math.round(parseFloat(adjustNewAmount) * 100);
    if (!newAmountCents || newAmountCents <= 0) {
      adjustError = "Please enter a valid new pay rate."; return;
    }
    if (!adjustEffectiveDate) {
      adjustError = "Please select an effective start date."; return;
    }

    adjustSaving = true;
    try {
      // 1. Calculate previous job end date: 1 day prior to effective date
      const effDateObj = new Date(adjustEffectiveDate + "T00:00:00");
      effDateObj.setDate(effDateObj.getDate() - 1);
      const prevEndDate = effDateObj.toISOString().slice(0, 10);

      // 2. Update existing job end date
      await updateJob(adjustSourceJob.id, {
        end_date: prevEndDate,
      });

      // 3. Insert new job stream starting from effective date
      await createJob({
        name: adjustSourceJob.name,
        who: adjustSourceJob.who,
        amount_cents: newAmountCents,
        frequency: adjustSourceJob.frequency,
        start_date: adjustEffectiveDate,
        end_date: null,
        notes: adjustNewNotes.trim() || null,
        is_active: true,
      });

      await fetchJobs();
      await fetchIncomeByPerson($selectedMonth);
      closeJobModal();
    } catch (err) {
      adjustError = err.message;
    } finally {
      adjustSaving = false;
    }
  }

  async function confirmDeleteJob(id) {
    if (confirmJobDeleteId !== id) {
      confirmJobDeleteId = id;
      return;
    }
    confirmJobDeleteId = null;
    deletingJobId = id;
    try {
      await deleteJob(id);
      await fetchJobs();
      await fetchIncomeByPerson($selectedMonth);
    } catch (err) {
      console.error(err);
    } finally {
      deletingJobId = null;
    }
  }

  // ── One-off Income Submit ──────────────────────────────────────────────────
  async function submitOneOffIncome() {
    formError = "";
    formSuccess = false;
    const amountCents = Math.round(parseFloat(formAmountEur) * 100);
    if (!formName.trim()) { formError = "Name is required."; return; }
    if (!amountCents || amountCents <= 0) { formError = "Enter a positive amount."; return; }
    if (!formWho) { formError = "Select a person."; return; }
    if (!formCategory) { formError = "Select a category."; return; }
    if (!formDate) { formError = "Select a date."; return; }

    submitting = true;
    try {
      await createIncome(
        [{
          name: formName.trim(),
          amount_cents: amountCents,
          who: formWho,
          category: formCategory,
          income_date: formDate,
          is_joint: formIsJoint
        }],
        $selectedMonth
      );
      await fetchIncome($selectedMonth);
      await fetchIncomeByPerson($selectedMonth);
      formName = "";
      formAmountEur = "";
      formSuccess = true;
      setTimeout(() => (formSuccess = false), 3000);
    } catch (err) {
      formError = err.message;
    } finally {
      submitting = false;
    }
  }

  async function confirmDeleteEntry(id) {
    if (confirmId !== id) { confirmId = id; return; }
    confirmId = null;
    deletingId = id;
    try {
      await deleteIncome(id);
      await fetchIncomeByPerson($selectedMonth);
    } catch (err) {
      console.error(err);
    } finally {
      deletingId = null;
    }
  }
</script>

<!-- ── Page Header ──────────────────────────────────────────────────────────── -->
<header class="page-header">
  <div>
    <h1 class="page-title flex items-center gap-2.5">
      <span>💰</span> Income & Employment Streams
    </h1>
    <p class="page-subtitle">
      Define salaries, timeline promotions, and one-off bonuses for accurate monthly budgeting and split ratios.
    </p>
  </div>
  <button
    id="btn-add-job-top"
    on:click={() => openAddJobModal()}
    class="btn-primary self-start sm:self-auto"
  >
    <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
      <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
    </svg>
    <span>+ Add Job / Income Stream</span>
  </button>
</header>

<!-- ── 1. Monthly Summary Cards ─────────────────────────────────────────────── -->
<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
  {#each userSummaryList as u (u.name)}
    {@const sharePct = totalHouseholdIncome > 0 ? ((u.totalCents / totalHouseholdIncome) * 100).toFixed(1) : "0.0"}
    <div
      class="card p-4 sm:p-5 relative overflow-hidden transition-all"
      style="border-color: {u.color}50"
    >
      <div class="flex items-center justify-between gap-2 mb-3">
        <div class="flex items-center gap-2">
          <div
            class="w-6 h-6 rounded-full flex items-center justify-center text-[10px] font-bold text-white shrink-0"
            style="background-color: {u.color}"
          >
            {userInitial(u.name)}
          </div>
          <span class="text-xs font-semibold text-neutral-800 dark:text-neutral-200">{u.name}</span>
        </div>
        <span
          class="text-[10px] font-bold px-2 py-0.5 rounded-full"
          style="background-color: {u.color}15; color: {u.color}; border: 1px solid {u.color}40"
        >
          {sharePct}% ratio
        </span>
      </div>

      <p class="text-2xl font-bold tabular-nums" style="color: {u.color}">
        <span class:privacy-masked={$privacyShield}>{fmt(u.totalCents)}</span>
      </p>

      <!-- Sub-breakdown -->
      <div class="mt-3 pt-3 border-t border-neutral-200 dark:border-neutral-800/80 flex items-center justify-between text-[11px] text-neutral-500 dark:text-neutral-400">
        <div class="flex items-center gap-1.5 flex-wrap">
          <span class="text-neutral-500">Base Salary:</span>
          <span class="font-medium text-neutral-700 dark:text-neutral-300 tabular-nums"><span class:privacy-masked={$privacyShield}>{fmt(u.baseSalaryCents)}</span></span>
          {#if u.hasOverride}
            <span
              class="inline-flex items-center px-1.5 py-0.2 rounded text-[10px] font-semibold bg-amber-50 dark:bg-amber-950/80 text-amber-700 dark:text-amber-300 border border-amber-200 dark:border-amber-800"
              title={u.overrideNote ? `Adjusted: ${u.overrideNote}` : 'Adjusted for this period'}
            >
              ⚡ Adjusted
            </span>
          {/if}
        </div>
        {#if u.oneOffCents > 0}
          <div>
            <span class="text-neutral-500">Bonuses:</span>
            <span class="font-medium text-emerald-600 dark:text-emerald-400 tabular-nums ml-1"><span class:privacy-masked={$privacyShield}>+{fmt(u.oneOffCents)}</span></span>
          </div>
        {/if}
      </div>

      <!-- Action to adjust salary for this specific month -->
      <div class="mt-2.5 pt-2 border-t border-dashed border-neutral-200 dark:border-neutral-800/60 flex items-center justify-between">
        <button
          id="btn-adjust-salary-{u.name}"
          type="button"
          on:click={() => openSalaryOverrideModal(u)}
          class="text-[11px] font-medium text-indigo-600 dark:text-indigo-400 hover:text-indigo-700 dark:hover:text-indigo-300 transition-colors flex items-center gap-1 cursor-pointer"
        >
          <span>✏️ {u.hasOverride ? `Edit ${$selectedMonth} Salary` : `Adjust for ${$selectedMonth}`}</span>
        </button>
        {#if u.hasOverride}
          <span class="text-[10px] text-neutral-400 dark:text-neutral-500 font-mono">
            Contract: <span class:privacy-masked={$privacyShield}>{fmt(u.contractSalaryCents)}</span>
          </span>
        {/if}
      </div>
    </div>
  {/each}

  <!-- Total Household Summary Card -->
  <div class="card p-4 sm:p-5 sm:col-span-2 lg:col-span-1 border-indigo-200 dark:border-indigo-900/40 bg-indigo-50/50 dark:bg-gradient-to-br dark:from-neutral-900 dark:to-indigo-950/40">
    <div class="flex items-center justify-between gap-2 mb-3">
      <span class="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">Household Total</span>
      <span class="badge-indigo">{$selectedMonth}</span>
    </div>
    <p class="text-2xl font-bold tabular-nums text-neutral-900 dark:text-white">
      <span class:privacy-masked={$privacyShield}>{fmt(totalHouseholdIncome)}</span>
    </p>
    <p class="text-[11px] text-neutral-500 mt-3 pt-3 border-t border-neutral-200 dark:border-neutral-800">
      Derived from active contracts & monthly logs
    </p>
  </div>
</div>

<!-- ── 2. Jobs & Employment Streams Section ─────────────────────────────────── -->
<div class="card p-5 sm:p-6 mb-8">
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 mb-6">
    <div>
      <h3 class="text-base font-semibold text-neutral-800 dark:text-neutral-200">Employment Streams & Contracts</h3>
      <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">
        Active jobs automatically supply monthly base salary for the selected period.
      </p>
    </div>
  </div>

  {#if $jobs.length === 0}
    <div class="text-center py-12 px-4 border border-dashed border-neutral-200 dark:border-neutral-800 rounded-2xl bg-neutral-50/50 dark:bg-neutral-950/30">
      <div class="w-12 h-12 rounded-2xl bg-indigo-100 dark:bg-indigo-950/60 border border-indigo-200 dark:border-indigo-800/60 flex items-center justify-center mx-auto mb-3 text-xl">
        💼
      </div>
      <p class="text-sm font-semibold text-neutral-700 dark:text-neutral-300">No employment streams configured yet</p>
      <p class="text-xs text-neutral-500 max-w-md mx-auto mt-1 mb-5">
        Add your primary job or regular freelance streams with weekly or monthly rates to automate income calculations.
      </p>
      <button
        id="btn-add-first-job"
        on:click={() => openAddJobModal()}
        class="btn-primary"
      >
        + Add First Employment Stream
      </button>
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
      {#each $jobs as job (job.id)}
        {@const userObj = $users.find((u) => u.name === job.who)}
        {@const color = userObj?.color ?? "#6366f1"}
        {@const statusBadge = getJobStatusBadge(job, $selectedMonth)}
        {@const monthlyEquiv = toMonthlyEquivalent(job.amount_cents, job.frequency)}
        {@const isFreqNonMonthly = job.frequency !== "monthly"}

        <div class="card-sub p-4 flex flex-col justify-between hover:border-neutral-400 dark:hover:border-neutral-700/80 transition-all group">
          <!-- Card Header -->
          <div>
            <div class="flex items-start justify-between gap-2 mb-2">
              <div class="flex items-center gap-2 min-w-0">
                <div
                  class="w-5 h-5 rounded-full flex items-center justify-center text-[9px] font-bold text-white shrink-0"
                  style="background-color: {color}"
                >
                  {userInitial(job.who)}
                </div>
                <span class="text-xs font-medium text-neutral-500 dark:text-neutral-400 truncate">{job.who}</span>
              </div>
              <span class="inline-flex items-center px-2 py-0.5 rounded text-[10px] font-semibold border {statusBadge.cls}">
                {statusBadge.label}
              </span>
            </div>

            <!-- Job Title -->
            <h4 class="text-sm font-semibold text-neutral-900 dark:text-neutral-100 truncate mb-2">{job.name}</h4>

            <!-- Rate Display -->
            <div class="mb-3">
              <div class="flex items-baseline gap-1.5">
                <span class="text-lg font-bold tabular-nums text-neutral-900 dark:text-white">
                  {fmt(job.amount_cents)}
                </span>
                <span class="text-xs text-neutral-500 capitalize">/ {job.frequency}</span>
              </div>
              {#if isFreqNonMonthly}
                <p class="text-[11px] text-indigo-600 dark:text-indigo-400 mt-0.5 tabular-nums font-medium">
                  ≈ {fmt(monthlyEquiv)} / month
                </p>
              {/if}
            </div>

            <!-- Timeline & Notes -->
            <div class="space-y-1 text-xs text-neutral-500 dark:text-neutral-400 border-t border-neutral-200 dark:border-neutral-800/80 pt-2.5 mb-4">
              <div class="flex items-center justify-between text-[11px]">
                <span class="text-neutral-500">Timeline:</span>
                <span class="text-neutral-700 dark:text-neutral-300 font-mono text-[10px]">
                  {fmtDate(job.start_date)} → {fmtDate(job.end_date)}
                </span>
              </div>
              {#if job.notes}
                <div class="flex items-center justify-between text-[11px]">
                  <span class="text-neutral-500">Note:</span>
                  <span class="text-neutral-700 dark:text-neutral-300 truncate max-w-[170px]" title={job.notes}>{job.notes}</span>
                </div>
              {/if}
            </div>
          </div>

          <!-- Card Actions -->
          <div class="flex items-center justify-between gap-2 border-t border-neutral-200 dark:border-neutral-800/60 pt-3">
            <button
              on:click={() => openAdjustJobModal(job)}
              title="Record a raise, promotion or leave starting from a date"
              class="text-[11px] font-semibold text-indigo-600 dark:text-indigo-400 hover:text-indigo-700 dark:hover:text-indigo-300 transition-colors flex items-center gap-1 cursor-pointer"
            >
              <span>Adjust Rate / Raise →</span>
            </button>

            <div class="flex items-center gap-1">
              <button
                on:click={() => openEditJobModal(job)}
                title="Edit job details"
                class="p-1.5 rounded-lg text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 hover:bg-neutral-100 dark:hover:bg-neutral-800 transition-colors text-xs"
              >
                ✏️
              </button>

              {#if confirmJobDeleteId === job.id}
                <div class="flex items-center gap-1">
                  <button
                    on:click={() => confirmDeleteJob(job.id)}
                    disabled={deletingJobId === job.id}
                    class="px-2 py-0.5 rounded text-[10px] font-bold bg-rose-600 hover:bg-rose-500 text-white transition-colors"
                  >
                    {deletingJobId === job.id ? "…" : "Confirm"}
                  </button>
                  <button
                    on:click={() => (confirmJobDeleteId = null)}
                    class="px-1.5 py-0.5 rounded text-[10px] bg-neutral-200 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-400 hover:bg-neutral-300 dark:hover:text-white"
                  >
                    ✕
                  </button>
                </div>
              {:else}
                <button
                  on:click={() => confirmDeleteJob(job.id)}
                  title="Delete job stream"
                  class="p-1.5 rounded-lg text-neutral-400 hover:text-rose-600 dark:hover:text-red-400 hover:bg-rose-50 dark:hover:bg-red-950/40 transition-colors text-xs"
                >
                  🗑️
                </button>
              {/if}
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- ── 3. One-Off Income & Bonuses Section ───────────────────────────────────── -->
<div class="grid grid-cols-1 lg:grid-cols-5 gap-6 mb-8">

  <!-- Log Form (2 cols) -->
  <div class="lg:col-span-2 card">
    <h3 class="text-sm font-semibold text-neutral-800 dark:text-neutral-200 mb-1">Log One-Off Income & Bonuses</h3>
    <p class="text-xs text-neutral-500 dark:text-neutral-400 mb-4">
      Log bonuses, tax returns, gifts, or dividends for {$selectedMonth}.
    </p>

    <div class="space-y-3">
      <div>
        <label for="income-name" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1">Description</label>
        <input
          id="income-name"
          type="text"
          bind:value={formName}
          placeholder="e.g. Q1 Performance Bonus"
          class="input-field"
        />
      </div>

      <div class="grid grid-cols-2 gap-3">
        <div>
          <label for="income-amount" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1">Amount ({$currencySymbol})</label>
          <input
            id="income-amount"
            type="number"
            min="0.01"
            step="0.01"
            bind:value={formAmountEur}
            placeholder="0.00"
            class="input-field tabular-nums"
          />
        </div>
        <div>
          <label for="income-who" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1">Person</label>
          <select
            id="income-who"
            bind:value={formWho}
            class="select-field"
          >
            {#each activeUsers as u}
              <option value={u.name}>{u.name}</option>
            {/each}
          </select>
        </div>
      </div>

      <div class="grid grid-cols-2 gap-3">
        <div>
          <label for="income-category" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1">Category</label>
          <select
            id="income-category"
            bind:value={formCategory}
            class="select-field"
          >
            {#each $incomeCategories as c}
              <option value={c.category}>{c.category}</option>
            {/each}
          </select>
        </div>
        <div>
          <label for="income-date" class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1">Date</label>
          <input
            id="income-date"
            type="date"
            bind:value={formDate}
            class="input-field"
          />
        </div>
      </div>

      <div>
        <span class="block text-xs font-medium text-neutral-700 dark:text-neutral-400 mb-1">Destination Account</span>
        <div class="grid grid-cols-2 gap-2">
          <button
            type="button"
            on:click={() => (formIsJoint = false)}
            class="py-2 px-3 rounded-xl text-xs font-semibold border transition-all {!formIsJoint ? 'bg-indigo-600 text-white border-indigo-500 shadow-md shadow-indigo-600/30' : 'bg-neutral-100 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-400 border-neutral-200 dark:border-neutral-700 hover:text-neutral-900 dark:hover:text-neutral-200'}"
          >
            Personal Account
          </button>
          <button
            type="button"
            on:click={() => (formIsJoint = true)}
            class="py-2 px-3 rounded-xl text-xs font-semibold border transition-all {formIsJoint ? 'bg-indigo-600 text-white border-indigo-500 shadow-md shadow-indigo-600/30' : 'bg-neutral-100 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-400 border-neutral-200 dark:border-neutral-700 hover:text-neutral-900 dark:hover:text-neutral-200'}"
          >
            🏦 Joint Account
          </button>
        </div>
      </div>

      {#if formError}
        <p class="text-xs text-rose-700 dark:text-red-400 bg-rose-50 dark:bg-red-950/40 border border-rose-200 dark:border-red-800 rounded-xl px-3 py-2">{formError}</p>
      {/if}
      {#if formSuccess}
        <p class="text-xs text-emerald-700 dark:text-emerald-400 bg-emerald-50 dark:bg-emerald-950/40 border border-emerald-200 dark:border-emerald-800 rounded-xl px-3 py-2">✓ Income entry recorded!</p>
      {/if}

      <button
        id="income-submit"
        type="button"
        on:click={submitOneOffIncome}
        disabled={submitting}
        class="btn-primary w-full mt-2"
      >
        {submitting ? "Saving…" : "+ Log Income"}
      </button>
    </div>
  </div>

  <!-- Ledger (3 cols) -->
  <div class="lg:col-span-3 card">
    <div class="flex items-center justify-between mb-4">
      <div>
        <h3 class="text-sm font-semibold text-neutral-800 dark:text-neutral-200">One-Off Income Log</h3>
        <p class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">Recorded bonuses and additions for {$selectedMonth}</p>
      </div>
      <span class="badge-neutral">
        {$incomeEntries.length} entries
      </span>
    </div>

    {#if $incomeEntries.length === 0}
      <div class="empty-state-box">
        <p class="text-neutral-700 dark:text-neutral-400 text-sm font-medium">No income recorded for {$selectedMonth}.</p>
        <p class="text-neutral-500 text-xs mt-1">Use the form on the left to record bonuses, dividends, or gifts.</p>
      </div>
    {:else}
      <div class="space-y-2 max-h-[380px] overflow-y-auto pr-1">
        {#each $incomeEntries as entry (entry.id)}
          {@const color = userColor(entry.who)}
          {@const badgeColor = catColour(entry.category)}
          <div class="card-sub p-3 flex items-center gap-3 hover:border-neutral-300 dark:hover:border-neutral-700 transition-colors group">
            <div
              class="w-7 h-7 rounded-full flex items-center justify-center text-[10px] font-bold text-white shrink-0"
              style="background-color: {color}"
            >
              {userInitial(entry.who)}
            </div>

            <div class="flex-1 min-w-0">
              <p class="text-sm font-semibold text-neutral-900 dark:text-neutral-200 truncate">{entry.name}</p>
              <div class="flex items-center gap-2 mt-0.5 flex-wrap text-xs">
                <span
                  class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-semibold"
                  style="background-color: {badgeColor}18; color: {badgeColor}"
                >
                  {entry.category}
                </span>
                {#if entry.is_joint}
                  <span class="badge-indigo">
                    🏦 Joint
                  </span>
                {/if}
                <span class="text-[11px] text-neutral-500">{fmtDate(entry.income_date)}</span>
              </div>
            </div>

            <p class="text-sm font-bold tabular-nums shrink-0" style="color: {color}">
              {fmt(entry.amount_cents)}
            </p>

            <button
              id="delete-income-{entry.id}"
              on:click={() => confirmDeleteEntry(entry.id)}
              disabled={deletingId === entry.id}
              class="shrink-0 w-7 h-7 rounded-lg flex items-center justify-center text-xs transition-all opacity-0 group-hover:opacity-100 {confirmId === entry.id ? 'bg-rose-600 text-white opacity-100' : 'bg-neutral-100 dark:bg-neutral-700 text-neutral-500 dark:text-neutral-400 hover:bg-rose-50 dark:hover:bg-red-900/60 hover:text-rose-600 dark:hover:text-red-400'} disabled:opacity-30 cursor-pointer"
            >
              {deletingId === entry.id ? "…" : confirmId === entry.id ? "!" : "×"}
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<!-- ── 4. Categories Link Card ──────────────────────────────────────────────── -->
<div class="card p-4 flex flex-col sm:flex-row sm:items-center justify-between gap-3 text-xs">
  <div class="flex items-center gap-2 text-neutral-600 dark:text-neutral-400">
    <span>🏷️</span>
    <span>Manage expense and income category tags in the <strong>Categories</strong> tab.</span>
  </div>
  <button
    id="link-manage-categories"
    type="button"
    on:click={() => dispatch("navigateCategories")}
    class="text-indigo-600 dark:text-indigo-400 font-semibold hover:underline transition-colors cursor-pointer whitespace-nowrap self-start sm:self-auto"
  >
    Go to Categories →
  </button>
</div>

<!-- ── Job Modal (Add / Edit) ──────────────────────────────────────────────── -->
{#if showJobModal === "add" || showJobModal === "edit"}
  <div class="fixed inset-0 bg-black/60 dark:bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-4">
    <div class="card p-6 max-w-md w-full shadow-2xl">
      <div class="flex items-center justify-between mb-4 border-b border-neutral-200 dark:border-neutral-800 pb-3">
        <h3 class="text-base font-semibold text-neutral-900 dark:text-neutral-100">{jobModalTitle}</h3>
        <button on:click={closeJobModal} class="text-neutral-400 hover:text-neutral-700 dark:hover:text-white text-lg">✕</button>
      </div>

      <div class="space-y-3.5">
        <div>
          <label for="modal-job-name" class="block text-xs font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-1">Job / Employer Title</label>
          <input
            id="modal-job-name"
            type="text"
            bind:value={jobForm.name}
            placeholder="e.g. Senior Software Engineer"
            class="input-field"
          />
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div>
            <label for="modal-job-who" class="block text-xs font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-1">Person</label>
            <select
              id="modal-job-who"
              bind:value={jobForm.who}
              class="select-field"
            >
              {#each activeUsers as u}
                <option value={u.name}>{u.name}</option>
              {/each}
            </select>
          </div>
          <div>
            <label for="modal-job-freq" class="block text-xs font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-1">Frequency</label>
            <select
              id="modal-job-freq"
              bind:value={jobForm.frequency}
              class="select-field"
            >
              <option value="monthly">Monthly</option>
              <option value="weekly">Weekly</option>
              <option value="biweekly">Bi-weekly</option>
              <option value="annual">Annual</option>
            </select>
          </div>
        </div>

        <div>
          <label for="modal-job-amount" class="block text-xs font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-1">
            Rate ({$currencySymbol})
          </label>
          <div class="relative">
            <input
              id="modal-job-amount"
              type="number"
              min="0.01"
              step="0.01"
              bind:value={jobForm.amount}
              placeholder="0.00"
              class="input-field tabular-nums"
            />
            {#if jobForm.amount && jobForm.frequency !== "monthly"}
              {@const equivCents = toMonthlyEquivalent(Math.round(parseFloat(jobForm.amount) * 100), jobForm.frequency)}
              <span class="text-[11px] text-indigo-600 dark:text-indigo-400 mt-1 block font-medium">
                ≈ {fmt(equivCents)} / month equivalent
              </span>
            {/if}
          </div>
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div>
            <label for="modal-job-start" class="block text-xs font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-1">Start Date</label>
            <input
              id="modal-job-start"
              type="date"
              bind:value={jobForm.start_date}
              class="input-field"
            />
          </div>
          <div>
            <label for="modal-job-end" class="block text-xs font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-1">End Date (optional)</label>
            <input
              id="modal-job-end"
              type="date"
              bind:value={jobForm.end_date}
              class="input-field"
            />
          </div>
        </div>

        <div>
          <label for="modal-job-notes" class="block text-xs font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-1">Notes / Tags (optional)</label>
          <input
            id="modal-job-notes"
            type="text"
            bind:value={jobForm.notes}
            placeholder="e.g. Full-time, Promotion, Leave"
            class="input-field"
          />
        </div>

        {#if jobError}
          <p class="text-xs text-rose-700 dark:text-red-400 bg-rose-50 dark:bg-red-950/40 border border-rose-200 dark:border-red-800 rounded-xl px-3 py-2">{jobError}</p>
        {/if}

        <div class="flex items-center justify-end gap-2 pt-2">
          <button
            type="button"
            on:click={closeJobModal}
            class="btn-secondary"
          >
            Cancel
          </button>
          <button
            type="button"
            on:click={handleSaveJob}
            disabled={jobSaving}
            class="btn-primary"
          >
            {jobSaving ? "Saving…" : "Save Stream"}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- ── Job Adjustment / Raise Modal ─────────────────────────────────────────── -->
{#if showJobModal === "adjust" && adjustSourceJob}
  <div class="fixed inset-0 bg-black/60 dark:bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-4">
    <div class="card p-6 max-w-md w-full shadow-2xl">
      <div class="flex items-center justify-between mb-3 border-b border-neutral-200 dark:border-neutral-800 pb-3">
        <h3 class="text-base font-semibold text-neutral-900 dark:text-neutral-100">Adjust Rate / Promotion</h3>
        <button on:click={closeJobModal} class="text-neutral-400 hover:text-neutral-700 dark:hover:text-white text-lg">✕</button>
      </div>

      <p class="text-xs text-neutral-600 dark:text-neutral-400 mb-4">
        Updating <strong>{adjustSourceJob.name}</strong> ({adjustSourceJob.who}). This closes the previous rate on the day before the effective date and starts the new rate automatically.
      </p>

      <div class="space-y-3.5">
        <div>
          <label for="adj-date" class="block text-xs font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-1">Effective Start Date</label>
          <input
            id="adj-date"
            type="date"
            bind:value={adjustEffectiveDate}
            class="input-field"
          />
        </div>

        <div>
          <label for="adj-amount" class="block text-xs font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-1">
            New Pay Rate ({$currencySymbol} / {adjustSourceJob.frequency})
          </label>
          <input
            id="adj-amount"
            type="number"
            min="0.01"
            step="0.01"
            bind:value={adjustNewAmount}
            placeholder="0.00"
            class="input-field tabular-nums"
          />
        </div>

        <div>
          <label for="adj-notes" class="block text-xs font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-1">Reason / Note</label>
          <input
            id="adj-notes"
            type="text"
            bind:value={adjustNewNotes}
            placeholder="e.g. Mid-year raise, Promotion to Lead, Sickness 80%"
            class="input-field"
          />
        </div>

        {#if adjustError}
          <p class="text-xs text-rose-700 dark:text-red-400 bg-rose-50 dark:bg-red-950/40 border border-rose-200 dark:border-red-800 rounded-xl px-3 py-2">{adjustError}</p>
        {/if}

        <div class="flex items-center justify-end gap-2 pt-2">
          <button
            type="button"
            on:click={closeJobModal}
            class="btn-secondary"
          >
            Cancel
          </button>
          <button
            type="button"
            on:click={handleSaveAdjustment}
            disabled={adjustSaving}
            class="btn-primary"
          >
            {adjustSaving ? "Applying…" : "Apply Adjustment"}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- ── Salary Override Modal (Period-Specific Adjustment) ────────────────────────── -->
{#if showSalaryOverrideModal && salaryOverrideUser}
  <div class="fixed inset-0 bg-black/60 dark:bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-4">
    <div class="card p-6 max-w-md w-full shadow-2xl">
      <div class="flex items-center justify-between mb-3 border-b border-neutral-200 dark:border-neutral-800 pb-3">
        <div class="flex items-center gap-2">
          <div
            class="w-6 h-6 rounded-full flex items-center justify-center text-[10px] font-bold text-white shrink-0"
            style="background-color: {salaryOverrideUser.color}"
          >
            {userInitial(salaryOverrideUser.name)}
          </div>
          <h3 class="text-base font-semibold text-neutral-900 dark:text-neutral-100">
            Adjust Salary for {$selectedMonth}
          </h3>
        </div>
        <button on:click={closeSalaryOverrideModal} class="text-neutral-400 hover:text-neutral-700 dark:hover:text-white text-lg">✕</button>
      </div>

      <div class="mb-4 p-3 rounded-xl bg-neutral-50 dark:bg-neutral-900/60 border border-neutral-200 dark:border-neutral-800 text-xs">
        <div class="flex items-center justify-between mb-1">
          <span class="text-neutral-500">Person:</span>
          <span class="font-semibold text-neutral-800 dark:text-neutral-200">{salaryOverrideUser.name}</span>
        </div>
        <div class="flex items-center justify-between mb-1">
          <span class="text-neutral-500">Contract Base Rate:</span>
          <span class="font-mono font-medium text-neutral-700 dark:text-neutral-300">{fmt(salaryOverrideContractCents)} / month</span>
        </div>
        <div class="flex items-center justify-between">
          <span class="text-neutral-500">Target Month:</span>
          <span class="badge-indigo">{$selectedMonth}</span>
        </div>
        <p class="text-[11px] text-neutral-500 dark:text-neutral-400 mt-2 border-t border-neutral-200 dark:border-neutral-800 pt-1.5 leading-relaxed">
          This adjustment applies <strong>only to {$selectedMonth}</strong> (e.g. for sickness, overtime, parental leave). Other months continue using the contract rate.
        </p>
      </div>

      <div class="space-y-3.5">
        <div>
          <label for="override-amount" class="block text-xs font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-1">
            Actual Pay for {$selectedMonth} ({$currencySymbol})
          </label>
          <input
            id="override-amount"
            type="number"
            min="0"
            step="0.01"
            bind:value={salaryOverrideAmount}
            placeholder="0.00"
            class="input-field tabular-nums"
          />
        </div>

        <div>
          <label for="override-note" class="block text-xs font-semibold text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-1">
            Reason / Note (optional)
          </label>
          <input
            id="override-note"
            type="text"
            bind:value={salaryOverrideNote}
            placeholder="e.g. 10 days unpaid leave, overtime, sickness"
            class="input-field"
          />
        </div>

        {#if salaryOverrideError}
          <p class="text-xs text-rose-700 dark:text-red-400 bg-rose-50 dark:bg-red-950/40 border border-rose-200 dark:border-red-800 rounded-xl px-3 py-2">{salaryOverrideError}</p>
        {/if}

        <div class="flex items-center justify-between gap-2 pt-2 border-t border-neutral-200 dark:border-neutral-800">
          <div>
            {#if salaryOverrideHasExisting}
              <button
                id="btn-reset-salary-override"
                type="button"
                on:click={handleResetSalaryOverride}
                disabled={salaryOverrideSaving}
                class="px-3 py-1.5 rounded-xl text-xs font-medium text-rose-600 dark:text-rose-400 hover:bg-rose-50 dark:hover:bg-rose-950/40 border border-rose-200 dark:border-rose-900/50 transition-colors cursor-pointer"
              >
                Reset to Base
              </button>
            {/if}
          </div>

          <div class="flex items-center gap-2">
            <button
              type="button"
              on:click={closeSalaryOverrideModal}
              class="btn-secondary"
            >
              Cancel
            </button>
            <button
              id="btn-save-salary-override"
              type="button"
              on:click={handleSaveSalaryOverride}
              disabled={salaryOverrideSaving}
              class="btn-primary"
            >
              {salaryOverrideSaving ? "Saving…" : "Save Adjustment"}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}

