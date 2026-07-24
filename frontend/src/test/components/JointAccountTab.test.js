import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen, waitFor } from '@testing-library/svelte';
import JointAccountTab from '../../lib/JointAccountTab.svelte';
import {
  jointAccount,
  jointCategories,
  jointDeposits,
  jointExpectedCosts,
  jointCorrections,
  jointDashboard,
  splits,
  users,
} from '../../lib/stores.js';
import * as api from '../../lib/api.js';

// ── Helpers ────────────────────────────────────────────────────────────────

const mockAccount = {
  id: 1,
  name: 'Household',
  balance_cents: 125000,
  safety_margin_pct: 10,
  deposit_split_mode: 'even',
  expected_total_cents: 200000,
};

const mockDash = {
  month: '2026-07',
  balance_cents: 125000,
  expected_total_cents: 200000,
  actual_total_cents: 80000,
  total_deposits_cents: 220000,
  safety_margin_pct: 10,
  target_deposit_cents: 220000,
  categories: [
    { category: 'GROCERIES', actual_cents: 50000, expected_cents: 100000, pct_used: 50.0 },
    { category: 'UTILITIES', actual_cents: 30000, expected_cents: 60000,  pct_used: 50.0 },
  ],
  has_joint_account: true,
};

function seedEmpty() {
  jointAccount.set(null);
  jointCategories.set([]);
  jointDeposits.set([]);
  jointExpectedCosts.set([]);
  jointCorrections.set([]);
  jointDashboard.set(null);
  splits.set([
    { category: 'GROCERIES', allocations: [] },
    { category: 'UTILITIES', allocations: [] },
  ]);
  users.set([
    { name: 'Jim',  color: '#6366f1', is_active: 1 },
    { name: 'Zina', color: '#f59e0b', is_active: 1 },
  ]);
}

function seedWithAccount() {
  jointAccount.set(mockAccount);
  jointDashboard.set(mockDash);
  jointCategories.set([
    { enc: 'enc_GROCERIES', plain: 'GROCERIES' },
    { enc: 'enc_UTILITIES', plain: 'UTILITIES' },
  ]);
  jointDeposits.set([
    { user_name: 'Jim',  amount_cents: 110000, day_of_month: 1 },
    { user_name: 'Zina', amount_cents: 110000, day_of_month: 1 },
  ]);
  jointExpectedCosts.set([
    { category: 'GROCERIES', expected_cents: 100000 },
    { category: 'UTILITIES', expected_cents: 60000  },
  ]);
  jointCorrections.set([
    { id: 1, amount_cents: 110000, correction_date: '2026-07-01', note: 'Jim deposit' },
    { id: 2, amount_cents: 110000, correction_date: '2026-07-01', note: 'Zina deposit' },
  ]);
  splits.set([
    { category: 'GROCERIES', allocations: [] },
    { category: 'UTILITIES', allocations: [] },
  ]);
  users.set([
    { name: 'Jim',  color: '#6366f1', is_active: 1 },
    { name: 'Zina', color: '#f59e0b', is_active: 1 },
  ]);
}

// ── Tests ──────────────────────────────────────────────────────────────────

describe('JointAccountTab.svelte — no account configured', () => {
  beforeEach(() => {
    seedEmpty();
    vi.restoreAllMocks();
  });

  it('renders setup prompt when no joint account exists', () => {
    render(JointAccountTab);
    expect(screen.getByText(/Set up your joint account/i)).toBeInTheDocument();
    expect(document.getElementById('ja-create-btn')).toBeInTheDocument();
  });

  it('disables create button when name is empty', () => {
    render(JointAccountTab);
    const btn = document.getElementById('ja-create-btn');
    expect(btn).toBeDisabled();
  });

  it('enables create button once name is entered', async () => {
    render(JointAccountTab);
    const nameInput = document.getElementById('ja-setup-name');
    await fireEvent.input(nameInput, { target: { value: 'Household' } });
    const btn = document.getElementById('ja-create-btn');
    expect(btn).not.toBeDisabled();
  });

  it('calls createJointAccount with correct payload', async () => {
    const spy = vi.spyOn(api, 'createJointAccount').mockResolvedValue({ ...mockAccount });
    vi.spyOn(api, 'fetchJointDashboard').mockResolvedValue(null);

    render(JointAccountTab);

    const nameInput = document.getElementById('ja-setup-name');
    await fireEvent.input(nameInput, { target: { value: 'Household' } });

    const balInput = document.getElementById('ja-setup-balance');
    await fireEvent.input(balInput, { target: { value: '1250.00' } });

    const btn = document.getElementById('ja-create-btn');
    await fireEvent.click(btn);

    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({ name: 'Household', balance_cents: 125000 })
      );
    });
  });
});

describe('JointAccountTab.svelte — account exists', () => {
  beforeEach(() => {
    seedWithAccount();
    vi.restoreAllMocks();
  });

  it('renders balance card with correct amount', () => {
    render(JointAccountTab);
    expect(screen.getByText(/1250\.00/)).toBeInTheDocument();
    expect(screen.getByText('Household')).toBeInTheDocument();
  });

  it('renders overview stat cards', () => {
    render(JointAccountTab);
    expect(screen.getByText(/Spent this month/i)).toBeInTheDocument();
    expect(screen.getByText(/Target deposit/i)).toBeInTheDocument();
    expect(screen.getByText(/Deposits this month/i)).toBeInTheDocument();
  });

  it('renders category progress bars in overview', () => {
    render(JointAccountTab);
    expect(screen.getByText('GROCERIES')).toBeInTheDocument();
    expect(screen.getByText('UTILITIES')).toBeInTheDocument();
  });

  it('renders sub-navigation tabs', () => {
    render(JointAccountTab);
    expect(document.getElementById('ja-nav-overview')).toBeInTheDocument();
    expect(document.getElementById('ja-nav-categories')).toBeInTheDocument();
    expect(document.getElementById('ja-nav-deposits')).toBeInTheDocument();
    expect(document.getElementById('ja-nav-expected')).toBeInTheDocument();
    expect(document.getElementById('ja-nav-corrections')).toBeInTheDocument();
    expect(document.getElementById('ja-nav-settle')).toBeInTheDocument();
  });

  it('navigates to Categories section', async () => {
    render(JointAccountTab);
    await fireEvent.click(document.getElementById('ja-nav-categories'));
    expect(screen.getByText(/Joint account categories/i)).toBeInTheDocument();
    expect(screen.getByText(/GROCERIES/)).toBeInTheDocument();
    expect(screen.getByText(/UTILITIES/)).toBeInTheDocument();
  });

  it('navigates to Corrections section and shows log form', async () => {
    render(JointAccountTab);
    await fireEvent.click(document.getElementById('ja-nav-corrections'));
    expect(screen.getByText(/Log a balance correction/i)).toBeInTheDocument();
    expect(document.getElementById('ja-corr-amount')).toBeInTheDocument();
    expect(document.getElementById('ja-add-corr-btn')).toBeInTheDocument();
  });

  it('shows correction history when corrections exist', async () => {
    render(JointAccountTab);
    await fireEvent.click(document.getElementById('ja-nav-corrections'));
    expect(screen.getByText('Jim deposit')).toBeInTheDocument();
    expect(screen.getByText('Zina deposit')).toBeInTheDocument();
  });

  it('deletes correction on confirm', async () => {
    const spy = vi.spyOn(api, 'deleteJointCorrection').mockResolvedValue(undefined);
    vi.spyOn(api, 'fetchJointDashboard').mockResolvedValue(mockDash);
    vi.spyOn(window, 'confirm').mockReturnValue(true);

    render(JointAccountTab);
    await fireEvent.click(document.getElementById('ja-nav-corrections'));
    await fireEvent.click(document.getElementById('ja-del-corr-1'));

    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(1);
    });
  });

  it('navigates to Settle section', async () => {
    render(JointAccountTab);
    await fireEvent.click(document.getElementById('ja-nav-settle'));
    expect(screen.getByText(/Settle balance for/i)).toBeInTheDocument();
    expect(document.getElementById('ja-settle-mode')).toBeInTheDocument();
    expect(document.getElementById('ja-settle-btn')).toBeInTheDocument();
  });

  it('calls settleJointAccount with direct_pay mode', async () => {
    const spy = vi.spyOn(api, 'settleJointAccount').mockResolvedValue({
      mode: 'direct_pay',
      month: '2026-07',
      difference_cents: 14000,
      message: 'Surplus',
    });
    vi.spyOn(api, 'fetchJointDeposits').mockResolvedValue([]);

    render(JointAccountTab);
    await fireEvent.click(document.getElementById('ja-nav-settle'));
    await fireEvent.click(document.getElementById('ja-settle-btn'));

    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({ mode: 'direct_pay' })
      );
    });
    expect(screen.getAllByText(/Surplus/i).length).toBeGreaterThan(0);
  });

  it('saves deposit schedule via setJointDeposits', async () => {
    const spy = vi.spyOn(api, 'setJointDeposits').mockResolvedValue([]);

    render(JointAccountTab);
    await fireEvent.click(document.getElementById('ja-nav-deposits'));
    await fireEvent.click(document.getElementById('ja-save-deposits-btn'));

    await waitFor(() => {
      expect(spy).toHaveBeenCalled();
    });
  });

  it('calls deleteJointAccount on delete confirmation', async () => {
    const spy = vi.spyOn(api, 'deleteJointAccount').mockResolvedValue(undefined);
    vi.spyOn(window, 'confirm').mockReturnValue(true);

    render(JointAccountTab);
    await fireEvent.click(document.getElementById('ja-delete-btn'));

    await waitFor(() => {
      expect(spy).toHaveBeenCalled();
    });
  });

  it('shows validation error when logging correction without amount', async () => {
    render(JointAccountTab);
    await fireEvent.click(document.getElementById('ja-nav-corrections'));
    await fireEvent.click(document.getElementById('ja-add-corr-btn'));
    expect(screen.getByText(/Amount and date required/i)).toBeInTheDocument();
  });
});
