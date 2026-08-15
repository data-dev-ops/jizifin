import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import TagsTab from '../../lib/TagsTab.svelte';
import { tags } from '../../lib/stores.js';
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
  ])('deletes a tag with inline confirmation ($tagId)', async ({ tagId }) => {
    const delSpy = vi.spyOn(api, 'deleteTag').mockResolvedValue({});

    render(TagsTab);

    const delBtn = screen.getByTitle('Delete tag');
    await fireEvent.click(delBtn);

    const yesBtn = screen.getByRole('button', { name: 'Yes' });
    await fireEvent.click(yesBtn);

    expect(delSpy).toHaveBeenCalledWith(tagId);
  });
});
