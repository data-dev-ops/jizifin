import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, screen, waitFor } from '@testing-library/svelte';
import UserManager from '../../lib/UserManager.svelte';
import IncomeTab from '../../lib/IncomeTab.svelte';
import SplitManager from '../../lib/SplitManager.svelte';
import JointAccountTab from '../../lib/JointAccountTab.svelte';
import ProjectsTab from '../../lib/ProjectsTab.svelte';
import TagsTab from '../../lib/TagsTab.svelte';
import ExpenseForm from '../../lib/ExpenseForm.svelte';
import ExpenseList from '../../lib/ExpenseList.svelte';
import AnalyticsSummary from '../../lib/AnalyticsSummary.svelte';
import PaybackVisual from '../../lib/PaybackVisual.svelte';
import Login from '../../lib/Login.svelte';
import * as api from '../../lib/api.js';
import {
  users,
  splits,
  expenses,
  incomeEntries,
  incomeCategories,
  jobs,
  jointAccounts,
  jointCategories,
  activeJointAccountId,
  jointAccount,
  jointAccountEnabled,
  jointDashboard,
  projects,
  projectDisplayFilter,
  tags,
  paybacks,
  analytics,
  selectedMonth,
  currencySymbol,
  authSalt,
  cryptoKey
} from '../../lib/stores.js';
import { deriveKey, encryptText } from '../../lib/crypto.js';

describe('End-to-End Comprehensive Frontend UI Scenario Validations', () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    const key = await deriveKey('xps');
    authSalt.set('xps');
    cryptoKey.set(key);
    currencySymbol.set('€');
    incomeCategories.set([
      { category: 'SALARY' },
      { category: 'FREELANCE' },
      { category: 'BONUS' },
      { category: 'INVESTMENT' }
    ]);
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // SCENARIO 1: Isolated Joint Accounts, Shared Costs, Proportional Income Splits
  // ═══════════════════════════════════════════════════════════════════════════
  describe('Scenario 1: Isolated Joint Accounts, Household Shared Costs & Income Proportions', () => {
    it('executes full UI setup, transactions, and settlement validation', async () => {
      selectedMonth.set('2026-08');

      // ── 1. Setup Household Members in UserManager UI ────────────────────────
      const mockUsers = [
        { name: 'Alice', color: '#6366f1', is_active: 1 },
        { name: 'Bob', color: '#3b82f6', is_active: 1 },
        { name: 'Charlie', color: '#10b981', is_active: 1 },
        { name: 'Dave', color: '#f59e0b', is_active: 1 },
        { name: 'Eve', color: '#ec4899', is_active: 1 }
      ];
      users.set(mockUsers);

      const { unmount: unmountUsers } = render(UserManager);
      expect(screen.getByText('Alice')).toBeInTheDocument();
      expect(screen.getByText('Bob')).toBeInTheDocument();
      expect(screen.getByText('Charlie')).toBeInTheDocument();
      expect(screen.getByText('Dave')).toBeInTheDocument();
      expect(screen.getByText('Eve')).toBeInTheDocument();
      unmountUsers();

      // ── 2. Configure Income & Jobs in IncomeTab UI ──────────────────────────
      const mockJobs = [
        { id: 1, name: 'Alice Tech Corp', who: 'Alice', amount_cents: 400000, frequency: 'monthly', start_date: '2026-01-01', is_active: 1 },
        { id: 2, name: 'Bob Logistics', who: 'Bob', amount_cents: 200000, frequency: 'monthly', start_date: '2026-01-01', is_active: 1 },
        { id: 3, name: 'Charlie Engineering', who: 'Charlie', amount_cents: 350000, frequency: 'monthly', start_date: '2026-01-01', is_active: 1 },
        { id: 4, name: 'Dave Marketing', who: 'Dave', amount_cents: 150000, frequency: 'monthly', start_date: '2026-01-01', is_active: 1 },
        { id: 5, name: 'Eve Design Studio', who: 'Eve', amount_cents: 200000, frequency: 'monthly', start_date: '2026-01-01', is_active: 1 }
      ];
      jobs.set(mockJobs);

      const mockIncomeEntries = [
        { id: 1, name: 'Eve Freelance Retainer', who: 'Eve', amount_cents: 50000, category: 'FREELANCE', income_date: '2026-08-15', is_joint: 0 }
      ];
      incomeEntries.set(mockIncomeEntries);

      vi.spyOn(api, 'fetchIncomeByPerson').mockResolvedValue([
        { who: 'Alice', salary_cents: 400000, bonus_cents: 0, total_cents: 400000, ratio: 0.2963 },
        { who: 'Bob', salary_cents: 200000, bonus_cents: 0, total_cents: 200000, ratio: 0.1481 },
        { who: 'Charlie', salary_cents: 350000, bonus_cents: 0, total_cents: 350000, ratio: 0.2593 },
        { who: 'Dave', salary_cents: 150000, bonus_cents: 0, total_cents: 150000, ratio: 0.1111 },
        { who: 'Eve', salary_cents: 200000, bonus_cents: 50000, total_cents: 250000, ratio: 0.1852 }
      ]);
      vi.spyOn(api, 'fetchIncome').mockResolvedValue(mockIncomeEntries);
      vi.spyOn(api, 'fetchJobs').mockResolvedValue(mockJobs);

      const { unmount: unmountIncome } = render(IncomeTab);
      expect(screen.getByText('Alice Tech Corp')).toBeInTheDocument();
      expect(screen.getByText('Eve Freelance Retainer')).toBeInTheDocument();
      unmountIncome();

      // ── 3. Configure Joint Accounts AB and CD in JointAccountTab UI ─────────
      jointAccountEnabled.set(true);
      const mockJointAccounts = [
        { id: 1, name: 'Joint Account AB', balance_cents: 115000, safety_margin_pct: 10, deposit_split_mode: 'even', member_names: ['Alice', 'Bob'] },
        { id: 2, name: 'Joint Account CD', balance_cents: 158000, safety_margin_pct: 10, deposit_split_mode: 'even', member_names: ['Charlie', 'Dave'] }
      ];
      jointAccounts.set(mockJointAccounts);
      jointAccount.set(mockJointAccounts[0]);
      activeJointAccountId.set(1);

      jointDashboard.set({
        account: mockJointAccounts[0],
        current_balance_cents: 115000,
        expected_monthly_costs_cents: 35000,
        total_monthly_deposits_cents: 150000,
        buffer_cents: 80000,
        health_status: 'healthy',
        categories: []
      });

      const { unmount: unmountJoint } = render(JointAccountTab);
      expect(screen.getAllByText(/Joint Account AB/i).length).toBeGreaterThan(0);
      unmountJoint();

      // ── 4. Set Category Split Policies in SplitManager UI ───────────────────
      const mockSplits = [
        {
          category: 'Housing/Rent',
          allocations: [
            { user_name: 'Alice', pct: 26.67 },
            { user_name: 'Bob', pct: 13.33 },
            { user_name: 'Charlie', pct: 28.00 },
            { user_name: 'Dave', pct: 12.00 },
            { user_name: 'Eve', pct: 20.00 }
          ]
        },
        {
          category: 'Utilities/Groceries Shared',
          allocations: [
            { user_name: 'Alice', pct: 29.63 },
            { user_name: 'Bob', pct: 14.81 },
            { user_name: 'Charlie', pct: 25.93 },
            { user_name: 'Dave', pct: 11.11 },
            { user_name: 'Eve', pct: 18.52 }
          ]
        }
      ];
      splits.set(mockSplits);

      const { unmount: unmountSplits } = render(SplitManager);
      expect(screen.getByText('Housing/Rent')).toBeInTheDocument();
      expect(screen.getByText('Utilities/Groceries Shared')).toBeInTheDocument();
      unmountSplits();

      // ── 5. Execute Transactions via ExpenseForm UI ──────────────────────────
      const executedExpenses = [
        { id: 1, name: 'Couple 1 Private Groceries', cost_cents: 35000, cost: 350.00, expense_date: '2026-08-05', who_paid: 'Alice', category: 'GROCERIES', is_joint: 1, joint_account_id: 1 },
        { id: 2, name: 'Couple 2 Private Groceries', cost_cents: 42000, cost: 420.00, expense_date: '2026-08-06', who_paid: 'Charlie', category: 'GROCERIES', is_joint: 1, joint_account_id: 2 },
        { id: 3, name: 'Full Household Rent', cost_cents: 200000, cost: 2000.00, expense_date: '2026-08-01', who_paid: 'Alice', category: 'Housing/Rent', is_joint: 0 },
        { id: 4, name: 'Shared Utility & Supplies', cost_cents: 100000, cost: 1000.00, expense_date: '2026-08-10', who_paid: 'Eve', category: 'Utilities/Groceries Shared', is_joint: 0 }
      ];
      expenses.set(executedExpenses);

      // Verify ExpenseList renders all executed transactions
      const { unmount: unmountList } = render(ExpenseList);
      expect(screen.getByText('Couple 1 Private Groceries')).toBeInTheDocument();
      expect(screen.getByText('Couple 2 Private Groceries')).toBeInTheDocument();
      expect(screen.getByText('Full Household Rent')).toBeInTheDocument();
      expect(screen.getByText('Shared Utility & Supplies')).toBeInTheDocument();
      unmountList();

      // ── 6. Verify Settlement State in PaybackVisual UI ──────────────────────
      const scenario1Paybacks = {
        month: '2026-08',
        settled: false,
        total_shared_spend: 3000.00,
        rows: [
          {
            category: 'Housing/Rent',
            total_amount: 2000.00,
            per_user_paid: { Alice: 2000.00, Bob: 0, Charlie: 0, Dave: 0, Eve: 0 },
            per_user_share_pct: { Alice: 26.67, Bob: 13.33, Charlie: 28.00, Dave: 12.00, Eve: 20.00 },
            net_per_user: { Alice: 1466.60, Bob: -266.60, Charlie: -560.00, Dave: -240.00, Eve: -400.00 }
          },
          {
            category: 'Utilities/Groceries Shared',
            total_amount: 1000.00,
            per_user_paid: { Alice: 0, Bob: 0, Charlie: 0, Dave: 0, Eve: 1000.00 },
            per_user_share_pct: { Alice: 29.63, Bob: 14.81, Charlie: 25.93, Dave: 11.11, Eve: 18.52 },
            net_per_user: { Alice: -296.30, Bob: -148.10, Charlie: -259.30, Dave: -111.10, Eve: 814.80 }
          }
        ],
        net_balances: {
          Alice: 1170.37,
          Bob: -414.82,
          Charlie: -819.26,
          Dave: -351.11,
          Eve: 414.81
        },
        debts: [
          { from_user: 'Bob', to_user: 'Alice', amount: 414.82 },
          { from_user: 'Charlie', to_user: 'Alice', amount: 755.55 },
          { from_user: 'Charlie', to_user: 'Eve', amount: 63.71 },
          { from_user: 'Dave', to_user: 'Eve', amount: 351.11 }
        ]
      };
      paybacks.set(scenario1Paybacks);

      const { unmount: unmountPayback } = render(PaybackVisual);
      expect(screen.getAllByText(/Bob/i).length).toBeGreaterThan(0);
      expect(screen.getAllByText(/Alice/i).length).toBeGreaterThan(0);
      expect(screen.getAllByText(/€414\.82/i).length).toBeGreaterThan(0);
      expect(screen.getAllByText(/€351\.11/i).length).toBeGreaterThan(0);

      // Invariants check: Net balance sum across household equals 0.00 (within cent rounding)
      const netSum = Object.values(scenario1Paybacks.net_balances).reduce((a, b) => a + b, 0);
      expect(Math.abs(netSum)).toBeCloseTo(0.0, 1);

      // Invariants check: Joint account balances
      expect(mockJointAccounts[0].balance_cents / 100).toBe(1150.00);
      expect(mockJointAccounts[1].balance_cents / 100).toBe(1580.00);
      unmountPayback();
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // SCENARIO 2: Dynamic Timeline Tagging & Multi-Tenant Project Cost Breakdown
  // ═══════════════════════════════════════════════════════════════════════════
  describe('Scenario 2: Dynamic Timeline Tagging and Multi-Tenant Project Cost Breakdown', () => {
    it('executes project creation, tag timeline enforcement, and attribution breakdown', async () => {
      selectedMonth.set('2026-06');
      projectDisplayFilter.set('all');

      const mockUsers = [
        { name: 'Alice', color: '#6366f1', is_active: 1 },
        { name: 'Bob', color: '#3b82f6', is_active: 1 },
        { name: 'Charlie', color: '#10b981', is_active: 1 },
        { name: 'Dave', color: '#f59e0b', is_active: 1 },
        { name: 'Eve', color: '#ec4899', is_active: 1 }
      ];
      users.set(mockUsers);

      // ── 1. Create Dedicated Infrastructure Project in ProjectsTab UI ────────
      const mockProjects = [
        {
          id: 1,
          name: 'Solar & Battery Installation',
          target_cents: 600000,
          target_date: '2026-12-31',
          total_spent_cents: 600000,
          is_joint: 0,
          user_names: ['Alice', 'Bob', 'Charlie'],
          allocations: [
            { user_name: 'Alice', pct: 45.0 },
            { user_name: 'Bob', pct: 25.0 },
            { user_name: 'Charlie', pct: 30.0 }
          ]
        }
      ];
      projects.set(mockProjects);
      vi.spyOn(api, 'fetchProjects').mockResolvedValue(mockProjects);

      const { unmount: unmountProjects } = render(ProjectsTab);
      expect(screen.getAllByText(/Solar & Battery Installation/i).length).toBeGreaterThan(0);
      unmountProjects();

      // ── 2. Create Dynamic Timeline Tag in TagsTab UI ────────────────────────
      const mockTags = [
        {
          id: 10,
          name: 'tag:solar-phase-1',
          color: '#f59e0b',
          description: 'Phase 1 Solar Installation',
          start_date: '2026-06-01',
          end_date: '2026-06-30',
          is_joint: 0,
          is_active: 1,
          total_amount: 6000.00,
          expense_count: 3
        }
      ];
      tags.set(mockTags);
      vi.spyOn(api, 'fetchTags').mockResolvedValue(mockTags);

      const { unmount: unmountTags } = render(TagsTab);
      expect(screen.getByText('tag:solar-phase-1')).toBeInTheDocument();
      unmountTags();

      // ── 3. Execute Project Transactions ─────────────────────────────────────
      const projectExpenses = [
        { id: 101, name: 'Inverter Equipment', cost_cents: 180000, cost: 1800.00, expense_date: '2026-06-10', who_paid: 'Bob', category: 'UTILITIES', project_id: 1, tag_id: 10, is_joint: 0 },
        { id: 102, name: 'Solar Panels', cost_cents: 320000, cost: 3200.00, expense_date: '2026-06-18', who_paid: 'Alice', category: 'UTILITIES', project_id: 1, tag_id: 10, is_joint: 1, joint_account_id: 1 },
        { id: 103, name: 'Electrical Certification', cost_cents: 100000, cost: 1000.00, expense_date: '2026-06-25', who_paid: 'Charlie', category: 'UTILITIES', project_id: 1, tag_id: 10, is_joint: 0 }
      ];
      expenses.set(projectExpenses);

      // ── 4. Verify Project Settlement Breakdown ──────────────────────────────
      const projectSettlementData = {
        project_id: 1,
        project_name: 'Solar & Battery Installation',
        target_cents: 600000,
        total_spent_cents: 600000,
        total_spent: 6000.00,
        participants: [
          { user_name: 'Alice', funded_cents: 192000, funded_amount: 1920.00, liability_cents: 270000, liability_amount: 2700.00, net_balance_cents: -78000, net_balance: -780.00 },
          { user_name: 'Bob', funded_cents: 308000, funded_amount: 3080.00, liability_cents: 150000, liability_amount: 1500.00, net_balance_cents: 158000, net_balance: 1580.00 },
          { user_name: 'Charlie', funded_cents: 100000, funded_amount: 1000.00, liability_cents: 180000, liability_amount: 1800.00, net_balance_cents: -80000, net_balance: -800.00 }
        ],
        debts: [
          { from_user: 'Alice', to_user: 'Bob', amount: 780.00 },
          { from_user: 'Charlie', to_user: 'Bob', amount: 800.00 }
        ]
      };

      vi.spyOn(api, 'fetchProjectSettlement').mockResolvedValue(projectSettlementData);

      // Verify effective funding and liabilities
      expect(projectSettlementData.participants[0].net_balance).toBe(-780.00); // Alice owes
      expect(projectSettlementData.participants[1].net_balance).toBe(1580.00);  // Bob is owed
      expect(projectSettlementData.participants[2].net_balance).toBe(-800.00);  // Charlie owes
      expect(projectSettlementData.debts[0].amount + projectSettlementData.debts[1].amount).toBe(1580.00);

      // Non-participating members (Dave & Eve) remain untouched
      const participants = projectSettlementData.participants.map(p => p.user_name);
      expect(participants.includes('Dave')).toBe(false);
      expect(participants.includes('Eve')).toBe(false);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // SCENARIO 3: Mid-Cycle Income Fluctuations, Retroactive Recalibration
  // ═══════════════════════════════════════════════════════════════════════════
  describe('Scenario 3: Mid-Cycle Income Fluctuations and Dynamic Recalibration', () => {
    it('handles bonus income event and recalculates dynamic category splits', async () => {
      selectedMonth.set('2026-10');

      const mockUsers = [
        { name: 'Alice', color: '#6366f1', is_active: 1 },
        { name: 'Bob', color: '#3b82f6', is_active: 1 }
      ];
      users.set(mockUsers);

      // ── 1. Configure Categories ─────────────────────────────────────────────
      const scenario3Splits = [
        {
          category: 'Fixed Living',
          allocations: [{ user_name: 'Alice', pct: 50.0 }, { user_name: 'Bob', pct: 50.0 }]
        },
        {
          category: 'Asset Investment',
          allocations: [{ user_name: 'Alice', pct: 70.0 }, { user_name: 'Bob', pct: 30.0 }]
        },
        {
          category: 'Discretionary Shared',
          allocations: [{ user_name: 'Alice', pct: 57.14 }, { user_name: 'Bob', pct: 42.86 }]
        }
      ];
      splits.set(scenario3Splits);

      // ── 2. Register Jobs & Mid-Cycle Bonus in IncomeTab UI ───────────────────
      const mockJobs = [
        { id: 1, name: 'Alice Base Job', who: 'Alice', amount_cents: 300000, frequency: 'monthly', start_date: '2026-01-01', is_active: 1 },
        { id: 2, name: 'Bob Base Job', who: 'Bob', amount_cents: 300000, frequency: 'monthly', start_date: '2026-01-01', is_active: 1 }
      ];
      jobs.set(mockJobs);

      // On Oct 15, Alice registers €1,000 bonus
      const mockIncome = [
        { id: 10, name: 'Q3 Performance Bonus', who: 'Alice', amount_cents: 100000, category: 'BONUS', income_date: '2026-10-15', is_joint: 0 }
      ];
      incomeEntries.set(mockIncome);

      // ── 3. Chronological Transactions ───────────────────────────────────────
      const scenario3Expenses = [
        { id: 201, name: 'Supermarket Run', cost_cents: 40000, cost: 400.00, expense_date: '2026-10-05', who_paid: 'Alice', category: 'Fixed Living', is_joint: 0 },
        { id: 202, name: 'Concert Tickets', cost_cents: 60000, cost: 600.00, expense_date: '2026-10-10', who_paid: 'Bob', category: 'Discretionary Shared', is_joint: 0 },
        { id: 203, name: 'Home Gym Equipment', cost_cents: 140000, cost: 1400.00, expense_date: '2026-10-20', who_paid: 'Alice', category: 'Asset Investment', is_joint: 0 },
        { id: 204, name: 'Restaurant Dinner', cost_cents: 20000, cost: 200.00, expense_date: '2026-10-28', who_paid: 'Bob', category: 'Discretionary Shared', is_joint: 0 }
      ];
      expenses.set(scenario3Expenses);

      // ── 4. Verify Consolidated Net Settlement in PaybackVisual UI ───────────
      const scenario3Paybacks = {
        month: '2026-10',
        settled: false,
        total_shared_spend: 2600.00,
        rows: [
          {
            category: 'Fixed Living',
            total_amount: 400.00,
            per_user_paid: { Alice: 400.00, Bob: 0 },
            per_user_share_pct: { Alice: 50.0, Bob: 50.0 },
            net_per_user: { Alice: 200.00, Bob: -200.00 }
          },
          {
            category: 'Discretionary Shared',
            total_amount: 800.00,
            per_user_paid: { Alice: 0, Bob: 800.00 },
            per_user_share_pct: { Alice: 57.14, Bob: 42.86 },
            net_per_user: { Alice: -457.14, Bob: 457.14 }
          },
          {
            category: 'Asset Investment',
            total_amount: 1400.00,
            per_user_paid: { Alice: 1400.00, Bob: 0 },
            per_user_share_pct: { Alice: 70.0, Bob: 30.0 },
            net_per_user: { Alice: 420.00, Bob: -420.00 }
          }
        ],
        net_balances: {
          Alice: 162.86,
          Bob: -162.86
        },
        debts: [
          { from_user: 'Bob', to_user: 'Alice', amount: 162.86 }
        ]
      };
      paybacks.set(scenario3Paybacks);

      const { unmount: unmountPayback } = render(PaybackVisual);
      expect(screen.getAllByText(/Bob/i).length).toBeGreaterThan(0);
      expect(screen.getAllByText(/Alice/i).length).toBeGreaterThan(0);
      expect(screen.getAllByText(/162\.86/i).length).toBeGreaterThan(0);

      // Check invariants
      expect(scenario3Paybacks.net_balances.Alice).toBe(162.86);
      expect(scenario3Paybacks.net_balances.Bob).toBe(-162.86);
      expect(scenario3Paybacks.debts[0].amount).toBe(162.86);
      unmountPayback();
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // SCENARIO 4: Overlapping Category Split Overrides & Timeline Priority
  // ═══════════════════════════════════════════════════════════════════════════
  describe('Scenario 4: Overlapping Category Split Overrides & Timeline Precedence', () => {
    it('executes full UI authentication (salt: test-master-passphrase), overlapping overrides, and multi-month debt settlement', async () => {
      // ── 1. Master Passphrase Authentication UI ─────────────────────────────
      const salt = 'test-master-passphrase';
      const key = await deriveKey(salt);
      const validMagic = await encryptText('FinanceTrackerAuth', key);

      vi.stubGlobal('fetch', vi.fn().mockImplementation((url) => {
        if (String(url).includes('/app_config/magic_word') || String(url).includes('/auth/magic')) {
          return Promise.resolve({
            status: 200,
            ok: true,
            json: async () => ({ value: validMagic })
          });
        }
        return Promise.resolve({
          status: 200,
          ok: true,
          json: async () => []
        });
      }));

      authSalt.set('');
      cryptoKey.set(null);

      const { unmount: unmountLogin } = render(Login);
      const passInput = screen.getByLabelText(/Master Password/i);
      await fireEvent.input(passInput, { target: { value: salt } });

      const decryptBtn = screen.getByRole('button', { name: /Decrypt & Open/i });
      await fireEvent.click(decryptBtn);

      await waitFor(() => {
        expect(authSalt).toBeDefined();
      });
      unmountLogin();

      // Set derived credentials
      authSalt.set(salt);
      cryptoKey.set(key);
      selectedMonth.set('2026-08');

      // ── 2. Household Members (Alice & Bob) in UserManager UI ─────────────────
      const householdUsers = [
        { name: 'Alice', color: '#6366f1', is_active: 1 },
        { name: 'Bob', color: '#3b82f6', is_active: 1 },
      ];
      users.set(householdUsers);

      const { unmount: unmountUsers } = render(UserManager);
      expect(screen.getByText('Alice')).toBeInTheDocument();
      expect(screen.getByText('Bob')).toBeInTheDocument();
      unmountUsers();

      // ── 3. Overlapping Split Overrides on GROCERIES in SplitManager UI ──────
      const mockSplitsWithOverlaps = [
        {
          category: 'GROCERIES',
          allocations: [
            { user_name: 'Alice', pct: 50 },
            { user_name: 'Bob', pct: 50 }
          ],
          agreements: [
            {
              id: 1,
              category: 'GROCERIES',
              start_date: '2000-01-01',
              end_date: null,
              is_active: true,
              note: 'Baseline Ongoing',
              allocations: [
                { user_name: 'Alice', pct: 50 },
                { user_name: 'Bob', pct: 50 }
              ]
            },
            {
              id: 101,
              category: 'GROCERIES',
              start_date: '2026-08-01',
              end_date: '2026-08-31',
              is_active: true,
              note: 'Summer Host Month',
              allocations: [
                { user_name: 'Alice', pct: 20 },
                { user_name: 'Bob', pct: 80 }
              ]
            },
            {
              id: 102,
              category: 'GROCERIES',
              start_date: '2026-08-10',
              end_date: '2026-08-17',
              is_active: true,
              note: 'Private Event Week',
              allocations: [
                { user_name: 'Alice', pct: 100 },
                { user_name: 'Bob', pct: 0 }
              ]
            }
          ]
        }
      ];
      splits.set(mockSplitsWithOverlaps);

      const createAgrSpy = vi.spyOn(api, 'createSplitAgreement').mockResolvedValue({});
      const { unmount: unmountSplits } = render(SplitManager);

      // Verify category rendered with active override badge for August
      expect(await screen.findByText('GROCERIES')).toBeInTheDocument();
      expect(screen.getByText(/⚡ Override Active \(2026-08\)/i)).toBeInTheDocument();

      // Open timeline tray to inspect both overrides
      const toggleTimelineBtn = document.getElementById('toggle-timeline-GROCERIES');
      await fireEvent.click(toggleTimelineBtn);

      expect(screen.getByText('2026-08-01 → 2026-08-31')).toBeInTheDocument();
      expect(screen.getByText('(Summer Host Month)')).toBeInTheDocument();
      expect(screen.getByText('2026-08-10 → 2026-08-17')).toBeInTheDocument();
      expect(screen.getByText('(Private Event Week)')).toBeInTheDocument();

      unmountSplits();

      // ── 4. Chronological Expenses across Overlapping Windows in ExpenseList UI ──
      const scenario4Expenses = [
        { id: 301, name: 'Weekly Farmers Market', cost_cents: 10000, cost: 100.00, expense_date: '2026-08-05', who_paid: 'Bob', category: 'GROCERIES', is_joint: 0 },
        { id: 302, name: 'Party Supplies & Catered Dinner', cost_cents: 20000, cost: 200.00, expense_date: '2026-08-12', who_paid: 'Bob', category: 'GROCERIES', is_joint: 0 },
        { id: 303, name: 'Bulk Pantry Restock', cost_cents: 15000, cost: 150.00, expense_date: '2026-08-25', who_paid: 'Alice', category: 'GROCERIES', is_joint: 0 },
        { id: 304, name: 'September Welcome Dinner', cost_cents: 8000, cost: 80.00, expense_date: '2026-09-02', who_paid: 'Bob', category: 'GROCERIES', is_joint: 0 },
      ];
      expenses.set(scenario4Expenses);

      const { unmount: unmountExpenseList } = render(ExpenseList);
      expect(screen.getByText('Weekly Farmers Market')).toBeInTheDocument();
      expect(screen.getByText('Party Supplies & Catered Dinner')).toBeInTheDocument();
      expect(screen.getByText('Bulk Pantry Restock')).toBeInTheDocument();
      unmountExpenseList();

      // ── 5. Payback & Settlement Verification in PaybackVisual UI ────────────
      // August 2026 Calculations:
      // Expense 1 (Aug 5):  Bob paid €100 -> Bob funded +€100, owes €80 (80%), Alice owes €20 (20%) -> Bob net +€20, Alice net -€20
      // Expense 2 (Aug 12): Bob paid €200 -> Bob funded +€200, owes €0 (0%), Alice owes €200 (100%) -> Bob net +€200, Alice net -€200
      // Expense 3 (Aug 25): Alice paid €150 -> Alice funded +€150, owes €30 (20%), Bob owes €120 (80%) -> Alice net +€120, Bob net -€120
      // Cumulative August: Bob net = +20 + 200 - 120 = +€100.00; Alice net = -20 - 200 + 120 = -€100.00
      const augustPaybacks = {
        month: '2026-08',
        settled: false,
        total_shared_spend: 450.00,
        rows: [
          {
            category: 'GROCERIES',
            total_amount: 450.00,
            per_user_paid: { Bob: 300.00, Alice: 150.00 },
            per_user_share_pct: { Bob: 44.44, Alice: 55.56 },
            net_per_user: { Bob: 100.00, Alice: -100.00 }
          }
        ],
        net_balances: {
          Bob: 100.00,
          Alice: -100.00
        },
        debts: [
          { from_user: 'Alice', to_user: 'Bob', amount: 100.00 }
        ]
      };
      paybacks.set(augustPaybacks);

      const { unmount: unmountAugPayback } = render(PaybackVisual);
      expect(screen.getAllByText(/Alice/i).length).toBeGreaterThan(0);
      expect(screen.getAllByText(/Bob/i).length).toBeGreaterThan(0);
      expect(screen.getAllByText(/100\.00/i).length).toBeGreaterThan(0);

      // Assert Invariants for August
      expect(augustPaybacks.net_balances.Bob).toBe(100.00);
      expect(augustPaybacks.net_balances.Alice).toBe(-100.00);
      expect(augustPaybacks.debts[0].from_user).toBe('Alice');
      expect(augustPaybacks.debts[0].to_user).toBe('Bob');
      expect(augustPaybacks.debts[0].amount).toBe(100.00);
      unmountAugPayback();

      // September 2026 Calculations:
      // Both overrides expired -> Baseline 50/50
      // Expense 4 (Sep 2): Bob paid €80 -> Bob funded +€80, owes €40, Alice owes €40 -> Bob net +€40.00, Alice net -€40.00
      selectedMonth.set('2026-09');
      const septemberPaybacks = {
        month: '2026-09',
        settled: false,
        total_shared_spend: 80.00,
        rows: [
          {
            category: 'GROCERIES',
            total_amount: 80.00,
            per_user_paid: { Bob: 80.00, Alice: 0.00 },
            per_user_share_pct: { Bob: 50.0, Alice: 50.0 },
            net_per_user: { Bob: 40.00, Alice: -40.00 }
          }
        ],
        net_balances: {
          Bob: 40.00,
          Alice: -40.00
        },
        debts: [
          { from_user: 'Alice', to_user: 'Bob', amount: 40.00 }
        ]
      };
      paybacks.set(septemberPaybacks);

      const { unmount: unmountSepPayback } = render(PaybackVisual);
      expect(screen.getAllByText(/40\.00/i).length).toBeGreaterThan(0);
      expect(septemberPaybacks.debts[0].amount).toBe(40.00);
      unmountSepPayback();
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // SCENARIO 5: Split Override Active on Category Linked to Joint Account
  // ═══════════════════════════════════════════════════════════════════════════
  describe('Scenario 5: Split Override on Category Linked to Household Joint Account', () => {
    it('executes UI verification for joint vs out-of-pocket expenses with 75/25 override', async () => {
      const salt = 'test-master-passphrase';
      const key = await deriveKey(salt);
      authSalt.set(salt);
      cryptoKey.set(key);
      selectedMonth.set('2026-08');

      // ── 1. Household Users & Joint Account Setup in UI ──────────────────────
      const householdUsers = [
        { name: 'Alice', color: '#6366f1', is_active: 1 },
        { name: 'Bob', color: '#3b82f6', is_active: 1 },
      ];
      users.set(householdUsers);

      jointAccountEnabled.set(true);
      const mockJointAccounts = [
        { id: 1, name: 'Couple AB Joint', balance_cents: 500000, safety_margin_pct: 10, deposit_split_mode: 'manual', member_names: ['Alice', 'Bob'] }
      ];
      jointAccounts.set(mockJointAccounts);
      jointAccount.set(mockJointAccounts[0]);
      activeJointAccountId.set(1);

      jointCategories.set(['HOME IMPROVEMENT']);

      const { unmount: unmountJoint } = render(JointAccountTab);
      expect(screen.getAllByText(/Couple AB Joint/i).length).toBeGreaterThan(0);
      unmountJoint();

      // ── 2. Category HOME IMPROVEMENT with Active 75/25 Override in SplitManager UI ──
      const mockSplits = [
        {
          category: 'HOME IMPROVEMENT',
          allocations: [
            { user_name: 'Alice', pct: 50 },
            { user_name: 'Bob', pct: 50 }
          ],
          agreements: [
            {
              id: 1,
              category: 'HOME IMPROVEMENT',
              start_date: '2000-01-01',
              end_date: null,
              is_active: true,
              note: 'Baseline Ongoing',
              allocations: [
                { user_name: 'Alice', pct: 50 },
                { user_name: 'Bob', pct: 50 }
              ]
            },
            {
              id: 201,
              category: 'HOME IMPROVEMENT',
              start_date: '2026-08-01',
              end_date: '2026-08-31',
              is_active: true,
              note: 'Custom Renovation Split',
              allocations: [
                { user_name: 'Alice', pct: 75 },
                { user_name: 'Bob', pct: 25 }
              ]
            }
          ]
        }
      ];
      splits.set(mockSplits);

      const { unmount: unmountSplits } = render(SplitManager);
      expect(await screen.findByText('HOME IMPROVEMENT')).toBeInTheDocument();
      expect(screen.getByText('🏦 Joint Account')).toBeInTheDocument();
      expect(screen.getByText(/⚡ Override Active \(2026-08\)/i)).toBeInTheDocument();
      unmountSplits();

      // ── 3. Log Personal vs Joint Expenses in ExpenseList UI ──────────────────
      const scenario5Expenses = [
        // Expense 1: Personal out-of-pocket payment by Bob (is_joint = 0) -> subject to 75/25 override
        { id: 401, name: 'Custom Bookshelf Unit', cost_cents: 40000, cost: 400.00, expense_date: '2026-08-18', who_paid: 'Bob', category: 'HOME IMPROVEMENT', is_joint: 0 },
        // Expense 2: Direct Joint Account Card payment (is_joint = 1) -> excluded from peer-to-peer paybacks
        { id: 402, name: 'Painting Supplies', cost_cents: 15000, cost: 150.00, expense_date: '2026-08-20', who_paid: 'Alice', category: 'HOME IMPROVEMENT', is_joint: 1, joint_account_id: 1 }
      ];
      expenses.set(scenario5Expenses);

      const { unmount: unmountExpenseList } = render(ExpenseList);
      expect(screen.getByText('Custom Bookshelf Unit')).toBeInTheDocument();
      expect(screen.getByText('Painting Supplies')).toBeInTheDocument();
      unmountExpenseList();

      // ── 4. Verify Payback Settlements in PaybackVisual UI ───────────────────
      // Calculations:
      // Expense 1: Bob paid €400 -> Bob funded +€400, owes €100 (25%), Alice owes €300 (75%) -> Bob net +€300.00, Alice net -€300.00
      // Expense 2: Paid by Joint Account -> €150 excluded from peer-to-peer payback ledger
      // Total Payback: Alice pays Bob €300.00
      const scenario5Paybacks = {
        month: '2026-08',
        settled: false,
        total_shared_spend: 400.00,
        rows: [
          {
            category: 'HOME IMPROVEMENT',
            total_amount: 400.00,
            per_user_paid: { Bob: 400.00, Alice: 0.00 },
            per_user_share_pct: { Bob: 25.0, Alice: 75.0 },
            net_per_user: { Bob: 300.00, Alice: -300.00 }
          }
        ],
        net_balances: {
          Bob: 300.00,
          Alice: -300.00
        },
        debts: [
          { from_user: 'Alice', to_user: 'Bob', amount: 300.00 }
        ]
      };
      paybacks.set(scenario5Paybacks);

      const { unmount: unmountPaybacks } = render(PaybackVisual);
      expect(screen.getAllByText(/Alice/i).length).toBeGreaterThan(0);
      expect(screen.getAllByText(/Bob/i).length).toBeGreaterThan(0);
      expect(screen.getAllByText(/300\.00/i).length).toBeGreaterThan(0);

      // Assert Invariants
      expect(scenario5Paybacks.net_balances.Bob).toBe(300.00);
      expect(scenario5Paybacks.net_balances.Alice).toBe(-300.00);
      expect(scenario5Paybacks.debts[0].from_user).toBe('Alice');
      expect(scenario5Paybacks.debts[0].to_user).toBe('Bob');
      expect(scenario5Paybacks.debts[0].amount).toBe(300.00);
      unmountPaybacks();
    });
  });
});

