import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import TagsTab from '../../lib/TagsTab.svelte';
import { tags, expenses, theme } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('TagsTab.svelte — Open-Ended Event Label Tags', () => {
  beforeEach(() => {
    tags.set([
      {
        id: 1,
        name: 'Summer Trip',
        color: '#f59e0b',
        description: 'Barcelona vacation',
        total_amount: 450.0,
        expense_count: 5,
        first_date: '2026-06-01',
        last_date: '2026-06-10',
      },
    ]);
    vi.restoreAllMocks();
  });

  it.each([
    { tagName: 'Summer Trip', desc: 'Barcelona vacation', amountFormatted: '€450.00', countStr: '5 expenses' },
  ])('renders configured tag cards and totals ($tagName)', ({ tagName, desc, amountFormatted, countStr }) => {
    render(TagsTab);

    expect(screen.getByText(tagName)).toBeInTheDocument();
    expect(screen.getByText(desc)).toBeInTheDocument();
    expect(screen.getByText(amountFormatted)).toBeInTheDocument();
    expect(screen.getByText(countStr)).toBeInTheDocument();
  });

  it.each([
    { expectedErr: 'Tag name is required.' },
  ])('validates tag creation input ($expectedErr)', async ({ expectedErr }) => {
    render(TagsTab);

    const submitBtn = screen.getByRole('button', { name: /Create Tag/i });
    await fireEvent.click(submitBtn);

    expect(screen.getByText(expectedErr)).toBeInTheDocument();
  });

  it.each([
    { name: 'Birthday Party', desc: '30th celebration', color: '#f59e0b' },
  ])('submits valid tag creation payload ($name)', async ({ name, desc, color }) => {
    const createSpy = vi.spyOn(api, 'createTag').mockResolvedValue({});

    render(TagsTab);

    const nameInput = document.getElementById('tag-name');
    await fireEvent.input(nameInput, { target: { value: name } });

    const descInput = document.getElementById('tag-description');
    await fireEvent.input(descInput, { target: { value: desc } });

    const submitBtn = screen.getByRole('button', { name: /Create Tag/i });
    await fireEvent.click(submitBtn);

    expect(createSpy).toHaveBeenCalledWith(expect.objectContaining({
      name: name,
      description: desc,
    }));
  });

  it.each([
    { tagId: 1 },
  ])('toggles tag active status ($tagId)', async ({ tagId }) => {
    const updateSpy = vi.spyOn(api, 'updateTag').mockResolvedValue({});

    render(TagsTab);

    const toggleBtn = document.getElementById(`toggle-tag-active-${tagId}`);
    expect(toggleBtn).toBeInTheDocument();
    expect(toggleBtn).toHaveTextContent('Active');

    await fireEvent.click(toggleBtn);
    expect(updateSpy).toHaveBeenCalledWith(tagId, { is_active: false });
  });

  it.each([
    { tagId: 1 },
  ])('deletes a tag with inline confirmation ($tagId)', async ({ tagId }) => {
    const delSpy = vi.spyOn(api, 'deleteTag').mockResolvedValue({});

    render(TagsTab);

    const delBtn = screen.getByTitle('Delete tag');
    await fireEvent.click(delBtn);

    const yesBtn = screen.getByRole('button', { name: 'Yes' });
    await fireEvent.click(yesBtn);

    expect(delSpy).toHaveBeenCalledWith(tagId);
  });

  it('selects a tag, renders itemized expense list and charts without freezing, and responds to theme changes', async () => {
    expenses.set([
      { id: 101, name: 'Flight to BCN', cost_cents: 20000, expense_date: '2026-06-01', who_paid: 'John', category: 'TRAVEL', tag_id: 1 },
      { id: 102, name: 'Tapas Dinner', cost_cents: 8000, expense_date: '2026-06-02', who_paid: 'Jane', category: 'FOOD', tag_id: 1 },
      { id: 103, name: 'Metro Pass', cost_cents: 2000, expense_date: '2026-06-03', who_paid: 'John', category: 'TRANSPORT', tag_id: 2 },
    ]);

    const analyticsSpy = vi.spyOn(api, 'fetchTagAnalytics').mockResolvedValue({
      tag: {
        id: 1,
        name: 'Summer Trip',
        color: '#f59e0b',
        description: 'Barcelona vacation',
        total_amount: 280.0,
        expense_count: 2,
        first_date: '2026-06-01',
        last_date: '2026-06-02',
        is_active: 1,
      },
      by_month: [
        { month: '2026-06', total_amount: 280.0, expense_count: 2 }
      ],
      by_category: [
        { category: 'TRAVEL', total_amount: 200.0, expense_count: 1 },
        { category: 'FOOD', total_amount: 80.0, expense_count: 1 }
      ]
    });

    render(TagsTab);

    const tagCard = document.getElementById('tag-card-1');
    expect(tagCard).toBeInTheDocument();

    // Click the tag card to open detail view
    await fireEvent.click(tagCard);
    expect(analyticsSpy).toHaveBeenCalledWith(1);

    // Verify detail header and statistics
    expect(await screen.findByText('All Expenses — 2 total')).toBeInTheDocument();
    expect(screen.getByText('Flight to BCN')).toBeInTheDocument();
    expect(screen.getByText('Tapas Dinner')).toBeInTheDocument();
    // Non-matching tag expense should not appear in the tag table
    expect(screen.queryByText('Metro Pass')).not.toBeInTheDocument();

    // Verify expense table exists
    const table = document.getElementById('tag-expense-table-1');
    expect(table).toBeInTheDocument();

    // Verify theme change does not trigger freeze or error
    theme.set('light');
    theme.set('dark');

    // Deselect tag
    await fireEvent.click(tagCard);
    expect(screen.getByText('Select a tag')).toBeInTheDocument();
    expect(document.getElementById('tag-expense-table-1')).not.toBeInTheDocument();
  });
});
