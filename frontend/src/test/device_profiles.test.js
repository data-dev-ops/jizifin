import { describe, it, expect, beforeEach } from 'vitest';
import {
  DEFAULT_DESKTOP_PROFILE,
  DEFAULT_MOBILE_PROFILE,
  activeDeviceMode,
  detectedDeviceType,
  detectDeviceType,
  getDeviceProfile,
  saveDeviceProfile,
  getSnapshotCurrentSettings,
  applySettingsSnapshot,
  loadDeviceProfile,
  saveCurrentToProfile,
  copyProfile,
  resetProfileToDefaults,
  initDeviceProfiles,
  mobileCompactView,
  mobileLargeTouchTargets,
  mobileAutoCloseMenu,
  chartStyle,
  splitInputMode,
  paybackDisplayMode,
  currencySymbol
} from '../lib/stores.js';

describe('Device-Aware Settings Profiles Engine', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('loads default profiles when no custom saved profiles exist', () => {
    const desktopProfile = getDeviceProfile('desktop');
    expect(desktopProfile.compactView).toBe(DEFAULT_DESKTOP_PROFILE.compactView);
    expect(desktopProfile.tabVisibility.dashboard).toBe(true);

    const mobileProfile = getDeviceProfile('mobile');
    expect(mobileProfile.compactView).toBe(DEFAULT_MOBILE_PROFILE.compactView);
    expect(mobileProfile.largeTouchTargets).toBe(true);
  });

  it('saves and reloads custom device profile to localStorage', () => {
    const customMobile = {
      ...DEFAULT_MOBILE_PROFILE,
      compactView: false,
      currencySymbol: '$',
      chartStyle: 'bar',
    };

    saveDeviceProfile('mobile', customMobile);
    const loaded = getDeviceProfile('mobile');

    expect(loaded.compactView).toBe(false);
    expect(loaded.currencySymbol).toBe('$');
    expect(loaded.chartStyle).toBe('bar');
  });

  it('snapshots current store settings and saves to target profile', () => {
    mobileCompactView.set(true);
    mobileLargeTouchTargets.set(true);
    mobileAutoCloseMenu.set(true);
    chartStyle.set('bar');
    splitInputMode.set('slider');
    paybackDisplayMode.set('bar');
    currencySymbol.set('£');

    const snapshot = saveCurrentToProfile('desktop');

    expect(snapshot.compactView).toBe(true);
    expect(snapshot.currencySymbol).toBe('£');
    expect(snapshot.chartStyle).toBe('bar');

    const stored = getDeviceProfile('desktop');
    expect(stored.currencySymbol).toBe('£');
    expect(stored.compactView).toBe(true);
  });

  it('applies loaded profile directly to reactive stores', () => {
    const customDesktop = {
      ...DEFAULT_DESKTOP_PROFILE,
      compactView: false,
      currencySymbol: 'CHF',
      chartStyle: 'bar',
      splitInputMode: 'inputs',
    };
    saveDeviceProfile('desktop', customDesktop);

    loadDeviceProfile('desktop');

    let curCompact = true;
    mobileCompactView.subscribe((v) => { curCompact = v; })();
    expect(curCompact).toBe(false);

    let curCurrency = '';
    currencySymbol.subscribe((v) => { curCurrency = v; })();
    expect(curCurrency).toBe('CHF');

    let curActiveMode = '';
    activeDeviceMode.subscribe((v) => { curActiveMode = v; })();
    expect(curActiveMode).toBe('desktop');
  });

  it('copies settings between desktop and mobile profiles', () => {
    const customDesktop = {
      ...DEFAULT_DESKTOP_PROFILE,
      currencySymbol: '¥',
      chartStyle: 'bar',
    };
    saveDeviceProfile('desktop', customDesktop);

    copyProfile('desktop', 'mobile');

    const mobileProfile = getDeviceProfile('mobile');
    expect(mobileProfile.currencySymbol).toBe('¥');
    expect(mobileProfile.chartStyle).toBe('bar');
  });

  it('resets profile to standard device defaults', () => {
    saveDeviceProfile('mobile', {
      ...DEFAULT_MOBILE_PROFILE,
      compactView: false,
      currencySymbol: 'XYZ',
    });

    resetProfileToDefaults('mobile');

    const resetMobile = getDeviceProfile('mobile');
    expect(resetMobile.compactView).toBe(DEFAULT_MOBILE_PROFILE.compactView);
    expect(resetMobile.currencySymbol).toBe('€');
  });

  it('initializes device profiles automatically on startup', () => {
    initDeviceProfiles();

    let detected = '';
    detectedDeviceType.subscribe((v) => { detected = v; })();
    expect(['desktop', 'mobile']).toContain(detected);

    expect(localStorage.getItem('jizifin_profile_desktop')).not.toBeNull();
    expect(localStorage.getItem('jizifin_profile_mobile')).not.toBeNull();
  });
});
