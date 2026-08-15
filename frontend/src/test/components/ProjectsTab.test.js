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

  it.each([
    { projName: 'New Car', spentFormatted: '€5,000.00', targetFormatted: '€10,000.00', pct: '50%' },
  ])('renders project cards with progress bars ($projName)', ({ projName, spentFormatted, targetFormatted, pct }) => {
    render(ProjectsTab);

    expect(screen.getByText(projName)).toBeInTheDocument();
    expect(screen.getByText(new RegExp(spentFormatted))).toBeInTheDocument();
    expect(screen.getByText(new RegExp(targetFormatted))).toBeInTheDocument();
    expect(screen.getByText(pct)).toBeInTheDocument();
  });

  it.each([
    { expectedErr: 'Project name required.' },
  ])('validates project creation inputs ($expectedErr)', async ({ expectedErr }) => {
    render(ProjectsTab);

    const submitBtn = document.getElementById('submit-project');
    await fireEvent.click(submitBtn);

    expect(screen.getByText(expectedErr)).toBeInTheDocument();
  });

  it.each([
    { name: 'Summer Holiday', targetStr: '2000.00', targetDate: '2026-12-31', expectedCents: 200000 },
  ])('submits valid project payload ($name)', async ({ name, targetStr, targetDate, expectedCents }) => {
    const createSpy = vi.spyOn(api, 'createProject').mockResolvedValue({});

    render(ProjectsTab);

    const nameInput = document.getElementById('project-name');
    await fireEvent.input(nameInput, { target: { value: name } });

    const targetInput = document.getElementById('project-target');
    await fireEvent.input(targetInput, { target: { value: targetStr } });

    const dateInput = document.getElementById('project-date');
    await fireEvent.input(dateInput, { target: { value: targetDate } });

    const submitBtn = document.getElementById('submit-project');
    await fireEvent.click(submitBtn);

    expect(createSpy).toHaveBeenCalledWith({
      name: name,
      target_cents: expectedCents,
      target_date: targetDate,
      is_joint: false,
      allow_subcategories: true,
    });
  });

  it.each([
    { projId: 1 },
  ])('deletes a project goal with inline confirmation ($projId)', async ({ projId }) => {
    const delSpy = vi.spyOn(api, 'deleteProject').mockResolvedValue({});

    render(ProjectsTab);

    const delBtn = document.getElementById(`delete-project-${projId}`);
    await fireEvent.click(delBtn);

    const yesBtn = screen.getByRole('button', { name: 'Yes' });
    await fireEvent.click(yesBtn);

    expect(delSpy).toHaveBeenCalledWith(projId);
  });
});
