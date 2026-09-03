# Getting Started & How-To Guide

A practical, step-by-step guide to configuring and operating Jizifin for multi-user household finance management.

---

## Overview

Jizifin operates with client-side zero-knowledge encryption: your master passphrase derives a 256-bit AES-GCM key in browser memory. Sensitive text (names, descriptions, notes, categories) is encrypted before reaching the server.

Follow the 8 setup steps below to configure your household.

---

## Step 1: Initialize Setup and Set Master Passphrase

1. Open the application URL in your web browser.
2. If launching for the first time, you will be prompted to create your **Household Master Passphrase**.
3. Choose a strong passphrase known to household members.
   - The browser uses PBKDF2 with 100,000 SHA-256 iterations to derive the AES-GCM key.
   - A verification magic token is written to the database to validate future logins.
4. If returning to an existing instance, enter the master passphrase on the **Login** screen.

---

## Step 2: Add Household Members and Customize Colors

1. Navigate to **Settings** (or open the **Users** management section).
2. Click **Add User** and enter:
   - **Name**: e.g., "Alice" or "Bob".
   - **Color**: Select a unique color swatch or enter a hex color code (e.g. `#6366f1`). This color will identify the user in charts, debt breakdowns, and expense ledgers.
3. Click **Save Member**.
4. Repeat for each household member.

---

## Step 3: Configure Categories and Default Split Allocations

1. Navigate to the **Splits** tab.
2. Review the default categories (e.g., *Groceries*, *Rent*, *Utilities*, *Dining Out*).
3. For each category, set the default percentage each member pays:
   - **50% / 50%**: Even split between two members.
   - **Proportional (e.g. 60% / 40%)**: Adjusted according to income or agreed terms.
   - **Constraint**: Total allocations must sum to exactly **100.0%**.
4. Click **Update Allocations** or use the **Batch Apply Rules** modal to apply a split rule across multiple categories at once.

---

## Step 4: Set Up Household Joint Account

1. Navigate to the **Joint Account** tab.
2. Click **Set Up Household Joint Account** if starting fresh.
3. Configure the joint pool parameters:
   - **Joint Account Name**: e.g., "Household Shared Account".
   - **Safety Margin Buffer (%)**: e.g., `10%` to `15%`. Adds a safety cushion above expected monthly expenses.
   - **Deposit Split Mode**:
     - `Salary Proportional`: Calculates each member's monthly contribution based on active employment job income.
     - `Even Split`: Divides total monthly expected expenses evenly.
     - `Manual`: Allows setting fixed monthly transfer amounts per member.
4. Map categories that are paid directly from the joint account (e.g., *Rent*, *Electricity*, *Groceries*).
5. Set the **Expected Monthly Cost** for each joint category.
6. When members transfer funds into the bank account or withdraw funds, record a **Balance Correction** (positive for top-up, negative for withdrawal).

---

## Step 5: Log Income Streams and Employment Contracts

1. Navigate to the **Income** tab.
2. **Add Recurring Employment Jobs**:
   - Enter Job Name (e.g., "Software Engineer", "Consulting").
   - Select the household member (**Who**).
   - Enter net pay rate and select frequency: `Monthly`, `Weekly`, `Bi-weekly`, or `Annual`.
   - Set the `start_date` and optional `end_date`.
   - Jizifin automatically computes normalized monthly equivalents.
3. **One-Off Income**:
   - For bonuses, tax returns, or gifts, use the **Add Income Entry** form with the target date.
4. **Salary Overrides**:
   - If a member has unpaid leave or overtime in a specific month, add a **Salary Override** (`YYYY-MM`) to temporarily adjust their base salary for deposit calculations.

---

## Step 6: Define Monthly Budgets & Savings Goals (Projects)

1. **Monthly Category Budgets**:
   - Navigate to the **Budgets** tab.
   - Set monthly spending limits in currency units for specific categories or `ALL` categories.
   - Monitor real-time spending progress bars and burndown rates in the **Dashboard**.
2. **Target Savings Goals (Projects)**:
   - Navigate to the **Projects** tab.
   - Create a project (e.g., "New Boiler", "Summer Holiday", "Wedding").
   - Set the `target_amount` and `target_date`.
   - Link expenses to projects during logging to track progress and point-in-time equity settlements.

---

## Step 7: Log Daily Expenses, Split Overrides, Tags & Recurring Templates

1. **Log Daily Expenses**:
   - On the **Dashboard**, use the **Quick Add Expense** form.
   - Select **Who Paid**, **Category**, **Amount**, and **Date**.
   - Select **Paid from Joint Account** if the expense was drawn directly from the shared pool.
2. **Per-Expense Split Overrides**:
   - Expand the **Split Override** section to deviate from the category default (e.g., if one user bought personal items in a shared grocery bill).
3. **Tagging Events**:
   - Assign a **Tag** (e.g., "Trip to Berlin", "Home Renovation") to aggregate expenses across multiple categories.
4. **Recurring Expenses**:
   - In the **Recurring** tab, set up recurring monthly templates (e.g., "Internet Bill" on day 15). The background scheduler will automatically post the expense on the specified day.

---

## Step 8: Month-End Settlement and Debt Simplification

1. Navigate to the **Dashboard** tab and select the **Reimbursements & Settle Up** subtab (or choose the **Complete Overview** option) at the end of the month.
2. View the **Settlement Summary**:
   - Jizifin aggregates all personal payments, split allocations, and overrides.
   - Graph simplification computes the minimum number of reimbursement transfers required to settle all debts.
3. Once all members have made their settlement bank transfers, click **Lock and Settle Month**.
4. Settling locks the historical record, ensuring past reports remain immutable.
