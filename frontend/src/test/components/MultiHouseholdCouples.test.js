import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen, waitFor } from '@testing-library/svelte';
import AnalyticsSummary from '../../lib/AnalyticsSummary.svelte';
import ProjectsTab from '../../lib/ProjectsTab.svelte';
import JointAccountTab from '../../lib/JointAccountTab.svelte';
import {
  users,
  projects,
  jointAccounts,
  activeJointAccountId,
  jointAccount,
  jointAccountEnabled,
  jointDashboard,
  dashboardScope,
  currencySymbol,
  analytics,
} from '../../lib/stores.js';
import * as api from '../../lib/api.js';

describe('Multi-Household & Couples Subset Filtering Frontend Integration', () => {
  const mockUsers = [
    { name: 'Alice', color: '#6366f1', is_active: 1 },
    { name: 'Bob', color: '#3b82f6', is_active: 1 },
    { name: 'Charlie', color: '#10b981', is_active: 1 },
    { name: 'Diana', color: '#f59e0b', is_active: 1 },
  ];

  const mockJointAccounts = [
    {
      id: 1,
      name: 'Alice & Bob Joint',
      balance_cents: 200000,
      safety_margin_pct: 10,
      deposit_split_mode: 'even',
      member_names: ['Alice', 'Bob'],
    },
    {
      id: 2,
      name: 'Charlie & Diana Joint',
      balance_cents: 300000,
      safety_margin_pct: 15,
      deposit_split_mode: 'even',
      member_names: ['Charlie', 'Diana'],
    },
  ];

  beforeEach(() => {
    currencySymbol.set('€');
    users.set(mockUsers);
    jointAccountEnabled.set(true);
    jointAccounts.set(mockJointAccounts);
    activeJointAccountId.set(1);
    jointAccount.set(mockJointAccounts[0]);
    dashboardScope.set('ALL');
    projects.set([
      {
        id: 1,
        name: 'Alice & Bob Vacation',
        target_cents: 200000,
        total_spent_cents: 50000,
        target_date: '2026-12-31',
        is_joint: 1,
        allow_subcategories: 1,
        user_names: ['Alice', 'Bob'],
      },
      {
        id: 2,
        name: 'Charlie & Diana Car',
        target_cents: 800000,
        total_spent_cents: 200000,
        target_date: '2027-06-30',
        is_joint: 1,
        allow_subcategories: 1,
        user_names: ['Charlie', 'Diana'],
      },
    ]);
    analytics.set({
      monthly_total: { total_amount: 1500.0, expense_count: 8, month: '2026-08' },
      by_payer: [
        { who_paid: 'Alice', total_amount: 500.0, expense_count: 3 },
        { who_paid: 'Bob', total_amount: 400.0, expense_count: 2 },
        { who_paid: 'Charlie', total_amount: 350.0, expense_count: 2 },
        { who_paid: 'Diana', total_amount: 250.0, expense_count: 1 },
      ],
      by_category: [
        { category: 'GROCERIES', total_amount: 800.0, expense_count: 4 },
        { category: 'RENT', total_amount: 700.0, expense_count: 4 },
      ],
    });
    vi.restoreAllMocks();
    vi.spyOn(api, 'fetchJointAccount').mockImplementation(async (id) => {
      const acc = mockJointAccounts.find((a) => a.id === id) || mockJointAccounts[0];
      jointAccount.set(acc);
      activeJointAccountId.set(acc.id);
      return acc;
    });
    vi.spyOn(api, 'fetchJointCategories').mockResolvedValue([]);
    vi.spyOn(api, 'fetchJointDeposits').mockResolvedValue([]);
    vi.spyOn(api, 'fetchJointExpectedCosts').mockResolvedValue([]);
    vi.spyOn(api, 'fetchJointCorrections').mockResolvedValue([]);
    vi.spyOn(api, 'fetchJointDashboard').mockResolvedValue(null);
    vi.spyOn(api, 'fetchJointMonthlyDeposits').mockResolvedValue([]);
  });

  it('renders top switcher with Everyone (exclusive), individual users, and joint accounts in AnalyticsSummary', async () => {
    render(AnalyticsSummary);

    expect(screen.getByText('Everyone')).toBeInTheDocument();
    expect(screen.getByText('👤 Alice')).toBeInTheDocument();
    expect(screen.getByText('👤 Bob')).toBeInTheDocument();
    expect(screen.getByText('👤 Charlie')).toBeInTheDocument();
    expect(screen.getByText('👤 Diana')).toBeInTheDocument();
    expect(screen.getByText(/Alice & Bob Joint/i)).toBeInTheDocument();
    expect(screen.getByText(/Charlie & Diana Joint/i)).toBeInTheDocument();
  });

  it('supports selecting users Alice, Bob and joint account albob together', async () => {
    const analyticsSpy = vi.spyOn(api, 'fetchAnalytics').mockResolvedValue({});
    const incomeSpy = vi.spyOn(api, 'fetchIncomeByPerson').mockResolvedValue([]);

    render(AnalyticsSummary);

    const aliceCheckbox = screen.getByDisplayValue('USER:Alice');
    const bobCheckbox = screen.getByDisplayValue('USER:Bob');
    const joint1Checkbox = screen.getByDisplayValue('JOINT:1');

    await fireEvent.click(aliceCheckbox);
    await fireEvent.click(bobCheckbox);
    await fireEvent.click(joint1Checkbox);

    await waitFor(() => {
      expect(analyticsSpy).toHaveBeenCalledWith(
        expect.any(String),
        expect.arrayContaining(['Alice', 'Bob'])
      );
    });
  });

  it('supports selecting users Charlie, Diana and joint account chadia together', async () => {
    const analyticsSpy = vi.spyOn(api, 'fetchAnalytics').mockResolvedValue({});
    render(AnalyticsSummary);

    const charlieCheckbox = screen.getByDisplayValue('USER:Charlie');
    const dianaCheckbox = screen.getByDisplayValue('USER:Diana');
    const joint2Checkbox = screen.getByDisplayValue('JOINT:2');

    await fireEvent.click(charlieCheckbox);
    await fireEvent.click(dianaCheckbox);
    await fireEvent.click(joint2Checkbox);

    await waitFor(() => {
      expect(analyticsSpy).toHaveBeenCalledWith(
        expect.any(String),
        expect.arrayContaining(['Charlie', 'Diana'])
      );
    });
  });

  it('supports selecting multiple cross-couple users such as Alice and Diana', async () => {
    const analyticsSpy = vi.spyOn(api, 'fetchAnalytics').mockResolvedValue({});
    render(AnalyticsSummary);

    const aliceCheckbox = screen.getByDisplayValue('USER:Alice');
    const dianaCheckbox = screen.getByDisplayValue('USER:Diana');

    await fireEvent.click(aliceCheckbox);
    await fireEvent.click(dianaCheckbox);

    await waitFor(() => {
      expect(analyticsSpy).toHaveBeenCalledWith(
        expect.any(String),
        expect.arrayContaining(['Alice', 'Diana'])
      );
    });
  });

  it('resets to all household when Everyone exclusive option is clicked', async () => {
    const analyticsSpy = vi.spyOn(api, 'fetchAnalytics').mockResolvedValue({});
    render(AnalyticsSummary);

    const aliceCheckbox = screen.getByDisplayValue('USER:Alice');
    await fireEvent.click(aliceCheckbox);

    const everyoneCheckbox = screen.getByDisplayValue('ALL');
    await fireEvent.click(everyoneCheckbox);

    await waitFor(() => {
      expect(analyticsSpy).toHaveBeenCalledWith(
        expect.any(String),
        null
      );
    });
  });

  it('displays member badges on project cards and supports member selection in ProjectsTab', async () => {
    render(ProjectsTab);

    expect(screen.getByText('Alice & Bob Vacation')).toBeInTheDocument();
    expect(screen.getByText('Charlie & Diana Car')).toBeInTheDocument();

    // Verify member chips appear on cards
    expect(screen.getAllByText('Alice').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Bob').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Charlie').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Diana').length).toBeGreaterThan(0);
  });

  it('switches between multiple joint accounts seamlessly in JointAccountTab', async () => {
    const fetchAccSpy = vi.spyOn(api, 'fetchJointAccount').mockResolvedValue(mockJointAccounts[1]);
    const fetchDashSpy = vi.spyOn(api, 'fetchJointDashboard').mockResolvedValue({
      month: '2026-08',
      balance_cents: 300000,
      expected_total_cents: 0,
      actual_total_cents: 0,
      total_deposits_cents: 0,
      safety_margin_pct: 15,
      target_deposit_cents: 0,
      categories: [],
      has_joint_account: true,
    });

    render(JointAccountTab);

    // Verify top account pills exist
    expect(screen.getAllByText('Alice & Bob Joint').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Charlie & Diana Joint').length).toBeGreaterThan(0);

    // Click to switch to Charlie & Diana's account
    const charlieBtn = screen.getAllByText('Charlie & Diana Joint')[0];
    await fireEvent.click(charlieBtn);

    await waitFor(() => {
      expect(fetchAccSpy).toHaveBeenCalledWith(2);
    });
  });

  it('allows selecting specific joint account in ExpenseForm when multiple joint accounts exist', async () => {
    const { default: ExpenseForm } = await import('../../lib/ExpenseForm.svelte');
    const { defaultCategory, splits, settlements, tags } = await import('../../lib/stores.js');
    splits.set([{ category: 'GROCERIES', allocations: [] }]);
    settlements.set([]);
    tags.set([]);
    defaultCategory.set('GROCERIES');

    const createSpy = vi.spyOn(api, 'createExpense').mockResolvedValue({});
    render(ExpenseForm);

    // Toggle Paid by Joint Account
    const jointCheckbox = screen.getByLabelText(/Joint Account/i);
    await fireEvent.click(jointCheckbox);

    // Verify selector dropdown for joint accounts is rendered
    const jointSelect = screen.getByLabelText(/Select Joint Account/i);
    expect(jointSelect).toBeInTheDocument();

    // Select Charlie & Diana Joint
    await fireEvent.change(jointSelect, { target: { value: 2 } });

    // Fill form and submit
    const nameInput = screen.getByLabelText(/Description/i);
    await fireEvent.input(nameInput, { target: { value: 'Couple Groceries' } });

    const amountInput = screen.getByLabelText(/Amount/i);
    await fireEvent.input(amountInput, { target: { value: '45.00' } });

    const submitBtn = screen.getByRole('button', { name: /Log Expense/i });
    await fireEvent.click(submitBtn);

    await waitFor(() => {
      expect(createSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          name: 'Couple Groceries',
          cost_cents: 4500,
          is_joint: true,
          joint_account_id: 2,
        }),
        expect.any(String)
      );
    });
  });

  it('allows selecting specific joint account in RecurringManager', async () => {
    const { default: RecurringManager } = await import('../../lib/RecurringManager.svelte');
    const { recurringExpenses, splits } = await import('../../lib/stores.js');
    splits.set([{ category: 'RENT', allocations: [] }]);
    recurringExpenses.set([
      {
        id: 10,
        name: 'Internet AlBob',
        cost_cents: 3000,
        who_paid: 'Alice',
        category: 'RENT',
        frequency: 'monthly',
        day_of_month: 1,
        is_joint: true,
        joint_account_id: 1,
        is_active: true,
      },
    ]);

    render(RecurringManager);

    // Verify badge shows Alice & Bob Joint
    expect(screen.getByText(/Alice & Bob Joint/i)).toBeInTheDocument();
  });

  it('renders quick presets in SplitManager for 4-user household', async () => {
    const { default: SplitManager } = await import('../../lib/SplitManager.svelte');
    const { splits, splitInputMode, jointCategories } = await import('../../lib/stores.js');
    splits.set([
      {
        category: 'ELECTRICITY',
        allocations: [
          { user_name: 'Alice', pct: 25 },
          { user_name: 'Bob', pct: 25 },
          { user_name: 'Charlie', pct: 25 },
          { user_name: 'Diana', pct: 25 },
        ],
      },
    ]);
    splitInputMode.set('inputs');
    jointCategories.set([]);

    render(SplitManager);

    // Verify presets exist
    expect(screen.getByText(/Even \(25%\)/i)).toBeInTheDocument();
    expect(screen.getByText(/Alice & Bob Joint \(Alice\+Bob\)/i)).toBeInTheDocument();
    expect(screen.getByText(/Charlie & Diana Joint \(Charlie\+Diana\)/i)).toBeInTheDocument();
  });

  it('filters Monthly Deposit Execution users to members of the selected joint account in JointAccountTab', async () => {
    const { default: JointAccountTab } = await import('../../lib/JointAccountTab.svelte');
    const { jointMonthlyDeposits } = await import('../../lib/stores.js');

    jointMonthlyDeposits.set([
      { user_name: 'Charlie', scheduled_cents: 115000, actual_cents: 115000, is_paid: true, paid_date: '2026-08-05', status: 'paid', account_id: 2 },
      { user_name: 'Diana', scheduled_cents: 115000, actual_cents: 0, is_paid: false, paid_date: null, status: 'pending', account_id: 2 },
    ]);

    render(JointAccountTab);

    // Switch to Account 2 (Charlie & Diana Joint)
    const acc2Btn = screen.getByText(/Charlie & Diana Joint/i);
    await fireEvent.click(acc2Btn);

    // Switch to deposits section
    const depositsTabBtn = screen.getByText(/Deposits/i);
    await fireEvent.click(depositsTabBtn);

    // Charlie & Diana should be in schedule and execution log
    await waitFor(() => {
      expect(screen.getAllByText('Charlie').length).toBeGreaterThanOrEqual(1);
      expect(screen.getAllByText('Diana').length).toBeGreaterThanOrEqual(1);
      // Alice & Bob should not be in the execution log or schedule for Account 2
      expect(screen.queryByText('Alice')).not.toBeInTheDocument();
      expect(screen.queryByText('Bob')).not.toBeInTheDocument();
    });
  });
});

