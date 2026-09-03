import { describe, it, expect } from 'vitest';
import { findMatchingExpenses, findBatchDuplicateMatches } from '../lib/duplicateDetector.js';

describe('duplicateDetector.js — Duplicate Expense Detection', () => {
  const existingExpenses = [
    {
      id: 1,
      name: 'Delhaize Groceries',
      cost_cents: 4550,
      category: 'GROCERIES',
      expense_date: '2026-08-01',
      who_paid: 'Jim',
    },
    {
      id: 2,
      name: 'Monthly Transit Pass',
      cost_cents: 4900,
      category: 'TRANSPORT',
      expense_date: '2026-08-02',
      who_paid: 'Jane',
    },
    {
      id: 3,
      name: 'Decathlon Sportswear',
      cost_cents: 3297,
      category: 'SPORTS',
      expense_date: '2026-08-03',
      who_paid: 'Jim',
    },
  ];

  it('matches existing expense when category and amount (cost_cents) match exactly', () => {
    const prospective = {
      name: 'Albert Heijn',
      cost_cents: 4550,
      category: 'GROCERIES',
    };

    const matches = findMatchingExpenses(prospective, existingExpenses);
    expect(matches).toHaveLength(1);
    expect(matches[0].name).toBe('Delhaize Groceries');
    expect(matches[0].cost_cents).toBe(4550);
  });

  it('matches case-insensitively on category', () => {
    const prospective = {
      name: 'Supermarket',
      cost_cents: 4550,
      category: 'groceries',
    };

    const matches = findMatchingExpenses(prospective, existingExpenses);
    expect(matches).toHaveLength(1);
    expect(matches[0].name).toBe('Delhaize Groceries');
  });

  it('returns empty array when category matches but amount differs', () => {
    const prospective = {
      name: 'Supermarket Small Buy',
      cost_cents: 1200,
      category: 'GROCERIES',
    };

    const matches = findMatchingExpenses(prospective, existingExpenses);
    expect(matches).toHaveLength(0);
  });

  it('returns empty array when amount matches but category differs', () => {
    const prospective = {
      name: 'Running Shoes',
      cost_cents: 4550,
      category: 'CLOTHING',
    };

    const matches = findMatchingExpenses(prospective, existingExpenses);
    expect(matches).toHaveLength(0);
  });

  it('ignores matching entry with same id (editing scenario)', () => {
    const prospective = {
      id: 1,
      name: 'Updated Delhaize Groceries',
      cost_cents: 4550,
      category: 'GROCERIES',
    };

    const matches = findMatchingExpenses(prospective, existingExpenses);
    expect(matches).toHaveLength(0);
  });

  it('handles empty or malformed inputs safely without throwing', () => {
    expect(findMatchingExpenses(null, existingExpenses)).toEqual([]);
    expect(findMatchingExpenses({}, existingExpenses)).toEqual([]);
    expect(findMatchingExpenses({ category: 'GROCERIES', cost_cents: 0 }, existingExpenses)).toEqual([]);
    expect(findMatchingExpenses({ category: 'GROCERIES', cost_cents: -50 }, existingExpenses)).toEqual([]);
    expect(findMatchingExpenses({ category: '', cost_cents: 4550 }, existingExpenses)).toEqual([]);
    expect(findMatchingExpenses({ category: 'GROCERIES', cost_cents: 4550 }, null)).toEqual([]);
  });

  it('finds batch duplicate matches across multiple prospective expenses', () => {
    const batch = [
      { name: 'Store A', cost_cents: 4550, category: 'GROCERIES' },
      { name: 'Store B', cost_cents: 3297, category: 'SPORTS' },
      { name: 'Store C', cost_cents: 9999, category: 'UTILITIES' },
    ];

    const results = findBatchDuplicateMatches(batch, existingExpenses);
    expect(results).toHaveLength(2);
    expect(results[0].prospective.name).toBe('Store A');
    expect(results[0].matches[0].name).toBe('Delhaize Groceries');
    expect(results[1].prospective.name).toBe('Store B');
    expect(results[1].matches[0].name).toBe('Decathlon Sportswear');
  });
});
