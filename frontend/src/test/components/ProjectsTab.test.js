import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import ProjectsTab from '../../lib/ProjectsTab.svelte';
import { projects } from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('ProjectsTab.svelte — Target Budget Goals & Completion Tracker', () => {
  beforeEach(() => {
    projects.set([
      {
        id: 1,
        name: 'New Car',
        target_cents: 1000000,
        target_date: '2027-12-31',
        total_spent_cents: 500000,
        estimated_completion_date: '2027-06-30',
      },
    ]);
    vi.restoreAllMocks();
  });

  it('renders project cards with progress bars', () => {
    render(ProjectsTab);

    expect(screen.getByText('New Car')).toBeInTheDocument();
    expect(screen.getByText(/€5,000\.00/)).toBeInTheDocument();
    expect(screen.getByText(/€10,000\.00/)).toBeInTheDocument();
    expect(screen.getByText('50%')).toBeInTheDocument();
  });

  it('validates project creation inputs', async () => {
    render(ProjectsTab);

    const submitBtn = document.getElementById('submit-project');
    await fireEvent.click(submitBtn);

    expect(screen.getByText('Project name required.')).toBeInTheDocument();
  });

  it('submits valid project payload', async () => {
    const createSpy = vi.spyOn(api, 'createProject').mockResolvedValue({});

    render(ProjectsTab);

    const nameInput = document.getElementById('project-name');
    await fireEvent.input(nameInput, { target: { value: 'Summer Holiday' } });

    const targetInput = document.getElementById('project-target');
    await fireEvent.input(targetInput, { target: { value: '2000.00' } });

    const dateInput = document.getElementById('project-date');
    await fireEvent.input(dateInput, { target: { value: '2026-12-31' } });

    const submitBtn = document.getElementById('submit-project');
    await fireEvent.click(submitBtn);

    expect(createSpy).toHaveBeenCalledWith({
      name: 'Summer Holiday',
      target_cents: 200000,
      target_date: '2026-12-31',
    });
  });

  it('deletes a project goal with inline confirmation', async () => {
    const delSpy = vi.spyOn(api, 'deleteProject').mockResolvedValue({});

    render(ProjectsTab);

    const delBtn = document.getElementById('delete-project-1');
    await fireEvent.click(delBtn);

    const yesBtn = screen.getByRole('button', { name: 'Yes' });
    await fireEvent.click(yesBtn);

    expect(delSpy).toHaveBeenCalledWith(1);
  });
});
