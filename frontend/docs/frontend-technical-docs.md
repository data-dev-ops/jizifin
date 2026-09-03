# Frontend Technical Documentation

Technical reference, state management architecture, component structure, and validation rules for the Jizifin frontend client.

---

## 1. Stack and Runtime Architecture

Jizifin's frontend is a responsive single-page application built with Svelte 4, Tailwind CSS 3, and Chart.js 4. Sensitive financial data is decrypted into browser memory on login and deterministically encrypted before write payloads leave the browser.

| Layer | Library | Version | Role |
|---|---|---|---|
| Framework | [Svelte](https://svelte.dev/) | `^4.2.18` | Declarative UI components and reactive stores |
| Styling | [Tailwind CSS](https://tailwindcss.com/) | `^3.4.4` | Utility-first design system, typography, dark mode, and accessibility tokens |
| Visualizations | [Chart.js](https://www.chartjs.org/) | `^4.4.3` | HTML5 canvas graphs for category spending, income streams, and budget meters |
| Cryptography | Web Crypto API | W3C Standard | PBKDF2 key derivation and deterministic AES-GCM encryption |
| Bundler | [Vite](https://vitejs.dev/) | `^5.3.1` | Build pipeline, HMR, and local API proxy |
| Test Runner | [Vitest](https://vitest.dev/) + JSDOM | `^4.1.10` / `^29.1.1` | Component unit tests, crypto verification, and end-to-end integration scenarios |

### Execution Strategy
1. **Primary Host CLI**: `npm --prefix frontend test -- --run` / `npm --prefix frontend run dev`
2. **NVM Fallback**: If Node/NPM are missing from the subshell PATH, resolve via `~/.nvm/versions/node/$(ls ~/.nvm/versions/node | tail -1)/bin/npm`
3. **Pre-Built Docker Container Fallback**:
   ```bash
   docker run --rm \
     -v $(pwd)/frontend/src:/app/src \
     -v $(pwd)/frontend/index.html:/app/index.html \
     -v $(pwd)/frontend/tailwind.config.js:/app/tailwind.config.js \
     -v $(pwd)/frontend/vite.config.js:/app/vite.config.js \
     jizifin-frontend-test npm test -- --run
   ```

---

## 2. Directory Layout

```
frontend/
├── docs/
│   └── frontend-technical-docs.md    # Frontend technical reference (this file)
├── src/
│   ├── lib/
│   │   ├── docs/                     # Interactive Documentation Hub (/docs)
│   │   │   ├── DocsHub.svelte        # Documentation index and tab coordinator
│   │   │   ├── GettingStartedDocs.svelte # Step-by-step How-To Guide
│   │   │   ├── FrontendDocs.svelte   # In-app frontend technical guide
│   │   │   ├── BackendDocs.svelte    # In-app backend architecture overview
│   │   │   └── SystemDocs.svelte     # Cryptography and zero-knowledge model
│   │   ├── AnalyticsSummary.svelte   # Monthly totals, payer splits, budget gauges
│   │   ├── BudgetManager.svelte      # Category budget targets and burndown meters
│   │   ├── ExpenseForm.svelte        # Expense creation with tag validation and split overrides
│   │   ├── ExpenseList.svelte        # Paginated expense ledger with filtering and bulk actions
│   │   ├── IncomeChart.svelte        # Income breakdown visualization
│   │   ├── IncomeTab.svelte          # One-off income, jobs contracts, and salary overrides
│   │   ├── JointAccountTab.svelte    # Multi-joint accounts, member balances, and deposit execution
│   │   ├── Login.svelte              # Master passphrase login and first-run setup
│   │   ├── PaybackVisual.svelte      # Simplified debt settlement graph and month locking
│   │   ├── ProjectsTab.svelte        # Target milestone budgets, user memberships, and settlement sheets
│   │   ├── QueryConsole.svelte       # Raw SQL console for development and audits
│   │   ├── RealtimeChart.svelte      # WebSocket live transaction ticker
│   │   ├── RecurringManager.svelte   # Automated recurring expense templates and schedules
│   │   ├── SettingsTab.svelte        # 7-Domain personalization, presets, DB export/import
│   │   ├── SplitManager.svelte       # Category registry and SCD2 split timeline overrides
│   │   ├── TagsTab.svelte            # Event tags and active date range rules
│   │   ├── UserManager.svelte        # Household member list, colors, and active flags
│   │   ├── api.js                    # Fetch wrapper with automatic AES-GCM encryption
│   │   ├── colorUtils.js             # HSL color math and contrast ratio checks
│   │   ├── crypto.js                 # WebCrypto PBKDF2 derivation and AES-GCM static IV routines
│   │   └── stores.js                 # Svelte writable stores and localStorage bindings
│   ├── test/
│   │   ├── components/               # 22 Svelte UI component test suites
│   │   ├── setup.js                  # JSDOM polyfills (WebCrypto, Canvas, matchMedia)
│   │   └── *.test.js                 # 17 domain and mathematical unit test suites
│   ├── App.svelte                    # Root layout shell, sidebar navigation, privacy shield
│   ├── app.css                       # Tailwind layers and root variables
│   └── main.js                       # App mount point and theme listener
├── index.html                        # HTML shell
├── package.json                      # Dependencies and test scripts
├── tailwind.config.js                # Theme and color tokens
└── vite.config.js                    # Vite dev server and proxy configuration
```

---

## 3. Cryptography Architecture (`crypto.js`)

To maintain zero-knowledge privacy for household finances, sensitive text data is encrypted client-side before transmission:

```
Master Passphrase -> PBKDF2 (SHA-256, 100,000 iterations, salt: "jizifin-salt-pbkdf2")
                  -> 256-bit AES-GCM CryptoKey
                  -> Encrypt(plaintext, key, IV: "jizifin-cryp")
                  -> Base64URL string (no padding)
```

### 3.1 Key Derivation

```javascript
const STATIC_SALT = new TextEncoder().encode('jizifin-salt-pbkdf2');
const STATIC_IV = new Uint8Array([106, 105, 122, 105, 102, 105, 110, 45, 99, 114, 121, 112]); // "jizifin-cryp"

export async function deriveKey(passphrase) {
  const enc = new TextEncoder();
  const keyMaterial = await crypto.subtle.importKey(
    'raw',
    enc.encode(passphrase),
    { name: 'PBKDF2' },
    false,
    ['deriveKey']
  );
  return crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt: STATIC_SALT,
      iterations: 100000,
      hash: 'SHA-256',
    },
    keyMaterial,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt']
  );
}
```

### 3.2 Deterministic AES-GCM Matching and Column Encryption Matrix
- **Static IV**: Using fixed 12-byte IV `[106, 105, 122, 105, 102, 105, 110, 45, 99, 114, 121, 112]` produces deterministic ciphertext. Identical plaintext strings always produce the identical Base64URL string. This allows SQLite to perform exact equality joins (`ON a.col = b.col`), `GROUP BY`, and foreign key cascades without the database ever having access to plaintext keys.
- **Query Rules**: Range queries (`<`, `>`), pattern matching (`LIKE %...%`), and alphabetical sorting must never be run on encrypted columns. Plaintext columns handle all numerical aggregations and date filtering.

```
┌────────────────────────────────────────────────────────────────────────┐
│                        COLUMN ENCRYPTION MATRIX                        │
├───────────────────────────────────┬────────────────────────────────────┤
│ ENCRYPTED (Base64URL)             │ PLAINTEXT                          │
├───────────────────────────────────┼────────────────────────────────────┤
│ users.name                        │ All *_cents (cost, amount, target) │
│ splits.category                   │ All dates (YYYY-MM-DD, month)      │
│ income_categories.category        │ All integer IDs and foreign keys   │
│ projects.name                     │ Boolean flags (is_joint, is_active)│
│ expenses.name, who_paid, category │ colors (#hex)                      │
│ income.name, who, category        │ settlements table                  │
│ recurring_expenses.name, who, cat │ day_of_month, frequencies          │
│ jobs.name, who, notes             │ percentages (pct, REAL)            │
│ split_allocations.category, user  │ safety_margin_pct                  │
│ tags.name, description            │ deposit_split_mode                 │
│ joint_accounts.name               │ magic_word (app_config)            │
│ joint_account_deposits.user_name  │                                    │
│ joint_account_corrections.note    │                                    │
│ salary_overrides.user_name, note  │                                    │
└───────────────────────────────────┴────────────────────────────────────┘
```

---

## 4. State Management (`stores.js`)

Client state is maintained in Svelte writable stores. Decrypted financial data exists only in browser memory and is wiped on logout. Preferences are persisted in `localStorage`.

| Store | Type | Persistence | Description |
|---|---|---|---|
| `authSalt` | `writable(string)` | Session Memory | Master key passphrase in memory (cleared on logout) |
| `cryptoKey` | `writable(CryptoKey)` | Session Memory | Derived 256-bit AES-GCM key |
| `selectedMonth` | `writable(string)` | LocalStorage | Active target month (`YYYY-MM`) |
| `dashboardScope` | `writable(string)` | LocalStorage | Scope filter (`ALL`, `USER:<name>`, `JOINT:<id>`) |
| `users` | `writable(User[])` | Memory | Household member list `[{ name, color, is_active }]` |
| `expenses` | `writable(Expense[])` | Memory | Expenses for active month (newest-first) |
| `splits` | `writable(Split[])` | Memory | Category registry, default allocations, and SCD2 agreements |
| `incomeCategories` | `writable(string[])` | Memory | Income category registry |
| `incomeEntries` | `writable(Income[])` | Memory | Non-recurring income items for the selected month |
| `incomeAnalytics` | `writable(IncomeRow[])` | Memory | Combined income totals per person with carry-forward |
| `jobs` | `writable(Job[])` | Memory | Employment contracts and normalized monthly salary |
| `projects` | `writable(Project[])` | Memory | Savings targets with participant member lists |
| `tags` | `writable(Tag[])` | Memory | Event tags with active date constraints |
| `budgets` | `writable(Budget[])` | Memory | Monthly category spending limits |
| `recurringExpenses` | `writable(Recurring[])`| Memory | Automated recurring expense templates |
| `settlements` | `writable(Settlement[])`| Memory | Locked historical months |
| `jointAccounts` | `writable(JointAccount[])` | Memory | Multi-joint account registry |
| `activeJointAccountId` | `writable(number)` | LocalStorage | Currently selected joint account ID |
| `jointAccount` | `writable(JointConfig)` | Memory | Active joint account configuration |
| `jointCategories` | `writable(string[])` | Memory | Categories mapped to joint account |
| `jointDeposits` | `writable(Deposit[])` | Memory | Monthly deposit schedules per user |
| `jointMonthlyDeposits` | `writable(MonthlyDep[])` | Memory | Deposit execution tracking (paid, overdue, pending) |
| `jointExpectedCosts` | `writable(ExpectedCost[])` | Memory | Expected monthly cost per category |
| `jointCorrections` | `writable(Correction[])` | Memory | Balance correction log (+/-) |
| `jointDashboard` | `writable(Dashboard)` | Memory | Spending actuals vs expected costs and deposit status |
| `jointAccountEnabled` | `writable(boolean)` | LocalStorage | Module toggle for joint accounts |
| `currencySymbol` | `writable(string)` | LocalStorage | Currency symbol preset (`€`, `$`, `£`, `CHF`, `¥`, `kr`) |
| `roundingMode` | `writable(string)` | LocalStorage | Decimal display mode (`cents` or `whole`) |
| `rowDensity` | `writable(string)` | LocalStorage | Display density (`minimal`, `compact`, `detailed`) |
| `activePreset` | `writable(string)` | LocalStorage | 1-Click functional preset |
| `mobileTabVisibility` | `writable(object)` | LocalStorage | Mobile navigation tab visibility map |
| `desktopTabVisibility` | `writable(object)` | LocalStorage | Desktop navigation tab visibility map |
| `theme` | `writable(string)` | LocalStorage | Theme selection (`dark`, `light`, `system`) |
| `privacyShield` | `writable(boolean)` | LocalStorage | Frosted-glass blur mask over balances |
| `highContrast` | `writable(boolean)` | LocalStorage | High-contrast borders and accessibility tokens |
| `textScale` | `writable(string)` | LocalStorage | Text size scale (`100`, `110`, `125`) |

---

## 5. API Client and Real-Time WebSockets (`api.js`)

### 5.1 HTTP Client & Automatic Cryptography
All backend communication flows through `request()` in `api.js`. Sensitive payload fields are encrypted before dispatch, and responses are decrypted before updating Svelte stores:
- Non-2xx responses parse the server's `detail` message and throw standard `Error` objects.
- Components bind `err.message` to local reactive error banners.

### 5.2 Real-Time WebSockets (`/ws/finance`)
Clients connect to `/ws/finance` upon authentication. When any household user modifies expenses, joint accounts, or categories, the server fans out an event payload (`expense_created`, `expense_updated`, `joint_account_updated`). The client reactively refreshes analytics, paybacks, and dashboard stores in real time.

---

## 6. Component Architecture & Deep-Dive

### 6.1 `AnalyticsSummary.svelte` & `PaybackVisual.svelte` (Dashboard Command Center & Subtabs)
- **Subtab Architecture**:
  - `Financial Pulse & Spend`: High-level KPI summary cards, payer distributions, multi-style spending charts (doughnut, bar, polarArea, pie), and category breakdown table.
  - `Reimbursements & Settle Up` (`PaybackVisual.svelte`): Graph-reduced minimal reimbursement debt transfers and month-locking reconciliation settlements (`Lock and Settle Month`).
  - `Joint Account & Deposits`: Health indicators, expected vs actual category costs, deposit schedules, and payment execution status (`paid`, `overdue`, `pending`).
  - `Category Budgets & Health`: Live monthly category budget caps, percent-used progress bars, and over-budget warnings.
  - `Live Timeline & Projects`: Real-time expense transaction velocity ticker and project completion progress.
  - `Complete Overview` (`complete`): Consolidated overview rendering all subtab modules concurrently in a scrollable, unified command center.
- **Global Scope Switcher**: Top-level multi-select filter (`ALL`, individual users, or joint accounts).

### 6.2 `SplitManager.svelte` (Category Registry & SCD2 Timeline Agreements)
- **Category Registry**: Manages expense categories with real-time percentage allocation sliders.
- **Renaming Cascades**: Renaming a category cascades across referencing tables without breaking historical expenses.
- **SCD2 Timeline Tray**: Collapsible tray on each category card displaying historical, active, and scheduled split overrides.
- **Active Override Badge**: Real-time visual badge (`⚡ Override Active (YYYY-MM)`) signaling when a date-bounded override is in effect.

### 6.3 `JointAccountTab.svelte` (Multi-Joint Accounts & Deposit Execution)
- **Multi-Account Switcher**: Tab bar switching between multiple isolated joint accounts (`Couple AB Joint`, `Couple CD Joint`).
- **Deposit Execution & Tracking**: Real-time monthly deposit schedule tracking (`paid`, `paid_diverted`, `overdue`, `pending`) with 1-click "Mark as Paid" actions.
- **Signed Corrections**: Audit log for manual top-ups (+) and withdrawals (-).
- **Settlement Engine**: Reconciles surplus/deficit via direct reimbursement or automated next-month deposit adjustments.

### 6.4 `ProjectsTab.svelte` (Milestone Budgets & Point-in-Time Settlement Sheets)
- **Multi-User Membership (`project_users`)**: Specific household participants assignable to each project.
- **Burndown Analytics**: Target savings goals, completion estimates, and monthly funding velocity.
- **Point-in-Time Settlement Sheet (`GET /projects/{id}/settlement`)**: Decomposes joint-account contributions into participant equity shares (e.g. 60/40), balancing effective funding against assigned liability and routing minimal debt transfers.

### 6.5 `ExpenseForm.svelte` & `ExpenseList.svelte` (Ledger Engine)
- **Form Memory & Search**: Smart form memory recalling recent categories, payers, and tags.
- **Tag Timeline Validation**: Dynamic inline feedback preventing submission if an expense date falls outside the selected tag's active window.
- **Split Percentage Overrides**: Optional per-transaction split override modal.
- **Direct Joint Flag**: Checkbox assigning expenses directly to a joint account (`is_joint = 1`), automatically excluding them from peer-to-peer payback calculations.

### 6.6 `SettingsTab.svelte` (7-Domain Personalization & Backups)
- **Device Profiles**: Independent Desktop vs Mobile tab visibility profiles.
- **1-Click Functional Presets**: Instantly toggles between `Streamlined`, `High Legibility`, `Standard / Balanced`, and `Detailed / Power User`.
- **Privacy Shield**: Frosted-glass blur obscuring sensitive balances in public spaces with hover-peek.
- **Database Backup & Streaming**: Encrypted `.db` export and seamless import with schema migration.

### 6.7 `DocsHub.svelte` (In-App Interactive Documentation Hub)
- Fully browsable in-app technical reference suite (`DocsHub.svelte`, `BackendDocs.svelte`, `FrontendDocs.svelte`, `GettingStartedDocs.svelte`, `SystemDocs.svelte`) mirroring repo architecture documentation.

---

## 7. 7-Domain Personalization & Settings Engine

Jizifin features an extensive display and usability personalization engine:

1. **Independent Device Profiles**: Configure visible navigation tabs independently for Desktop and Mobile viewports via `mobileTabVisibility` and `desktopTabVisibility`.
2. **1-Click Functional Presets**:
   - `Streamlined`: Hides secondary tabs, enables minimal density, focuses on quick expense capture.
   - `High Legibility`: Activates high contrast borders, 125% text scale, and spacious density.
   - `Standard / Balanced`: Default full-featured configuration.
   - `Detailed / Power User`: Activates all tabs, detailed row density, and full analytics meters.
3. **Public Stealth Privacy Shield**: Obscures currency amounts with frosted-glass blur; reveals on mouse hover.
4. **Currency Presets & Rounding**: Select symbol (`€`, `$`, `£`, `CHF`, `¥`, `kr`) and toggle between whole-unit rounding and exact cent precision.
5. **Row Density**: Configurable table row padding (`minimal`, `compact`, `detailed`).
6. **Theme Engine**: Seamless switching between light, dark, and system color schemes.
7. **Accessibility Tokens**: High-contrast outline borders and scaled typography tokens.

---

## 8. Settings & Input Validation Matrix

Authoritative client-side validation rules matching backend schema constraints:

### 8.1 Category Split Allocations (`/splits`, `SplitManager.svelte`)
- **Rule**: Every user's allocation percentage must be between `0.0` and `100.0`. Total sum must equal `100.0%` (tolerance $\pm 0.02\%$). Duplicate members are prohibited.

```json
// VALID: Exact 100.0% sum
{
  "category": "Groceries",
  "allocations": [
    { "user_name": "Alice", "pct": 60.0 },
    { "user_name": "Bob", "pct": 40.0 }
  ]
}

// INVALID: Sum is 90.0% (HTTP 422: "Allocations must sum to 100.0")
{
  "category": "Groceries",
  "allocations": [
    { "user_name": "Alice", "pct": 50.0 },
    { "user_name": "Bob", "pct": 40.0 }
  ]
}
```

### 8.2 SCD2 Split Agreements (`/splits/{category}/agreements`, `SplitManager.svelte`)
- **Rule**: `start_date` must be formatted `YYYY-MM-DD`. If `end_date` is provided, `start_date <= end_date`. Allocations within the agreement must sum to `100.0%`.

```json
// VALID: Date-bounded override window
{
  "category": "Groceries",
  "start_date": "2026-08-01",
  "end_date": "2026-08-31",
  "is_active": true,
  "note": "Summer Host Month",
  "allocations": [
    { "user_name": "Alice", "pct": 20.0 },
    { "user_name": "Bob", "pct": 80.0 }
  ]
}

// INVALID: Inverted timeline (HTTP 422: "start_date must be less than or equal to end_date")
{
  "category": "Groceries",
  "start_date": "2026-08-31",
  "end_date": "2026-08-01"
}
```

### 8.3 Tag Timelines & Expense Bounds (`/tags`, `TagsTab.svelte`)
- **Rule**: `start_date <= end_date`. Any expense tagged with `tag_id` must have `tag.start_date <= expense.expense_date <= tag.end_date`.

```json
// VALID: Expense within tag bounds
{
  "name": "Paint Supplies",
  "cost_cents": 5000,
  "expense_date": "2026-06-15",
  "tag_id": 1  // Tag window: 2026-06-01 to 2026-06-30
}

// INVALID: Out-of-bounds expense (HTTP 422: "Expense date outside tag timeline")
{
  "name": "Post-Event Dinner",
  "cost_cents": 8000,
  "expense_date": "2026-07-05",
  "tag_id": 1
}
```

### 8.4 Multi-Joint Account Settings (`/joint-accounts`, `JointAccountTab.svelte`)
- **Rule**: `safety_margin_pct` must be an integer between `0` and `100`. `deposit_split_mode` must be one of `"even"`, `"salary"`, or `"manual"`.

```json
// VALID: Joint account with members
{
  "name": "Couple AB Joint",
  "balance_cents": 115000,
  "safety_margin_pct": 10,
  "deposit_split_mode": "even",
  "member_names": ["Alice", "Bob"]
}

// INVALID: Out-of-bounds safety margin (HTTP 422)
{
  "safety_margin_pct": 150
}
```

### 8.5 Employment Contracts & Salary Overrides (`/jobs`, `IncomeTab.svelte`)
- **Rule**: Job `amount_cents > 0`. Frequency must be `"monthly"`, `"weekly"`, `"biweekly"`, or `"annual"`. Salary override `amount_cents >= 0`, `month` formatted `YYYY-MM`.

---

## 9. Automated Testing & Verification

The frontend test suite contains **343 tests across 39 test files** using Vitest and JSDOM:

### Test Execution Commands
- **Primary Host CLI**:
  ```bash
  npm --prefix frontend test -- --run
  ```
- **Pre-Built Docker Container Fallback**:
  ```bash
  docker run --rm \
    -v $(pwd)/frontend/src:/app/src \
    -v $(pwd)/frontend/index.html:/app/index.html \
    -v $(pwd)/frontend/tailwind.config.js:/app/tailwind.config.js \
    -v $(pwd)/frontend/vite.config.js:/app/vite.config.js \
    jizifin-frontend-test npm test -- --run
  ```

### Test Scope and Coverage
- **Unit & Mathematical Suites (17 files)**:
  - Cryptographic key derivation, AES-GCM static IV determinism, Base64URL encoding (`crypto.test.js`)
  - HTTP error translation and status propagation (`api.test.js`)
  - Float cent conversion, currency display, and Largest Remainder splits (`numerical_precision.test.js`)
  - 7-domain personas and device profile persistence (`customization_personas.test.js`, `device_profiles.test.js`)
  - Historical reconciliation month locking (`reconciliation_locking.test.js`)
  - Multi-currency symbols and formatting (`multicurrency.test.js`)
- **Component & Integration Suites (22 files)**:
  - Full UI scenario verification with synthetic identities `Alice` & `Bob` and `test-master-passphrase` (`ScenariosFrontendUI.test.js`)
  - Multi-joint accounts and monthly deposit execution (`JointAccountTab.test.js`, `MultiHouseholdCouples.test.js`)
  - SCD2 split agreement timeline trays and override badges (`SplitManager.test.js`)
  - Project milestone equity settlements (`ProjectsTab.test.js`)
  - Tag timeline boundary validation and form guards (`TagsTab.test.js`, `ExpenseForm.test.js`)
  - In-app documentation hub rendering and routes (`Docs.test.js`)
