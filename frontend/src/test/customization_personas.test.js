import { describe, it, expect, beforeEach } from 'vitest';
import {
  experienceTier,
  privacyShield,
  currencyPrecisionMode,
  projectDisplayFilter,
  expenseRowDensity,
  formMemoryMode,
  enabledFormFields,
  dashboardWidgets,
  textScale,
  highContrast,
  mobileTabVisibility,
  applyExperienceTier,
  getSnapshotCurrentSettings,
  applySettingsSnapshot,
  saveCurrentToProfile,
  loadDeviceProfile,
  getDeviceProfile
} from '../lib/stores.js';
import { get } from 'svelte/store';

describe('Workflow & Display Presets Engine', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('applies Streamlined / Minimal preset correctly', () => {
    applyExperienceTier('focused');

    expect(get(experienceTier)).toBe('focused');
    expect(get(expenseRowDensity)).toBe('minimal');
    expect(get(projectDisplayFilter)).toBe('active');
    expect(get(currencyPrecisionMode)).toBe('whole_units_on_summaries');
    expect(get(formMemoryMode)).toBe('remember_last');

    const fields = get(enabledFormFields);
    expect(fields.projects).toBe(false);
    expect(fields.tags).toBe(false);
    expect(fields.joint).toBe(false);
    expect(fields.splitOverride).toBe(false);

    const widgets = get(dashboardWidgets);
    expect(widgets.cashFlowSavings).toBe(false);

    const tabs = get(mobileTabVisibility);
    expect(tabs.income).toBe(false);
    expect(tabs.budgets).toBe(false);
  });

  it('applies High Legibility preset correctly', () => {
    applyExperienceTier('legibility');

    expect(get(experienceTier)).toBe('legibility');
    expect(get(textScale)).toBe('115');
    expect(get(highContrast)).toBe(true);
    expect(get(expenseRowDensity)).toBe('compact');
    expect(get(currencyPrecisionMode)).toBe('whole_units_on_summaries');
    expect(get(projectDisplayFilter)).toBe('active');

    const fields = get(enabledFormFields);
    expect(fields.projects).toBe(false);
    expect(fields.tags).toBe(false);
    expect(fields.splitOverride).toBe(false);
  });

  it('applies Standard / Balanced preset and restores all settings from previously active mode', () => {
    // Start in focused (which hid tabs and scaled fields)
    applyExperienceTier('focused');
    expect(get(mobileTabVisibility).income).toBe(false);
    expect(get(expenseRowDensity)).toBe('minimal');

    // Switch to legibility (which activated 115% scale and high contrast)
    applyExperienceTier('legibility');
    expect(get(textScale)).toBe('115');
    expect(get(highContrast)).toBe(true);

    // Switch to standard — must cleanly restore all defaults without keeping prior mode settings
    applyExperienceTier('standard');

    expect(get(experienceTier)).toBe('standard');
    expect(get(textScale)).toBe('100');
    expect(get(highContrast)).toBe(false);
    expect(get(expenseRowDensity)).toBe('compact');
    expect(get(projectDisplayFilter)).toBe('active');
    expect(get(formMemoryMode)).toBe('remember_last');

    const fields = get(enabledFormFields);
    expect(fields.projects).toBe(true);
    expect(fields.tags).toBe(true);
    expect(fields.joint).toBe(true);
    expect(fields.splitOverride).toBe(true);

    const tabs = get(mobileTabVisibility);
    expect(tabs.dashboard).toBe(true);
    expect(tabs.expenses).toBe(true);
    expect(tabs.income).toBe(true);
    expect(tabs.splits).toBe(true);
    expect(tabs.budgets).toBe(true);
    expect(tabs.projects).toBe(true);
    expect(tabs.tags).toBe(true);
    expect(tabs.recurring).toBe(true);
    expect(tabs.settings).toBe(true);
    expect(tabs.joint).toBe(true);
  });

  it('applies Detailed / Power User preset correctly', () => {
    applyExperienceTier('detailed');

    expect(get(experienceTier)).toBe('detailed');
    expect(get(currencyPrecisionMode)).toBe('always_exact');
    expect(get(expenseRowDensity)).toBe('detailed');
    expect(get(projectDisplayFilter)).toBe('all');
    expect(get(formMemoryMode)).toBe('static_preset');

    const fields = get(enabledFormFields);
    expect(fields.projects).toBe(true);
    expect(fields.tags).toBe(true);
    expect(fields.joint).toBe(true);
    expect(fields.splitOverride).toBe(true);

    const widgets = get(dashboardWidgets);
    expect(widgets.monthlyTotal).toBe(true);
    expect(widgets.payerBreakdown).toBe(true);
    expect(widgets.categoryChart).toBe(true);
    expect(widgets.cashFlowSavings).toBe(true);
    expect(widgets.tagBreakdown).toBe(true);
  });

  it('persists privacy shield toggle across state', () => {
    privacyShield.set(true);
    expect(get(privacyShield)).toBe(true);

    privacyShield.set(false);
    expect(get(privacyShield)).toBe(false);
  });

  it('captures full customization state in device profile snapshots', () => {
    applyExperienceTier('legibility');
    privacyShield.set(true);

    const snapshot = getSnapshotCurrentSettings();
    expect(snapshot.experienceTier).toBe('legibility');
    expect(snapshot.textScale).toBe('115');
    expect(snapshot.highContrast).toBe(true);
    expect(snapshot.privacyShield).toBe(true);
    expect(snapshot.expenseRowDensity).toBe('compact');

    // Reset to detailed and restore snapshot
    applyExperienceTier('detailed');
    expect(get(experienceTier)).toBe('detailed');

    applySettingsSnapshot(snapshot);
    expect(get(experienceTier)).toBe('legibility');
    expect(get(textScale)).toBe('115');
    expect(get(highContrast)).toBe(true);
    expect(get(privacyShield)).toBe(true);
  });

  it('saves and restores workflow preset to named profile', () => {
    applyExperienceTier('focused');
    privacyShield.set(true);

    saveCurrentToProfile('mobile');

    const profile = getDeviceProfile('mobile');
    expect(profile.experienceTier).toBe('focused');
    expect(profile.privacyShield).toBe(true);
    expect(profile.expenseRowDensity).toBe('minimal');

    // Switch to desktop and apply legibility
    applyExperienceTier('legibility');
    privacyShield.set(false);
    saveCurrentToProfile('desktop');

    // Reload mobile profile
    loadDeviceProfile('mobile');
    expect(get(experienceTier)).toBe('focused');
    expect(get(privacyShield)).toBe(true);
    expect(get(expenseRowDensity)).toBe('minimal');
  });
});
