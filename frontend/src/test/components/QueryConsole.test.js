import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import QueryConsole from '../../lib/QueryConsole.svelte';
import { cryptoKey } from '../../lib/stores.js';
import { deriveKey, encryptText } from '../../lib/crypto.js';

describe('QueryConsole.svelte — Raw SQL Console', () => {
  beforeEach(async () => {
    const key = await deriveKey('query-test-key');
    cryptoKey.set(key);
    vi.restoreAllMocks();
  });

  it('renders query console header and textarea', () => {
    render(QueryConsole);

    expect(screen.getByText('SQL Query')).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/SELECT \* FROM expenses/i)).toBeInTheDocument();
  });

  it('executes SQL query and renders decrypted table output', async () => {
    const key = await deriveKey('query-test-key');
    const encName = await encryptText('Jim', key);

    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        columns: ['name', 'cost_cents'],
        rows: [[encName, 1250]],
        row_count: 1,
        truncated: false,
      }),
    }));

    render(QueryConsole);

    const textarea = screen.getByPlaceholderText(/SELECT \* FROM expenses/i);
    await fireEvent.input(textarea, { target: { value: 'SELECT * FROM users' } });

    const runBtn = document.getElementById('query-run');
    await fireEvent.click(runBtn);

    expect(await screen.findByText('Jim')).toBeInTheDocument();
    expect(screen.getByText('1250')).toBeInTheDocument();
  });

  it('populates textarea when example query buttons are clicked', async () => {
    render(QueryConsole);

    const exampleBtn = document.getElementById('example-0');
    await fireEvent.click(exampleBtn);

    const textarea = document.getElementById('query-input');
    expect(textarea.value).toContain('SELECT * FROM expenses');
  });

  it('displays truncation notice when query result exceeds maximum limit', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        columns: ['id'],
        rows: [[1]],
        row_count: 50,
        truncated: true,
      }),
    }));

    render(QueryConsole);

    const exampleBtn = document.getElementById('example-0');
    await fireEvent.click(exampleBtn);

    const runBtn = document.getElementById('query-run');
    await fireEvent.click(runBtn);

    expect(await screen.findByText(/Results truncated — only first 50 rows shown/i)).toBeInTheDocument();
  });

  it('displays query execution error on failure', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: false,
      status: 400,
      json: async () => ({ detail: 'no such table: invalid_table' }),
    }));

    render(QueryConsole);

    const textarea = screen.getByPlaceholderText(/SELECT \* FROM expenses/i);
    await fireEvent.input(textarea, { target: { value: 'SELECT * FROM invalid_table' } });

    const runBtn = document.getElementById('query-run');
    await fireEvent.click(runBtn);

    expect(await screen.findByText(/no such table/i)).toBeInTheDocument();
  });
});
