import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import SettingsTab from '../../lib/SettingsTab.svelte';
import {
  users,
  splits,
  projects,
  currencySymbol,
  defaultPayer,
  defaultCategory,
  defaultProject,
  splitInputMode,
  paybackDisplayMode,
  chartStyle,
  jointAccountEnabled,
  showProjectsInExpense,
  mobileTabVisibility,
  authSalt
} from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('SettingsTab.svelte — Central Settings & Preferences', () => {
  const dummyTabs = [
    { id: 'dashboard', label: 'Dashboard', icon: '<span>D</span>' },
    { id: 'expenses', label: 'Expenses', icon: '<span>E</span>' },
    { id: 'income', label: 'Income', icon: '<span>I</span>' },
    { id: 'splits', label: 'Categories', icon: '<span>C</span>' },
    { id: 'projects', label: 'Projects', icon: '<span>P</span>' },
    { id: 'tags', label: 'Tags', icon: '<span>T</span>' },
    { id: 'recurring', label: 'Recurring', icon: '<span>R</span>' },
    { id: 'query', label: 'Query', icon: '<span>Q</span>' },
    { id: 'joint', label: 'Joint Account', icon: '<span>J</span>' },
    { id: 'settings', label: 'Settings', icon: '<span>S</span>' },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
    authSalt.set('test-salt');
    users.set([
      { name: 'John', color: '#6366f1', is_active: true },
      { name: 'Jane', color: '#ec4899', is_active: true },
    ]);
    splits.set([{ category: 'GROCERIES' }, { category: 'RENT' }]);
    projects.set([{ id: 1, name: 'Renovation' }]);
    currencySymbol.set('€');
    defaultPayer.set('');
    defaultCategory.set('');
    defaultProject.set('');
    splitInputMode.set('inputs');
    paybackDisplayMode.set('cards');
    chartStyle.set('doughnut');
    jointAccountEnabled.set(false);
    showProjectsInExpense.set(true);
    mobileTabVisibility.set({
      dashboard: true,
      expenses: true,
      income: true,
      splits: true,
      projects: true,
      tags: true,
      recurring: true,
      query: true,
      settings: true,
      joint: true,
    });
  });

  it.each([
    { section: 'Household Members' },
  ])('renders settings sections properly ($section)', ({ section }) => {
    render(SettingsTab, { props: { tabs: dummyTabs } });

    expect(screen.getAllByText(/Household Members/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Feature Modules/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Navigation Tabs Customization/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Form & Entry Defaults/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Visualizations & Controls/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Zero-Knowledge Security/i).length).toBeGreaterThan(0);
  });

  it.each([
    { preset: '$' },
  ])('updates currency symbol via quick preset buttons ($preset)', async ({ preset }) => {
    render(SettingsTab, { props: { tabs: dummyTabs } });

    const dollarBtn = screen.getByRole('button', { name: preset });
    await fireEvent.click(dollarBtn);

    let currentVal = '';
    currencySymbol.subscribe((v) => { currentVal = v; })();
    expect(currentVal).toBe(preset);
  });

  it.each([
    { tabId: 'expenses' },
  ])('toggles tab visibility and protects required tabs ($tabId)', async ({ tabId }) => {
    render(SettingsTab, { props: { tabs: dummyTabs } });

    const expenseToggle = document.getElementById(`toggle-tab-${tabId}`);
    expect(expenseToggle).toBeInTheDocument();
    await fireEvent.click(expenseToggle);

    let currentVis = {};
    mobileTabVisibility.subscribe((v) => { currentVis = v; })();
    expect(currentVis[tabId]).toBe(false);

    // Try toggling required settings tab
    const settingsToggle = document.getElementById('toggle-tab-settings');
    expect(settingsToggle).toBeDisabled();
  });

  it.each([
    { initial: false },
  ])('toggles joint account and fires prompt callback ($initial)', async () => {
    let promptCalled = false;
    render(SettingsTab, {
      props: {
        tabs: dummyTabs,
        onToggleJointPrompt: () => { promptCalled = true; },
      },
    });

    const jointToggle = document.getElementById('toggle-joint-account-enabled');
    await fireEvent.click(jointToggle);

    let enabledVal = false;
    jointAccountEnabled.subscribe((v) => { enabledVal = v; })();
    expect(enabledVal).toBe(true);
    expect(promptCalled).toBe(true);
  });

  it.each([
    { exportLabel: 'Export .db File' },
  ])('handles database export action ($exportLabel)', async () => {
    const exportSpy = vi.spyOn(api, 'exportDatabase').mockResolvedValue({});

    render(SettingsTab, { props: { tabs: dummyTabs } });

    const exportBtn = document.getElementById('export-db-btn');
    expect(exportBtn).toBeInTheDocument();
    await fireEvent.click(exportBtn);

    expect(exportSpy).toHaveBeenCalledWith('test-salt');
  });
});
