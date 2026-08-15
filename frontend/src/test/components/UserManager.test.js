import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import UserManager from '../../lib/UserManager.svelte';
import { users } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('UserManager.svelte — Household User Settings', () => {
  beforeEach(() => {
    users.set([
      { name: 'John', color: '#6366f1', is_active: true },
      { name: 'Old User', color: '#6b7280', is_active: false },
    ]);
    vi.spyOn(api, 'fetchUsers').mockResolvedValue([]);
    vi.restoreAllMocks();
  });

  it.each([
    { activeUser: 'John', inactiveUser: 'Old User' },
  ])('renders active and deactivated household members ($activeUser)', ({ activeUser, inactiveUser }) => {
    render(UserManager);

    expect(screen.getByText(activeUser)).toBeInTheDocument();
    expect(screen.getByText(inactiveUser)).toBeInTheDocument();
    expect(screen.getByText('inactive')).toBeInTheDocument();
  });

  it.each([
    { expectedErr: 'Name is required.' },
  ])('validates new member name field ($expectedErr)', async ({ expectedErr }) => {
    render(UserManager);

    const addBtn = screen.getByRole('button', { name: /\+ Add Member/i });
    await fireEvent.click(addBtn);

    expect(screen.getByText(expectedErr)).toBeInTheDocument();
  });

  it.each([
    { name: 'Jane' },
  ])('submits new member payload with a randomized pastel colour ($name)', async ({ name }) => {
    const createSpy = vi.spyOn(api, 'createUser').mockResolvedValue({});

    render(UserManager);

    const nameInput = screen.getByPlaceholderText(/e.g. Alex/i);
    await fireEvent.input(nameInput, { target: { value: name } });

    const addBtn = screen.getByRole('button', { name: /\+ Add Member/i });
    await fireEvent.click(addBtn);

    expect(createSpy).toHaveBeenCalledWith({
      name: name,
      color: expect.stringMatching(/^#[0-9a-fA-F]{6}$/),
      is_active: true,
    });
  });

  it.each([
    { btnTitle: 'Randomize pastel colour' },
  ])('randomizes pastel colour when clicking randomize button ($btnTitle)', async ({ btnTitle }) => {
    render(UserManager);

    const colorInput = screen.getByTitle('Pick avatar colour');

    const randomizeBtn = screen.getByRole('button', { name: new RegExp(btnTitle, 'i') });
    await fireEvent.click(randomizeBtn);

    expect(colorInput.value).toMatch(/^#[0-9a-fA-F]{6}$/);
  });

  it.each([
    { targetUser: 'John', expectedStatus: false },
  ])('toggles member active status ($targetUser)', async ({ targetUser, expectedStatus }) => {
    const updateSpy = vi.spyOn(api, 'updateUser').mockResolvedValue({});

    render(UserManager);

    const deactivateBtn = screen.getByRole('button', { name: /Deactivate/i });
    await fireEvent.click(deactivateBtn);

    expect(updateSpy).toHaveBeenCalledWith(targetUser, { is_active: expectedStatus });
  });
});
