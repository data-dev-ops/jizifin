/**
 * stores.js — server responses and UI state. No derived money math: every
 * total, share, target and status is computed by the backend.
 *
 * Catalog stores hold decrypted rows (labels in place of ciphertext) and are
 * refreshed by api.js after writes and by live.js on WebSocket events.
 * `ledgerVersion` ticks on every server change so tabs refetch their views.
 */

import { writable, derived } from 'svelte/store';

/**
 * Helper: writable store that reads/writes a boolean to localStorage.
 * Falls back to `defaultValue` when no entry exists yet.
 */
function persistedBoolean(key, defaultValue) {
  const stored = typeof localStorage !== 'undefined' ? localStorage.getItem(key) : null;
  const initial = stored !== null ? stored === 'true' : defaultValue;
  const store = writable(initial);
  if (typeof localStorage !== 'undefined') {
    store.subscribe((val) => localStorage.setItem(key, String(val)));
  }
  return store;
}

/**
 * Helper: writable store that reads/writes a JSON object to localStorage.
 */
function persistedObject(key, defaultValue) {
  const stored = typeof localStorage !== 'undefined' ? localStorage.getItem(key) : null;
  let initial = defaultValue;
  if (stored !== null) {
    try {
      initial = { ...defaultValue, ...JSON.parse(stored) };
    } catch {}
  }
  const store = writable(initial);
  if (typeof localStorage !== 'undefined') {
    store.subscribe((val) => localStorage.setItem(key, JSON.stringify(val)));
  }
  return store;
}

/**
 * Helper: writable store that reads/writes a string to localStorage.
 */
function persistedString(key, defaultValue) {
  const stored = typeof localStorage !== 'undefined' ? localStorage.getItem(key) : null;
  const initial = stored !== null ? stored : defaultValue;
  const store = writable(initial);
  if (typeof localStorage !== 'undefined') {
    store.subscribe((val) => localStorage.setItem(key, val));
  }
  return store;
}

export const members = writable([]);
export const categories = writable([]);
export const accounts = writable([]);
export const projects = writable([]);
export const tags = writable([]);
export const jobs = writable([]);
export const recurring = writable([]);
export const budgets = writable([]);
export const offsetRules = writable([]);
/** Sealed months: `{ month, result_json, inputs_hash, sealed_at }`. */
export const seals = writable([]);

/** Bumped on every server-side change; views depend on it to refetch. */
export const ledgerVersion = writable(0);

function byId(store) {
  return derived(store, ($rows) => new Map($rows.map((row) => [row.id, row])));
}

export const memberById = byId(members);
export const categoryById = byId(categories);
export const accountById = byId(accounts);
export const projectById = byId(projects);
export const tagById = byId(tags);

export const activeMembers = derived(members, ($m) => $m.filter((m) => m.is_active));
export const expenseCategories = derived(categories, ($c) => $c.filter((c) => c.kind === 'expense'));
export const incomeCategories = derived(categories, ($c) => $c.filter((c) => c.kind === 'income'));

/** Label for member `id` (decrypted name), or `#id` when unknown. */
export const memberName = derived(memberById, ($m) => (id) => $m.get(id)?.name ?? (id == null ? '' : `#${id}`));
export const categoryName = derived(categoryById, ($c) => (id) => $c.get(id)?.name ?? (id == null ? '' : `#${id}`));

/** Current month in YYYY-MM format, shared across tabs. */
const now = new Date();
export const selectedMonth = writable(`${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`);

/**
 * Dashboard scope: 'ALL', or comma-separated 'MEMBER:<id>' / 'ACCOUNT:<id>' tokens.
 */
export const dashboardScope = persistedString('dashboardScope', 'ALL');

/** Scope string to tokens; `['ALL']` when empty. */
export function parseDashboardScope(scope) {
  const parts = (scope || 'ALL').split(',').map((s) => s.trim()).filter(Boolean);
  return parts.length === 0 || parts.includes('ALL') ? ['ALL'] : parts;
}

/** Member ids selected by `tokens`, or null for the whole household. */
export function resolveScopedMembers(tokens, accountRows) {
  if (tokens.includes('ALL')) return null;
  const ids = new Set();
  for (const token of tokens) {
    const [kind, raw] = token.split(':');
    const id = Number(raw);
    if (kind === 'MEMBER') ids.add(id);
    if (kind === 'ACCOUNT') accountRows.find((a) => a.id === id)?.members.forEach((m) => ids.add(m.member));
  }
  return ids.size > 0 ? [...ids] : null;
}

/** Active scope as member ids, or null for everyone. */
export const scopedMembers = derived([dashboardScope, accounts], ([$scope, $accounts]) =>
  resolveScopedMembers(parseDashboardScope($scope), $accounts));

/** Joint account selected in the Joint tab. */
export const activeAccountId = writable(null);

/**
 * UI preference: Show project selector dropdown in ExpenseForm.
 * Default true when projects exist; user can toggle off in Projects tab or Settings.
 */
export const showProjectsInExpense = persistedBoolean('showProjectsInExpense', true);

/**
 * UI preference: Joint account feature enabled.
 * Default false — opt-in module for multi-member households.
 */
export const jointAccountEnabled = persistedBoolean('jointAccountEnabled', false);

/**
 * Mobile preferences & tab visibility stores.
 * Maps tab ID to boolean visibility on mobile devices.
 * Settings and at least one other tab are always enforced active.
 */
export const mobileTabVisibility = persistedObject('mobileTabVisibility', {
  dashboard: true,
  expenses: true,
  income: true,
  splits: true,
  budgets: true,
  projects: true,
  tags: true,
  recurring: true,
  query: true,
  settings: true,
  joint: true,
});

export const mobileAutoCloseMenu = persistedBoolean('mobileAutoCloseMenu', true);
export const mobileCompactView = persistedBoolean('mobileCompactView', false);
export const mobileLargeTouchTargets = persistedBoolean('mobileLargeTouchTargets', false);

export const defaultPayer = persistedString('defaultPayer', '');
export const defaultCategory = persistedString('defaultCategory', '');
export const defaultProject = persistedString('defaultProject', '');
export const showQueryTab = persistedBoolean('showQueryTab', true);
export const currencySymbol = persistedString('currencySymbol', '€');

/**
 * UI display mode: how split percentages are entered.
 * 'inputs' — individual number fields per user (default, works for any user count).
 * 'slider' — single linked range slider when exactly 2 active users; drag one side,
 *            the other auto-complements to 100%. Falls back to inputs if >2 users.
 */
export const splitInputMode = persistedString('splitInputMode', 'inputs');

/**
 * UI display mode: how per-category payback breakdown is rendered.
 * 'cards' — current grid of per-user cards showing paid/share/net (default).
 * 'bar'   — horizontal stacked bars per category, segments proportional to amount paid.
 */
export const paybackDisplayMode = persistedString('paybackDisplayMode', 'cards');

/**
 * UI display mode: category spending chart type on the dashboard.
 * 'doughnut' — current doughnut/donut chart (default).
 * 'bar'      — horizontal bar chart with categories on the Y axis.
 */
export const chartStyle = persistedString('chartStyle', 'doughnut');

/**
 * Theme mode preference: 'dark' | 'light' | 'system'
 * Defaults to 'dark' to preserve the signature aesthetic while supporting crisp light mode.
 */
export const theme = persistedString('theme', 'dark');

export const cryptoKey = writable(null);

const storedSession = typeof sessionStorage !== 'undefined' ? sessionStorage.getItem('jizifin_session') : '';
export const sessionToken = writable(storedSession || '');
if (typeof sessionStorage !== 'undefined') {
  sessionToken.subscribe((val) => {
    if (val) {
      sessionStorage.setItem('jizifin_session', val);
    } else {
      sessionStorage.removeItem('jizifin_session');
    }
  });
}

/**
 * ── Multi-Generational Household Customization & Persona Tiers ───────────────
 */

export const experienceTier = persistedString('experienceTier', 'standard'); // 'standard' | 'teen' | 'senior' | 'auditor'
export const privacyShield = persistedBoolean('privacyShield', false);
export const currencyPrecisionMode = persistedString('currencyPrecisionMode', 'whole_units_on_summaries');
export const projectDisplayFilter = persistedString('projectDisplayFilter', 'active'); // 'active' | 'in_progress' | 'all'
export const expenseRowDensity = persistedString('expenseRowDensity', 'compact'); // 'minimal' | 'compact' | 'detailed'
export const formMemoryMode = persistedString('formMemoryMode', 'remember_last'); // 'remember_last' | 'static_preset' | 'empty'
export const lastLoggedPayer = persistedString('lastLoggedPayer', '');
export const lastLoggedCategory = persistedString('lastLoggedCategory', '');

export const enabledFormFields = persistedObject('enabledFormFields', {
  projects: true,
  tags: true,
  joint: true,
  splitOverride: true,
});

export const dashboardWidgets = persistedObject('dashboardWidgets', {
  monthlyTotal: true,
  payerBreakdown: true,
  categoryChart: true,
  cashFlowSavings: true,
  tagBreakdown: false,
  recentExpenses: true,
});

export const textScale = persistedString('textScale', '100'); // '100' | '115' | '130'
export const highContrast = persistedBoolean('highContrast', false);

/**
 * ── Device-Aware Profiles Engine ─────────────────────────────────────────────
 * Provides distinct configurations for Desktop Mode vs. Mobile Mode,
 * loaded automatically based on client device and manually configurable.
 */

export const DEFAULT_DESKTOP_PROFILE = {
  tabVisibility: {
    dashboard: true,
    expenses: true,
    income: true,
    splits: true,
    budgets: true,
    projects: true,
    tags: true,
    recurring: true,
    query: true,
    settings: true,
    joint: true,
  },
  compactView: false,
  largeTouchTargets: false,
  autoCloseMenu: false,
  chartStyle: 'doughnut',
  splitInputMode: 'inputs',
  paybackDisplayMode: 'cards',
  theme: 'dark',
  showProjectsInExpense: true,
  defaultPayer: '',
  defaultCategory: '',
  defaultProject: '',
  currencySymbol: '€',
  experienceTier: 'standard',
  privacyShield: false,
  currencyPrecisionMode: 'whole_units_on_summaries',
  projectDisplayFilter: 'active',
  expenseRowDensity: 'compact',
  formMemoryMode: 'remember_last',
  enabledFormFields: { projects: true, tags: true, joint: true, splitOverride: true },
  dashboardWidgets: { monthlyTotal: true, payerBreakdown: true, categoryChart: true, cashFlowSavings: true, tagBreakdown: false, recentExpenses: true },
  textScale: '100',
  highContrast: false,
};

export const DEFAULT_MOBILE_PROFILE = {
  tabVisibility: {
    dashboard: true,
    expenses: true,
    income: true,
    splits: true,
    budgets: false,
    projects: false,
    tags: false,
    recurring: false,
    query: false,
    settings: true,
    joint: true,
  },
  compactView: true,
  largeTouchTargets: true,
  autoCloseMenu: true,
  chartStyle: 'doughnut',
  splitInputMode: 'slider',
  paybackDisplayMode: 'cards',
  theme: 'dark',
  showProjectsInExpense: true,
  defaultPayer: '',
  defaultCategory: '',
  defaultProject: '',
  currencySymbol: '€',
  experienceTier: 'standard',
  privacyShield: false,
  currencyPrecisionMode: 'whole_units_on_summaries',
  projectDisplayFilter: 'active',
  expenseRowDensity: 'compact',
  formMemoryMode: 'remember_last',
  enabledFormFields: { projects: true, tags: false, joint: true, splitOverride: true },
  dashboardWidgets: { monthlyTotal: true, payerBreakdown: true, categoryChart: true, cashFlowSavings: true, tagBreakdown: false, recentExpenses: true },
  textScale: '100',
  highContrast: false,
};

export const detectedDeviceType = writable('desktop');
export const activeDeviceMode = persistedString('activeDeviceMode', 'desktop');

export function detectDeviceType() {
  if (typeof window === 'undefined') return 'desktop';
  const isSmallScreen = window.innerWidth < 768;
  const isTouch = window.matchMedia && window.matchMedia('(pointer: coarse)').matches;
  return (isSmallScreen || isTouch) ? 'mobile' : 'desktop';
}

export function getDeviceProfile(mode) {
  const defaultProfile = mode === 'mobile' ? DEFAULT_MOBILE_PROFILE : DEFAULT_DESKTOP_PROFILE;
  if (typeof localStorage === 'undefined') return { ...defaultProfile };
  const raw = localStorage.getItem(`jizifin_profile_${mode}`);
  if (!raw) return { ...defaultProfile };
  try {
    const parsed = JSON.parse(raw);
    return {
      ...defaultProfile,
      ...parsed,
      tabVisibility: {
        ...defaultProfile.tabVisibility,
        ...(parsed.tabVisibility || {}),
        settings: true,
        dashboard: true,
      },
      enabledFormFields: {
        ...defaultProfile.enabledFormFields,
        ...(parsed.enabledFormFields || {}),
      },
      dashboardWidgets: {
        ...defaultProfile.dashboardWidgets,
        ...(parsed.dashboardWidgets || {}),
      },
    };
  } catch {
    return { ...defaultProfile };
  }
}

export function saveDeviceProfile(mode, profile) {
  if (typeof localStorage === 'undefined') return;
  localStorage.setItem(`jizifin_profile_${mode}`, JSON.stringify(profile));
}

export function getSnapshotCurrentSettings() {
  let currentTabs = {};
  mobileTabVisibility.subscribe((v) => { currentTabs = v; })();
  let currentCompact = false;
  mobileCompactView.subscribe((v) => { currentCompact = v; })();
  let currentTouch = false;
  mobileLargeTouchTargets.subscribe((v) => { currentTouch = v; })();
  let currentAutoClose = true;
  mobileAutoCloseMenu.subscribe((v) => { currentAutoClose = v; })();
  let currentChart = 'doughnut';
  chartStyle.subscribe((v) => { currentChart = v; })();
  let currentSplit = 'inputs';
  splitInputMode.subscribe((v) => { currentSplit = v; })();
  let currentPayback = 'cards';
  paybackDisplayMode.subscribe((v) => { currentPayback = v; })();
  let currentTheme = 'dark';
  theme.subscribe((v) => { currentTheme = v; })();
  let currentProjExp = true;
  showProjectsInExpense.subscribe((v) => { currentProjExp = v; })();
  let currentPayer = '';
  defaultPayer.subscribe((v) => { currentPayer = v; })();
  let currentCat = '';
  defaultCategory.subscribe((v) => { currentCat = v; })();
  let currentProj = '';
  defaultProject.subscribe((v) => { currentProj = v; })();
  let currentCurr = '€';
  currencySymbol.subscribe((v) => { currentCurr = v; })();

  let currentTier = 'standard';
  experienceTier.subscribe((v) => { currentTier = v; })();
  let currentPrivacy = false;
  privacyShield.subscribe((v) => { currentPrivacy = v; })();
  let currentPrecision = 'whole_units_on_summaries';
  currencyPrecisionMode.subscribe((v) => { currentPrecision = v; })();
  let currentProjFilter = 'active';
  projectDisplayFilter.subscribe((v) => { currentProjFilter = v; })();
  let currentDensity = 'compact';
  expenseRowDensity.subscribe((v) => { currentDensity = v; })();
  let currentFormMem = 'remember_last';
  formMemoryMode.subscribe((v) => { currentFormMem = v; })();
  let currentFormFields = { projects: true, tags: true, joint: true, splitOverride: true };
  enabledFormFields.subscribe((v) => { currentFormFields = v; })();
  let currentWidgets = { monthlyTotal: true, payerBreakdown: true, categoryChart: true, cashFlowSavings: true, tagBreakdown: false, recentExpenses: true };
  dashboardWidgets.subscribe((v) => { currentWidgets = v; })();
  let currentTextScale = '100';
  textScale.subscribe((v) => { currentTextScale = v; })();
  let currentHighContrast = false;
  highContrast.subscribe((v) => { currentHighContrast = v; })();

  return {
    tabVisibility: { ...currentTabs },
    compactView: currentCompact,
    largeTouchTargets: currentTouch,
    autoCloseMenu: currentAutoClose,
    chartStyle: currentChart,
    splitInputMode: currentSplit,
    paybackDisplayMode: currentPayback,
    theme: currentTheme,
    showProjectsInExpense: currentProjExp,
    defaultPayer: currentPayer,
    defaultCategory: currentCat,
    defaultProject: currentProj,
    currencySymbol: currentCurr,
    experienceTier: currentTier,
    privacyShield: currentPrivacy,
    currencyPrecisionMode: currentPrecision,
    projectDisplayFilter: currentProjFilter,
    expenseRowDensity: currentDensity,
    formMemoryMode: currentFormMem,
    enabledFormFields: { ...currentFormFields },
    dashboardWidgets: { ...currentWidgets },
    textScale: currentTextScale,
    highContrast: currentHighContrast,
  };
}

export function applySettingsSnapshot(snapshot) {
  if (!snapshot) return;
  if (snapshot.tabVisibility) {
    mobileTabVisibility.set({
      ...snapshot.tabVisibility,
      settings: true,
      dashboard: true,
    });
  }
  if (typeof snapshot.compactView === 'boolean') mobileCompactView.set(snapshot.compactView);
  if (typeof snapshot.largeTouchTargets === 'boolean') mobileLargeTouchTargets.set(snapshot.largeTouchTargets);
  if (typeof snapshot.autoCloseMenu === 'boolean') mobileAutoCloseMenu.set(snapshot.autoCloseMenu);
  if (snapshot.chartStyle) chartStyle.set(snapshot.chartStyle);
  if (snapshot.splitInputMode) splitInputMode.set(snapshot.splitInputMode);
  if (snapshot.paybackDisplayMode) paybackDisplayMode.set(snapshot.paybackDisplayMode);
  if (snapshot.theme) theme.set(snapshot.theme);
  if (typeof snapshot.showProjectsInExpense === 'boolean') showProjectsInExpense.set(snapshot.showProjectsInExpense);
  if (typeof snapshot.defaultPayer === 'string') defaultPayer.set(snapshot.defaultPayer);
  if (typeof snapshot.defaultCategory === 'string') defaultCategory.set(snapshot.defaultCategory);
  if (typeof snapshot.defaultProject === 'string') defaultProject.set(snapshot.defaultProject);
  if (typeof snapshot.currencySymbol === 'string') currencySymbol.set(snapshot.currencySymbol);

  if (snapshot.experienceTier) experienceTier.set(snapshot.experienceTier);
  if (typeof snapshot.privacyShield === 'boolean') privacyShield.set(snapshot.privacyShield);
  if (snapshot.currencyPrecisionMode) currencyPrecisionMode.set(snapshot.currencyPrecisionMode);
  if (snapshot.projectDisplayFilter) projectDisplayFilter.set(snapshot.projectDisplayFilter);
  if (snapshot.expenseRowDensity) expenseRowDensity.set(snapshot.expenseRowDensity);
  if (snapshot.formMemoryMode) formMemoryMode.set(snapshot.formMemoryMode);
  if (snapshot.enabledFormFields) enabledFormFields.set({ ...snapshot.enabledFormFields });
  if (snapshot.dashboardWidgets) dashboardWidgets.set({ ...snapshot.dashboardWidgets });
  if (snapshot.textScale) textScale.set(snapshot.textScale);
  if (typeof snapshot.highContrast === 'boolean') highContrast.set(snapshot.highContrast);
}

export function applyExperienceTier(tier) {
  let normalized = tier;
  if (tier === 'teen') normalized = 'focused';
  if (tier === 'senior') normalized = 'legibility';
  if (tier === 'auditor') normalized = 'detailed';

  experienceTier.set(normalized);

  if (normalized === 'focused') {
    textScale.set('100');
    highContrast.set(false);
    currencyPrecisionMode.set('whole_units_on_summaries');
    expenseRowDensity.set('minimal');
    formMemoryMode.set('remember_last');
    projectDisplayFilter.set('active');
    enabledFormFields.set({ projects: false, tags: false, joint: false, splitOverride: false });
    dashboardWidgets.set({ monthlyTotal: true, payerBreakdown: false, categoryChart: true, cashFlowSavings: false, tagBreakdown: false, recentExpenses: true });
    mobileTabVisibility.set({
      dashboard: true,
      expenses: true,
      income: false,
      splits: false,
      budgets: false,
      projects: false,
      tags: false,
      recurring: false,
      query: false,
      settings: true,
      joint: false,
    });
  } else if (normalized === 'legibility') {
    textScale.set('115');
    highContrast.set(true);
    currencyPrecisionMode.set('whole_units_on_summaries');
    expenseRowDensity.set('compact');
    formMemoryMode.set('remember_last');
    projectDisplayFilter.set('active');
    enabledFormFields.set({ projects: false, tags: false, joint: true, splitOverride: false });
    dashboardWidgets.set({ monthlyTotal: true, payerBreakdown: true, categoryChart: true, cashFlowSavings: false, tagBreakdown: false, recentExpenses: true });
    mobileTabVisibility.set({
      dashboard: true,
      expenses: true,
      income: false,
      splits: false,
      budgets: false,
      projects: false,
      tags: false,
      recurring: false,
      query: false,
      settings: true,
      joint: true,
    });
  } else if (normalized === 'detailed') {
    textScale.set('100');
    highContrast.set(false);
    currencyPrecisionMode.set('always_exact');
    expenseRowDensity.set('detailed');
    formMemoryMode.set('static_preset');
    projectDisplayFilter.set('all');
    enabledFormFields.set({ projects: true, tags: true, joint: true, splitOverride: true });
    dashboardWidgets.set({ monthlyTotal: true, payerBreakdown: true, categoryChart: true, cashFlowSavings: true, tagBreakdown: true, recentExpenses: true });
    mobileTabVisibility.set({
      dashboard: true,
      expenses: true,
      income: true,
      splits: true,
      budgets: true,
      projects: true,
      tags: true,
      recurring: true,
      query: true,
      settings: true,
      joint: true,
    });
  } else {
    // standard / balanced default
    textScale.set('100');
    highContrast.set(false);
    currencyPrecisionMode.set('whole_units_on_summaries');
    expenseRowDensity.set('compact');
    formMemoryMode.set('remember_last');
    projectDisplayFilter.set('active');
    enabledFormFields.set({ projects: true, tags: true, joint: true, splitOverride: true });
    dashboardWidgets.set({ monthlyTotal: true, payerBreakdown: true, categoryChart: true, cashFlowSavings: true, tagBreakdown: false, recentExpenses: true });
    mobileTabVisibility.set({
      dashboard: true,
      expenses: true,
      income: true,
      splits: true,
      budgets: true,
      projects: true,
      tags: true,
      recurring: true,
      query: false,
      settings: true,
      joint: true,
    });
  }
}

export function loadDeviceProfile(mode) {
  const profile = getDeviceProfile(mode);
  applySettingsSnapshot(profile);
  activeDeviceMode.set(mode);
  return profile;
}

export function saveCurrentToProfile(mode) {
  const snapshot = getSnapshotCurrentSettings();
  saveDeviceProfile(mode, snapshot);
  activeDeviceMode.set(mode);
  return snapshot;
}

export function copyProfile(fromMode, toMode) {
  const source = getDeviceProfile(fromMode);
  saveDeviceProfile(toMode, source);
  return source;
}

export function resetProfileToDefaults(mode) {
  const defaultProfile = mode === 'mobile' ? DEFAULT_MOBILE_PROFILE : DEFAULT_DESKTOP_PROFILE;
  saveDeviceProfile(mode, defaultProfile);
  let currentActive = 'desktop';
  activeDeviceMode.subscribe((v) => { currentActive = v; })();
  if (currentActive === mode) {
    applySettingsSnapshot(defaultProfile);
  }
  return defaultProfile;
}

export function initDeviceProfiles() {
  const detected = detectDeviceType();
  detectedDeviceType.set(detected);
  
  // If no saved profiles exist yet, initialize them with defaults
  if (typeof localStorage !== 'undefined') {
    if (!localStorage.getItem('jizifin_profile_desktop')) {
      saveDeviceProfile('desktop', DEFAULT_DESKTOP_PROFILE);
    }
    if (!localStorage.getItem('jizifin_profile_mobile')) {
      saveDeviceProfile('mobile', DEFAULT_MOBILE_PROFILE);
    }
  }
  
  loadDeviceProfile(detected);
}

/**
 * Active dashboard drilldown target.
 * Structure: { type: 'category' | 'category-adjustment' | 'payer' | 'budget' | 'project' | 'joint-spent' | 'income' | 'monthly-total', title: string, data?: any } | null
 */
export const drilldownTarget = writable(null);

export function openDrilldown(type, title, data = {}) {
  drilldownTarget.set({ type, title, data });
}

export function closeDrilldown() {
  drilldownTarget.set(null);
}
