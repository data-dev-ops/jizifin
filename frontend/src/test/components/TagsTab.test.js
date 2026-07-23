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

  it('renders configured tag cards and totals', () => {
    render(TagsTab);

    expect(screen.getByText('Summer Trip')).toBeInTheDocument();
    expect(screen.getByText('Barcelona vacation')).toBeInTheDocument();
    expect(screen.getByText('€450.00')).toBeInTheDocument();
    expect(screen.getByText('5 expenses')).toBeInTheDocument();
  });

  it('validates tag creation input', async () => {
    render(TagsTab);

    const submitBtn = screen.getByRole('button', { name: /Create Tag/i });
    await fireEvent.click(submitBtn);

    expect(screen.getByText('Tag name is required.')).toBeInTheDocument();
  });

  it('submits valid tag creation payload', async () => {
    const createSpy = vi.spyOn(api, 'createTag').mockResolvedValue({});

    render(TagsTab);

    const nameInput = document.getElementById('tag-name');
    await fireEvent.input(nameInput, { target: { value: 'Birthday Party' } });

    const descInput = document.getElementById('tag-description');
    await fireEvent.input(descInput, { target: { value: '30th celebration' } });

    const submitBtn = screen.getByRole('button', { name: /Create Tag/i });
    await fireEvent.click(submitBtn);

    expect(createSpy).toHaveBeenCalledWith({
      name: 'Birthday Party',
      color: '#f59e0b',
      description: '30th celebration',
    });
  });

  it('deletes a tag with inline confirmation', async () => {
    const delSpy = vi.spyOn(api, 'deleteTag').mockResolvedValue({});

    render(TagsTab);

    const delBtn = screen.getByTitle('Delete tag');
    await fireEvent.click(delBtn);

    const yesBtn = screen.getByRole('button', { name: 'Yes' });
    await fireEvent.click(yesBtn);

    expect(delSpy).toHaveBeenCalledWith(1);
  });
});
