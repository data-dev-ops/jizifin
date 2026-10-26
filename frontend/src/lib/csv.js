/**
 * csv.js — bank statement import through jizifin-wasm (built by `make wasm`):
 * parsing, category cascade, micro-debit grouping, totals and entry drafts.
 */

import init, { parseStatement, prepareImport, summarizeImport, cascadeTargets, microDebits } from './wasm/jizifin_wasm.js';

let ready = null;

async function wasm() {
  ready ??= init();
  await ready;
}

/** `{ transactions, stats }` for a statement export of `bank` ('ING'). */
export async function parseBankCsv(text, bank = 'ING') {
  await wasm();
  return parseStatement(text, bank);
}

/** `{ expenses, incomes, grouped, expenseCents, incomeCents, groupedCents, uncategorized }`. */
export async function summarize(transactions) {
  await wasm();
  return summarizeImport(transactions);
}

/** Indices that take row `index`'s category (same beneficiary and direction). */
export async function cascadeFrom(transactions, index) {
  await wasm();
  return Array.from(cascadeTargets(transactions, index));
}

/** Indices of included debits below `maxCents`. */
export async function microIndices(transactions, maxCents) {
  await wasm();
  return Array.from(microDebits(transactions, maxCents));
}

/**
 * Entry drafts for reviewed rows; grouped debits are bundled into `bundleCategory`.
 * Each draft has a plaintext `label` for the caller to send as `name`.
 */
export async function draftsFor(transactions, bundleCategory = null) {
  await wasm();
  return prepareImport(transactions, bundleCategory ?? undefined);
}
