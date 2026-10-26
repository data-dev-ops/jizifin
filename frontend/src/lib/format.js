/**
 * format.js — presentation only: cents and dates to text, typed input to cents.
 */

import { derived } from 'svelte/store';
import { currencySymbol, currencyPrecisionMode } from './stores.js';

const GROUPED = { minimumFractionDigits: 2, maximumFractionDigits: 2 };

function render(cents, symbol, whole) {
  const value = Number(cents || 0) / 100;
  const text = whole ? Math.round(value).toLocaleString('en-GB') : value.toLocaleString('en-GB', GROUPED);
  return `${symbol}${text}`;
}

/** `$money(cents)` exact; `$money(cents, true)` whole units on summaries when enabled. */
export const money = derived([currencySymbol, currencyPrecisionMode], ([$symbol, $mode]) =>
  (cents, summary = false) => render(cents, $symbol, summary && $mode === 'whole_units_on_summaries'));

/** `YYYY-MM-DD` → `DD/MM/YYYY`. */
export function fmtDate(iso) {
  if (!iso) return '—';
  const [y, m, d] = iso.split('-');
  return `${d}/${m}/${y}`;
}

/** `YYYY-MM` → `Jul 2025` (or `July 2025` with `long`). */
export function fmtMonth(ym, long = false) {
  if (!ym) return '—';
  const [y, m] = ym.split('-').map(Number);
  return new Date(y, m - 1, 1).toLocaleDateString('en-GB', { month: long ? 'long' : 'short', year: 'numeric' });
}

/** Server percentage → `42.5%`. */
export function fmtPct(pct, digits = 0) {
  return pct == null ? '—' : `${Number(pct).toFixed(digits)}%`;
}

/** Typed amount (`12`, `12.3`, `12,34`, `-5`) → integer cents, or null when not a number. */
export function toCents(input) {
  const match = String(input ?? '').trim().replace(',', '.').match(/^(-?)(\d*)(?:\.(\d{0,2}))?$/);
  if (!match || (match[2] === '' && !match[3])) return null;
  const cents = Number(match[2] || 0) * 100 + Number((match[3] || '').padEnd(2, '0'));
  return match[1] ? -cents : cents;
}

/** Integer cents → editable text `12.34`. */
export function centsInput(cents) {
  return cents == null ? '' : (cents / 100).toFixed(2);
}

/** Today as `YYYY-MM-DD` (local). */
export function today() {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

/** `YYYY-MM` shifted by `delta` months. */
export function shiftMonth(ym, delta) {
  const [y, m] = ym.split('-').map(Number);
  const d = new Date(y, m - 1 + delta, 1);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}`;
}

/** First and last ISO day of `YYYY-MM`. */
export function monthBounds(ym) {
  const [y, m] = ym.split('-').map(Number);
  const last = new Date(y, m, 0).getDate();
  return { from: `${ym}-01`, to: `${ym}-${String(last).padStart(2, '0')}` };
}
