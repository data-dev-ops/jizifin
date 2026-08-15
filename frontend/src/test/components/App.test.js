import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import App from '../../App.svelte';
import { authSalt, cryptoKey } from '../../lib/stores.js';
import { deriveKey } from '../../lib/crypto.js';

describe('App.svelte — Shell & Tab Navigation', () => {
  beforeEach(() => {
    authSalt.set('');
    cryptoKey.set(null);
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      status: 200,
      ok: true,
      json: async () => ({ value: 'some-magic' }),
    }));
  });

  it.each([
    { title: 'Jizifin Finance' },
  ])('renders Login screen when app is unauthenticated ($title)', ({ title }) => {
    render(App);
    expect(screen.getByText(title)).toBeInTheDocument();
    expect(screen.getByLabelText(/Master Password/i)).toBeInTheDocument();
  });

  it.each([
    { pass: 'test-pass' },
  ])('renders application shell and navigation when authenticated ($pass)', async ({ pass }) => {
    const key = await deriveKey(pass);
    authSalt.set(pass);
    cryptoKey.set(key);

    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => [],
    }));

    render(App);

    expect(await screen.findByText('Dashboard')).toBeInTheDocument();
    expect(screen.getByText('Expenses')).toBeInTheDocument();
    expect(screen.getByText('Income')).toBeInTheDocument();
    expect(screen.getByText('Categories')).toBeInTheDocument();
    expect(screen.getByText('Settings')).toBeInTheDocument();
  });

  it.each([
    { tabId: 'nav-settings', expectedText: /Navigation Tabs Customization/i },
  ])('switches active tab when navigation item is clicked ($tabId)', async ({ tabId, expectedText }) => {
    const key = await deriveKey('test-pass');
    authSalt.set('test-pass');
    cryptoKey.set(key);

    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => [],
    }));

    render(App);

    const settingsTabBtn = document.getElementById(tabId);
    await fireEvent.click(settingsTabBtn);

    expect(await screen.findByText(expectedText)).toBeInTheDocument();
  });
});
