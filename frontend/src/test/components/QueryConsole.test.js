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

  it.each([
    { title: 'SQL Query' },
  ])('renders query console header and textarea ($title)', ({ title }) => {
    render(QueryConsole);

    expect(screen.getByText(title)).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/SELECT \* FROM expenses/i)).toBeInTheDocument();
  });

  it.each([
    { querySql: 'SELECT * FROM users', decryptedName: 'John', valStr: '1250' },
  ])('executes SQL query and renders decrypted table output ($querySql)', async ({ querySql, decryptedName, valStr }) => {
    const key = await deriveKey('query-test-key');
    const encName = await encryptText('John', key);

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
    await fireEvent.input(textarea, { target: { value: querySql } });

    const runBtn = document.getElementById('query-run');
    await fireEvent.click(runBtn);

    expect(await screen.findByText(decryptedName)).toBeInTheDocument();
    expect(screen.getByText(valStr)).toBeInTheDocument();
  });

  it.each([
    { exampleIdx: 0, expectedQuery: 'SELECT * FROM expenses' },
  ])('populates textarea when example query buttons are clicked (example $exampleIdx)', async ({ exampleIdx, expectedQuery }) => {
    render(QueryConsole);

    const exampleBtn = document.getElementById(`example-${exampleIdx}`);
    await fireEvent.click(exampleBtn);

    const textarea = document.getElementById('query-input');
    expect(textarea.value).toContain(expectedQuery);
  });

  it.each([
    { rowCount: 50, noticeText: 'Results truncated — only first 50 rows shown' },
  ])('displays truncation notice when query result exceeds maximum limit ($rowCount)', async ({ rowCount, noticeText }) => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        columns: ['id'],
        rows: [[1]],
        row_count: rowCount,
        truncated: true,
      }),
    }));

    render(QueryConsole);

    const exampleBtn = document.getElementById('example-0');
    await fireEvent.click(exampleBtn);

    const runBtn = document.getElementById('query-run');
    await fireEvent.click(runBtn);

    expect(await screen.findByText(new RegExp(noticeText, 'i'))).toBeInTheDocument();
  });

  it.each([
    { invalidSql: 'SELECT * FROM invalid_table', errDetail: 'no such table: invalid_table' },
  ])('displays query execution error on failure ($invalidSql)', async ({ invalidSql, errDetail }) => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: false,
      status: 400,
      json: async () => ({ detail: errDetail }),
    }));

    render(QueryConsole);

    const textarea = screen.getByPlaceholderText(/SELECT \* FROM expenses/i);
    await fireEvent.input(textarea, { target: { value: invalidSql } });

    const runBtn = document.getElementById('query-run');
    await fireEvent.click(runBtn);

    expect(await screen.findByText(/no such table/i)).toBeInTheDocument();
  });
});
