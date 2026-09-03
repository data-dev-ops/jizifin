/**
 * duplicateDetector.js
 *
 * Provides duplicate detection for prospective expenses against existing expenses.
 * Matches on:
 * - Matching category (case-insensitive)
 * - Matching amount in cents (cost_cents)
 */

/**
 * Finds all existing expenses matching a prospective expense by category and amount.
 *
 * @param {Object} prospectiveExpense { category: string, cost_cents: number, id?: number }
 * @param {Array<Object>} existingExpenses List of existing decrypted expenses
 * @returns {Array<Object>} Matching existing expenses
 */
export function findMatchingExpenses(prospectiveExpense, existingExpenses = []) {
  if (!prospectiveExpense || !prospectiveExpense.category || prospectiveExpense.cost_cents === undefined) {
    return [];
  }

  const targetCategory = String(prospectiveExpense.category).trim().toLowerCase();
  const targetCents = Number(prospectiveExpense.cost_cents);

  if (!targetCategory || isNaN(targetCents) || targetCents <= 0) {
    return [];
  }

  return (existingExpenses || []).filter((existing) => {
    if (!existing || !existing.category || existing.cost_cents === undefined) return false;
    // If editing, don't match the same expense record
    if (prospectiveExpense.id && existing.id === prospectiveExpense.id) return false;

    const existingCategory = String(existing.category).trim().toLowerCase();
    const existingCents = Number(existing.cost_cents);

    return existingCategory === targetCategory && existingCents === targetCents;
  });
}

/**
 * Identifies duplicate matches across a list of prospective expenses.
 *
 * @param {Array<Object>} prospectiveExpenses
 * @param {Array<Object>} existingExpenses
 * @returns {Array<{ prospective: Object, matches: Array<Object> }>}
 */
export function findBatchDuplicateMatches(prospectiveExpenses = [], existingExpenses = []) {
  const duplicateResults = [];

  for (const prospective of prospectiveExpenses) {
    const matches = findMatchingExpenses(prospective, existingExpenses);
    if (matches.length > 0) {
      duplicateResults.push({
        prospective,
        matches,
      });
    }
  }

  return duplicateResults;
}
