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

  it('renders Login screen when app is unauthenticated', () => {
    render(App);
    expect(screen.getByText('Jizifin Finance')).toBeInTheDocument();
    expect(screen.getByLabelText(/Master Password/i)).toBeInTheDocument();
  });

  it('renders application shell and navigation when authenticated', async () => {
    const key = await deriveKey('test-pass');
    authSalt.set('test-pass');
    cryptoKey.set(key);

    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => [],
    }));

    render(App);

    expect(await screen.findByText('Dashboard')).toBeInTheDocument();
    expect(screen.getByText('Expenses')).toBeInTheDocument();
    expect(screen.getByText('Income')).toBeInTheDocument();
    expect(screen.getByText('Splits')).toBeInTheDocument();
    expect(screen.getByText('Settings')).toBeInTheDocument();
  });

  it('switches active tab when navigation item is clicked', async () => {
    const key = await deriveKey('test-pass');
    authSalt.set('test-pass');
    cryptoKey.set(key);

    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => [],
    }));

    render(App);

    const settingsTabBtn = document.getElementById('nav-settings');
    await fireEvent.click(settingsTabBtn);

    expect(await screen.findByText('Mobile Preferences')).toBeInTheDocument();
  });
});
