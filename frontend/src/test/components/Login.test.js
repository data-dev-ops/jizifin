import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import Login from '../../lib/Login.svelte';
import { authSalt, cryptoKey } from '../../lib/stores.js';
import { deriveKey, encryptText } from '../../lib/crypto.js';
import { get } from 'svelte/store';

describe('Login.svelte — Master Password & Auth Component', () => {
  beforeEach(() => {
    authSalt.set('');
    cryptoKey.set(null);
    vi.restoreAllMocks();
  });

  it.each([
    { appName: 'Jizifin Finance' },
  ])('renders login form with password input ($appName)', async ({ appName }) => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      status: 200,
      ok: true,
      json: async () => ({ value: 'some-encrypted-magic' }),
    }));

    render(Login);
    expect(screen.getByText(appName)).toBeInTheDocument();
    expect(screen.getByLabelText(/Master Password/i)).toBeInTheDocument();
  });

  it.each([
    { expectedErr: 'Master password is required' },
  ])('shows error if submitted with an empty password ($expectedErr)', async ({ expectedErr }) => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ status: 200, ok: true, json: async () => ({}) }));

    render(Login);
    const button = screen.getByRole('button', { name: /Decrypt & Open/i });
    await fireEvent.click(button);

    expect(screen.getByText(expectedErr)).toBeInTheDocument();
  });

  it.each([
    { wrongPass: 'wrong-pass', expectedErr: 'Incorrect master password' },
  ])('shows error if password is incorrect ($wrongPass)', async ({ wrongPass, expectedErr }) => {
    const key = await deriveKey('correct-pass');
    const validMagicEncrypted = await encryptText('FinanceTrackerAuth', key);

    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      status: 200,
      ok: true,
      json: async () => ({ value: validMagicEncrypted }),
    }));

    render(Login);
    const input = screen.getByLabelText(/Master Password/i);
    await fireEvent.input(input, { target: { value: wrongPass } });

    const button = screen.getByRole('button', { name: /Decrypt & Open/i });
    await fireEvent.click(button);

    expect(await screen.findByText(expectedErr)).toBeInTheDocument();
  });

  it.each([
    { passphrase: 'correct-pass' },
  ])('authenticates and sets crypto key when correct password is entered ($passphrase)', async ({ passphrase }) => {
    const key = await deriveKey(passphrase);
    const validMagicEncrypted = await encryptText('FinanceTrackerAuth', key);

    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      status: 200,
      ok: true,
      json: async () => ({ value: validMagicEncrypted }),
    }));

    render(Login);
    const input = screen.getByLabelText(/Master Password/i);
    await fireEvent.input(input, { target: { value: passphrase } });

    const button = screen.getByRole('button', { name: /Decrypt & Open/i });
    await fireEvent.click(button);

    await vi.waitFor(() => {
      expect(get(authSalt)).toBe(passphrase);
      expect(get(cryptoKey)).not.toBeNull();
    });
  });
});
