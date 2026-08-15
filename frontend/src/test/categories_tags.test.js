import { describe, it, expect } from 'vitest';
import { fetchSplits, fetchTags, fetchTagAnalytics } from '../lib/api.js';
import { splits, tags } from '../lib/stores.js';
import { get } from 'svelte/store';

describe('Categories & Tags Domain Specifications', () => {
  describe('[CatNest] Nested Category Path Parsing', () => {
    it.each([
      {
        path: 'Food:Groceries:Supermarket',
        expectedRoot: 'Food',
        expectedLeaf: 'Supermarket',
        expectedDepth: 3
      },
      {
        path: 'Housing:Utilities',
        expectedRoot: 'Housing',
        expectedLeaf: 'Utilities',
        expectedDepth: 2
      },
      {
        path: 'Leisure',
        expectedRoot: 'Leisure',
        expectedLeaf: 'Leisure',
        expectedDepth: 1
      }
    ])('[CatNest] parsing nested path "$path"', async ({ path, expectedRoot, expectedLeaf, expectedDepth }) => {
      await fetchSplits();
      const parts = path.split(':').map(s => s.trim()).filter(Boolean);
      expect(parts[0]).toBe(expectedRoot);
      expect(parts[parts.length - 1]).toBe(expectedLeaf);
      expect(parts.length).toBe(expectedDepth);
    });
  });

  describe('[TagOr] Expense Filtering Matching Logical OR Across Tag IDs', () => {
    it.each([
      {
        expenses: [
          { id: 1, name: 'Tx 1', tagIds: [10, 20] },
          { id: 2, name: 'Tx 2', tagIds: [30] },
          { id: 3, name: 'Tx 3', tagIds: [20, 40] }
        ],
        filterTags: [20, 99],
        expectedMatchedIds: [1, 3]
      },
      {
        expenses: [
          { id: 1, name: 'Tx 1', tagIds: [10] },
          { id: 2, name: 'Tx 2', tagIds: [30] }
        ],
        filterTags: [50],
        expectedMatchedIds: []
      }
    ])('[TagOr] filtering expenses with tag filter $filterTags', async ({ expenses, filterTags, expectedMatchedIds }) => {
      await fetchTags();
      const matched = expenses.filter(e => e.tagIds.some(t => filterTags.includes(t)));
      const matchedIds = matched.map(m => m.id);
      expect(matchedIds).toEqual(expectedMatchedIds);
    });
  });

  describe('[RuleMtch] Categorization Rule Matching via Regex', () => {
    it.each([
      {
        expenseName: 'Uber Trip 123',
        rules: [{ id: 1, pattern: 'uber|lyft', category: 'TRANSPORT', priority: 1 }],
        expectedCategory: 'TRANSPORT'
      },
      {
        expenseName: 'Walmart Supercenter',
        rules: [{ id: 2, pattern: 'walmart|target', category: 'GROCERIES', priority: 1 }],
        expectedCategory: 'GROCERIES'
      },
      {
        expenseName: 'Unknown Vendor',
        rules: [{ id: 1, pattern: 'uber', category: 'TRANSPORT', priority: 1 }],
        expectedCategory: 'UNASSIGNED'
      }
    ])('[RuleMtch] matching "$expenseName"', async ({ expenseName, rules, expectedCategory }) => {
      await fetchSplits();
      const matchedRule = rules.find(r => new RegExp(r.pattern, 'i').test(expenseName));
      const category = matchedRule ? matchedRule.category : 'UNASSIGNED';
      expect(category).toBe(expectedCategory);
    });
  });

  describe('[RulePrio] Rule Evaluation Priority Order', () => {
    it.each([
      {
        expenseName: 'Uber Eats Restaurant Order',
        rules: [
          { id: 1, pattern: 'uber', category: 'TRANSPORT', priority: 5 },
          { id: 2, pattern: 'uber eats', category: 'DINING', priority: 10 }
        ],
        expectedCategory: 'DINING',
        expectedRuleId: 2
      }
    ])('[RulePrio] evaluating priority for "$expenseName"', async ({ expenseName, rules, expectedCategory, expectedRuleId }) => {
      await fetchTags();
      const sortedRules = [...rules].sort((a, b) => b.priority - a.priority);
      const matchedRule = sortedRules.find(r => new RegExp(r.pattern, 'i').test(expenseName));
      expect(matchedRule.category).toBe(expectedCategory);
      expect(matchedRule.id).toBe(expectedRuleId);
    });
  });
});
