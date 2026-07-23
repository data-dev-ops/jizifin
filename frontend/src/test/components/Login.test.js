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

  it('renders login form with password input', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      status: 200,
      ok: true,
      json: async () => ({ value: 'some-encrypted-magic' }),
    }));

    render(Login);
    expect(screen.getByText('Jizifin Finance')).toBeInTheDocument();
    expect(screen.getByLabelText(/Master Password/i)).toBeInTheDocument();
  });

  it('shows error if submitted with an empty password', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ status: 200, ok: true, json: async () => ({}) }));

    render(Login);
    const button = screen.getByRole('button', { name: /Decrypt & Open/i });
    await fireEvent.click(button);

    expect(screen.getByText('Master password is required')).toBeInTheDocument();
  });

  it('shows error if password is incorrect', async () => {
    const key = await deriveKey('correct-pass');
    const validMagicEncrypted = await encryptText('FinanceTrackerAuth', key);

    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      status: 200,
      ok: true,
      json: async () => ({ value: validMagicEncrypted }),
    }));

    render(Login);
    const input = screen.getByLabelText(/Master Password/i);
    await fireEvent.input(input, { target: { value: 'wrong-pass' } });

    const button = screen.getByRole('button', { name: /Decrypt & Open/i });
    await fireEvent.click(button);

    expect(await screen.findByText('Incorrect master password')).toBeInTheDocument();
  });

  it('authenticates and sets crypto key when correct password is entered', async () => {
    const passphrase = 'correct-pass';
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

    // Wait until authSalt is updated
    await vi.waitFor(() => {
      expect(get(authSalt)).toBe(passphrase);
      expect(get(cryptoKey)).not.toBeNull();
    });
  });
});
