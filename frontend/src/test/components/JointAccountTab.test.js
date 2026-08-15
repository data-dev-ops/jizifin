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
    { name: 'John',  color: '#6366f1', is_active: 1 },
    { name: 'Jane', color: '#f59e0b', is_active: 1 },
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
    { user_name: 'John',  amount_cents: 110000, day_of_month: 1 },
    { user_name: 'Jane', amount_cents: 110000, day_of_month: 1 },
  ]);
  jointExpectedCosts.set([
    { category: 'GROCERIES', expected_cents: 100000 },
    { category: 'UTILITIES', expected_cents: 60000  },
  ]);
  jointCorrections.set([
    { id: 1, amount_cents: 110000, correction_date: '2026-07-01', note: 'John deposit' },
    { id: 2, amount_cents: 110000, correction_date: '2026-07-01', note: 'Jane deposit' },
  ]);
  splits.set([
    { category: 'GROCERIES', allocations: [] },
    { category: 'UTILITIES', allocations: [] },
  ]);
  users.set([
    { name: 'John',  color: '#6366f1', is_active: 1 },
    { name: 'Jane', color: '#f59e0b', is_active: 1 },
  ]);
}

// ── Tests ──────────────────────────────────────────────────────────────────

describe('JointAccountTab.svelte — no account configured', () => {
  beforeEach(() => {
    seedEmpty();
    vi.restoreAllMocks();
  });

  it.each([
    { title: 'Set Up Household Joint Account' },
  ])('renders setup prompt when no joint account exists ($title)', ({ title }) => {
    render(JointAccountTab);
    expect(screen.getByText(new RegExp(title, 'i'))).toBeInTheDocument();
    expect(document.getElementById('ja-create-btn')).toBeInTheDocument();
  });

  it.each([
    { btnId: 'ja-create-btn' },
  ])('disables create button when name is empty ($btnId)', ({ btnId }) => {
    render(JointAccountTab);
    const btn = document.getElementById(btnId);
    expect(btn).toBeDisabled();
  });

  it.each([
    { accountName: 'Household' },
  ])('enables create button once name is entered ($accountName)', async ({ accountName }) => {
    render(JointAccountTab);
    const nameInput = document.getElementById('ja-setup-name');
    await fireEvent.input(nameInput, { target: { value: accountName } });
    const btn = document.getElementById('ja-create-btn');
    expect(btn).not.toBeDisabled();
  });

  it.each([
    { accountName: 'Household', balStr: '1250.00', expectedCents: 125000 },
  ])('calls createJointAccount with correct payload ($accountName)', async ({ accountName, balStr, expectedCents }) => {
    const spy = vi.spyOn(api, 'createJointAccount').mockResolvedValue({ ...mockAccount });
    vi.spyOn(api, 'fetchJointDashboard').mockResolvedValue(null);

    render(JointAccountTab);

    const nameInput = document.getElementById('ja-setup-name');
    await fireEvent.input(nameInput, { target: { value: accountName } });

    const balInput = document.getElementById('ja-setup-balance');
    await fireEvent.input(balInput, { target: { value: balStr } });

    const btn = document.getElementById('ja-create-btn');
    await fireEvent.click(btn);

    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({ name: accountName, balance_cents: expectedCents })
      );
    });
  });
});

describe('JointAccountTab.svelte — account exists', () => {
  beforeEach(() => {
    seedWithAccount();
    vi.restoreAllMocks();
  });

  it.each([
    { name: 'Household' },
  ])('renders balance card with correct amount ($name)', ({ name }) => {
    render(JointAccountTab);
    expect(screen.getByText(/1250\.00/)).toBeInTheDocument();
    expect(screen.getByText(name)).toBeInTheDocument();
  });

  it.each([
    { cardLabel: 'Spent' },
  ])('renders overview stat cards ($cardLabel)', ({ cardLabel }) => {
    render(JointAccountTab);
    expect(screen.getByText(new RegExp(cardLabel, 'i'))).toBeInTheDocument();
    expect(screen.getByText(/Target Deposit/i)).toBeInTheDocument();
    expect(screen.getByText(/Deposits Received/i)).toBeInTheDocument();
  });

  it.each([
    { cat: 'GROCERIES' },
  ])('renders category progress bars in overview ($cat)', ({ cat }) => {
    render(JointAccountTab);
    expect(screen.getAllByText(cat).length).toBeGreaterThan(0);
    expect(screen.getAllByText('UTILITIES').length).toBeGreaterThan(0);
  });

  it.each([
    { tabId: 'ja-nav-overview' },
  ])('renders sub-navigation tabs ($tabId)', ({ tabId }) => {
    render(JointAccountTab);
    expect(document.getElementById(tabId)).toBeInTheDocument();
    expect(document.getElementById('ja-nav-categories')).toBeInTheDocument();
    expect(document.getElementById('ja-nav-deposits')).toBeInTheDocument();
    expect(document.getElementById('ja-nav-expected')).toBeInTheDocument();
    expect(document.getElementById('ja-nav-corrections')).toBeInTheDocument();
    expect(document.getElementById('ja-nav-settle')).toBeInTheDocument();
  });

  it.each([
    { tabId: 'ja-nav-categories' },
  ])('navigates to Categories section ($tabId)', async ({ tabId }) => {
    render(JointAccountTab);
    await fireEvent.click(document.getElementById(tabId));
    expect(screen.getByText(/Joint Account Categories/i)).toBeInTheDocument();
    expect(screen.getAllByText(/GROCERIES/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/UTILITIES/).length).toBeGreaterThan(0);
  });

  it.each([
    { tabId: 'ja-nav-corrections' },
  ])('navigates to Corrections section and shows log form ($tabId)', async ({ tabId }) => {
    render(JointAccountTab);
    await fireEvent.click(document.getElementById('ja-nav-categories'));
    await fireEvent.click(document.getElementById(tabId));
    expect(screen.getByText(/Log Balance Correction/i)).toBeInTheDocument();
    expect(document.getElementById('ja-corr-amount')).toBeInTheDocument();
    expect(document.getElementById('ja-add-corr-btn')).toBeInTheDocument();
  });

  it.each([
    { note: 'John deposit' },
  ])('shows correction history when corrections exist ($note)', async ({ note }) => {
    render(JointAccountTab);
    await fireEvent.click(document.getElementById('ja-nav-corrections'));
    expect(screen.getByText(new RegExp(note, 'i'))).toBeInTheDocument();
    expect(screen.getByText(/Jane deposit/i)).toBeInTheDocument();
  });

  it.each([
    { corrId: 1 },
  ])('deletes correction on confirm ($corrId)', async ({ corrId }) => {
    const spy = vi.spyOn(api, 'deleteJointCorrection').mockResolvedValue(undefined);
    vi.spyOn(api, 'fetchJointDashboard').mockResolvedValue(mockDash);
    vi.spyOn(window, 'confirm').mockReturnValue(true);

    render(JointAccountTab);
    await fireEvent.click(document.getElementById('ja-nav-corrections'));
    await fireEvent.click(document.getElementById(`ja-del-corr-${corrId}`));

    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(corrId);
    });
  });

  it.each([
    { tabId: 'ja-nav-settle' },
  ])('navigates to Settle section ($tabId)', async ({ tabId }) => {
    render(JointAccountTab);
    await fireEvent.click(document.getElementById(tabId));
    expect(screen.getByText(/Settle Joint Account/i)).toBeInTheDocument();
    expect(document.getElementById('settle-option-direct-pay')).toBeInTheDocument();
    expect(document.getElementById('ja-settle-btn')).toBeInTheDocument();
  });

  it.each([
    { mode: 'direct_pay' },
  ])('calls settleJointAccount with direct_pay mode ($mode)', async ({ mode }) => {
    const spy = vi.spyOn(api, 'settleJointAccount').mockResolvedValue({
      mode: mode,
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
        expect.objectContaining({ mode: mode })
      );
    });
    expect(screen.getAllByText(/Surplus/i).length).toBeGreaterThan(0);
  });

  it.each([
    { btnId: 'ja-save-deposits-btn' },
  ])('saves deposit schedule via setJointDeposits ($btnId)', async ({ btnId }) => {
    const spy = vi.spyOn(api, 'setJointDeposits').mockResolvedValue([]);

    render(JointAccountTab);
    await fireEvent.click(document.getElementById('ja-nav-deposits'));
    await fireEvent.click(document.getElementById(btnId));

    await waitFor(() => {
      expect(spy).toHaveBeenCalled();
    });
  });

  it.each([
    { btnId: 'ja-delete-btn' },
  ])('calls deleteJointAccount on delete confirmation ($btnId)', async ({ btnId }) => {
    const spy = vi.spyOn(api, 'deleteJointAccount').mockResolvedValue(undefined);
    vi.spyOn(window, 'confirm').mockReturnValue(true);

    render(JointAccountTab);
    await fireEvent.click(document.getElementById(btnId));

    await waitFor(() => {
      expect(spy).toHaveBeenCalled();
    });
  });

  it.each([
    { errText: 'Amount and date required' },
  ])('shows validation error when logging correction without amount ($errText)', async ({ errText }) => {
    render(JointAccountTab);
    await fireEvent.click(document.getElementById('ja-nav-corrections'));
    await fireEvent.click(document.getElementById('ja-add-corr-btn'));
    expect(screen.getByText(new RegExp(errText, 'i'))).toBeInTheDocument();
  });

  it.each([
    { msg: 'Proposed deposit amounts calculated' },
  ])('proposes deposit amounts based on expected costs and salary ratio ($msg)', async ({ msg }) => {
    vi.spyOn(api, 'fetchLatestSalaries').mockResolvedValue([
      { who: 'John', amount_cents: 400000 },
      { who: 'Jane', amount_cents: 200000 },
    ]);

    render(JointAccountTab);
    await fireEvent.click(document.getElementById('ja-nav-deposits'));

    const proposeBtn = document.getElementById('ja-propose-deposits-btn');
    expect(proposeBtn).toBeInTheDocument();
    await fireEvent.click(proposeBtn);

    await waitFor(() => {
      expect(screen.getByText(new RegExp(msg, 'i'))).toBeInTheDocument();
    });
  });

  it.each([
    { grossVal: '3500.00', expectedCents: 350000 },
  ])('supports gross total cost estimation mode and category specific estimation mode ($grossVal)', async ({ grossVal, expectedCents }) => {
    const updateSpy = vi.spyOn(api, 'updateJointAccount').mockResolvedValue({});
    render(JointAccountTab);

    await fireEvent.click(document.getElementById('ja-nav-expected'));
    expect(screen.getByText(/Expected Gross Cost Estimation/i)).toBeInTheDocument();
    expect(screen.getByText(/Category Specific Cost Estimations/i)).toBeInTheDocument();

    const grossRadio = document.getElementById('radio-estimation-gross');
    await fireEvent.click(grossRadio);

    const grossInput = document.getElementById('ja-gross-total');
    expect(grossInput).toBeInTheDocument();
    await fireEvent.input(grossInput, { target: { value: grossVal } });

    await fireEvent.click(document.getElementById('ja-save-expected-btn'));

    await waitFor(() => {
      expect(updateSpy).toHaveBeenCalledWith({ expected_total_cents: expectedCents });
    });
  });

  it.each([
    { inputVal: '834.50', expectedRounded: '840.00' },
  ])('rounds up deposit amounts to the nearest specified interval ($inputVal -> $expectedRounded)', async ({ inputVal, expectedRounded }) => {
    render(JointAccountTab);
    await fireEvent.click(document.getElementById('ja-nav-deposits'));

    const input0 = document.getElementById('ja-dep-amount-0');
    await fireEvent.input(input0, { target: { value: inputVal } });

    const roundBtn = document.getElementById('ja-round-deposits-btn');
    expect(roundBtn).toBeInTheDocument();
    await fireEvent.click(roundBtn);

    await waitFor(() => {
      expect(input0.value).toBe(expectedRounded);
    });
  });

  it.each([
    { user: 'John' },
  ])('allows marking a monthly deposit paid and entering a custom paid amount ($user)', async ({ user }) => {
    const markSpy = vi.spyOn(api, 'updateJointMonthlyDeposit').mockResolvedValue({
      user_name: user,
      scheduled_cents: 132000,
      day_of_month: 25,
      actual_cents: 135000,
      is_paid: true,
      status: 'paid_diverted',
    });

    render(JointAccountTab);
    await fireEvent.click(document.getElementById('ja-nav-deposits'));

    expect(screen.getByText(/Monthly Deposit Execution/i)).toBeInTheDocument();
    const markBtn = document.getElementById(`btn-mark-paid-${user}`);
    expect(markBtn).toBeInTheDocument();
    await fireEvent.click(markBtn);

    await waitFor(() => {
      expect(markSpy).toHaveBeenCalledWith(expect.objectContaining({
        user_name: user,
        is_paid: true,
      }));
    });
  });
});
