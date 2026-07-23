import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import UserManager from '../../lib/UserManager.svelte';
import { users } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('UserManager.svelte — Household User Settings', () => {
  beforeEach(() => {
    users.set([
      { name: 'Jim', color: '#6366f1', is_active: true },
      { name: 'Old User', color: '#6b7280', is_active: false },
    ]);
    vi.spyOn(api, 'fetchUsers').mockResolvedValue([]);
    vi.restoreAllMocks();
  });

  it('renders active and deactivated household members', () => {
    render(UserManager);

    expect(screen.getByText('Jim')).toBeInTheDocument();
    expect(screen.getByText('Old User')).toBeInTheDocument();
    expect(screen.getByText('inactive')).toBeInTheDocument();
  });

  it('validates new member name field', async () => {
    render(UserManager);

    const addBtn = screen.getByRole('button', { name: /\+ Add Member/i });
    await fireEvent.click(addBtn);

    expect(screen.getByText('Name is required.')).toBeInTheDocument();
  });

  it('submits new member payload', async () => {
    const createSpy = vi.spyOn(api, 'createUser').mockResolvedValue({});

    render(UserManager);

    const nameInput = screen.getByPlaceholderText(/e.g. Alex/i);
    await fireEvent.input(nameInput, { target: { value: 'Zina' } });

    const addBtn = screen.getByRole('button', { name: /\+ Add Member/i });
    await fireEvent.click(addBtn);

    expect(createSpy).toHaveBeenCalledWith({
      name: 'Zina',
      color: '#6366f1',
      is_active: true,
    });
  });

  it('toggles member active status', async () => {
    const updateSpy = vi.spyOn(api, 'updateUser').mockResolvedValue({});

    render(UserManager);

    const deactivateBtn = screen.getByRole('button', { name: /Deactivate/i });
    await fireEvent.click(deactivateBtn);

    expect(updateSpy).toHaveBeenCalledWith('Jim', { is_active: false });
  });
});
