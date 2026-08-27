# Frontend Technical Documentation

Technical reference for the Jizifin frontend client.

---

## 1. Stack

Jizifin's frontend is a single-page app built with Svelte 4, Tailwind CSS 3, and Chart.js 4. Client state is decrypted in browser memory on login and encrypted before sending write payloads to the backend.

| Layer | Library | Version | Role |
|---|---|---|---|
| Framework | [Svelte](https://svelte.dev/) | `^4.2.18` | UI components and reactive state |
| Styling | [Tailwind CSS](https://tailwindcss.com/) | `^3.4.4` | Layout, typography, dark mode, and accessibility tokens |
| Charts | [Chart.js](https://www.chartjs.org/) | `^4.4.3` | HTML5 canvas graphs for spending, income, and budgets |
| Security | Web Crypto API | W3C Standard | PBKDF2 key derivation and AES-GCM encryption |
| Bundler | [Vite](https://vitejs.dev/) | `^5.3.1` | Build pipeline, HMR, and local proxy |
| Test Runner | [Vitest](https://vitest.dev/) + JSDOM | `^4.1.10` / `^29.1.1` | Component tests, crypto validation, and math checks |

---

## 2. Directory Layout

```
frontend/
├── docs/
│   └── frontend-technical-docs.md    # Frontend technical reference (this file)
├── src/
│   ├── lib/
│   │   ├── docs/                     # Documentation pages
│   │   │   ├── DocsHub.svelte        # Documentation index (/docs)
│   │   │   ├── GettingStartedDocs.svelte # Step-by-step How-To Guide (/docs/getting-started)
│   │   │   ├── FrontendDocs.svelte   # Frontend technical reference (/docs/frontend)
│   │   │   ├── BackendDocs.svelte    # Backend architecture overview (/docs/backend)
│   │   │   └── SystemDocs.svelte     # Cryptography and system design (/docs/architecture)
│   │   ├── AnalyticsSummary.svelte   # Monthly totals, payer splits, budget meters
│   │   ├── BudgetManager.svelte      # Category budget targets and tracking
│   │   ├── ExpenseForm.svelte        # Expense logging and split override editor
│   │   ├── ExpenseList.svelte        # Paginated expense ledger with filtering
│   │   ├── IncomeChart.svelte        # Income breakdown chart
│   │   ├── IncomeTab.svelte          # One-off income and ongoing job contracts
│   │   ├── JointAccountTab.svelte    # Joint pool balance, monthly deposits, corrections
│   │   ├── Login.svelte              # Master passphrase login
│   │   ├── PaybackVisual.svelte      # Simplified debt balances and month settlements
│   │   ├── ProjectsTab.svelte        # Goal tracking and settlement calculations
│   │   ├── QueryConsole.svelte       # Raw SQL console for development
│   │   ├── RealtimeChart.svelte      # WebSocket live transaction ticker
│   │   ├── RecurringManager.svelte   # Recurring expense schedule definitions
│   │   ├── SettingsTab.svelte        # Appearance, currency symbol, DB export
│   │   ├── SplitManager.svelte       # Category definitions and default split ratios
│   │   ├── TagsTab.svelte            # Event tags and date range rules
│   │   ├── UserManager.svelte        # Household member list and colors
│   │   ├── api.js                    # Fetch wrapper and WebSocket client
│   │   ├── colorUtils.js             # HSL color math and contrast checks
│   │   ├── crypto.js                 # PBKDF2 derivation and AES-GCM routines
│   │   └── stores.js                 # Svelte stores and localStorage bindings
│   ├── test/
│   │   ├── components/               # Svelte component tests
│   │   ├── setup.js                  # JSDOM polyfills (WebCrypto, Canvas, matchMedia)
│   │   ├── api.test.js               # API client error handling tests
│   │   ├── crypto.test.js            # Key derivation and encryption tests
│   │   └── numerical_precision.test.js # Cents math and allocation distribution tests
│   ├── App.svelte                    # Root layout, navigation router, and sidebar
│   ├── app.css                       # Tailwind layers and root variables
│   └── main.js                       # App mount and theme listener
├── index.html                        # HTML shell
├── package.json                      # Scripts and dependencies
├── tailwind.config.js                # Theme and color config
└── vite.config.js                    # Vite dev server and proxy rules
```

---

## 3. Cryptography (`crypto.js`)

Sensitive text fields (names, descriptions, categories, notes) are encrypted in the browser before sending requests over the network.

```
Master Passphrase -> PBKDF2 (SHA-256, 100k iterations, salt: "jizifin-salt-pbkdf2")
                  -> 256-bit AES-GCM Key
                  -> Encrypt(plaintext, key, IV: "jizifin-cryp")
                  -> Base64URL string
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

### 3.2 Deterministic Matching and Data Scope

- **Static IV**: Encryption uses a fixed 12-byte IV (`"jizifin-cryp"`). An identical plaintext string always produces the identical Base64URL ciphertext. This lets SQLite group rows by category and enforce foreign keys without holding plaintext keys.
- **Encrypted Columns**: User names, category names, expense descriptions, project names, tag names and descriptions, job notes, salary override notes.
- **Plaintext Columns**: Currency amounts (in integer cents), ISO dates (`YYYY-MM-DD`), IDs, and boolean flags (`is_joint`, `is_active`).

---

## 4. State Management (`stores.js`)

State is held in Svelte writable stores. Sensitive decrypted data remains in memory only during the session.

| Store | Type | Persistence | Description |
|---|---|---|---|
| `authSalt` | `writable(string)` | Session Memory | Master key passphrase in memory (cleared on logout) |
| `selectedMonth` | `writable(string)` | LocalStorage | Active month (`YYYY-MM`) |
| `users` | `writable(User[])` | Memory | Household member list `[{ name, color, is_active }]` |
| `expenses` | `writable(Expense[])` | Memory | Expenses for the active month |
| `splits` | `writable(Split[])` | Memory | Category list and per-user split percentages |
| `incomeCategories` | `writable(string[])` | Memory | Income category names |
| `incomeRows` | `writable(Income[])` | Memory | Non-recurring income items for the active month |
| `jobs` | `writable(Job[])` | Memory | Recurring employment contracts |
| `projects` | `writable(Project[])` | Memory | Savings targets |
| `tags` | `writable(Tag[])` | Memory | Event tags with date constraints |
| `budgets` | `writable(Budget[])` | Memory | Monthly category limits |
| `settlements` | `writable(Settlement[])` | Memory | Locked historical months |
| `jointAccount` | `writable(JointConfig)` | Memory | Joint account balance and deposit settings |
| `currencySymbol` | `writable(string)` | LocalStorage | Currency sign (e.g. `€`, `$`) |
| `theme` | `writable(string)` | LocalStorage | `dark`, `light`, or `system` |
| `privacyShield` | `writable(boolean)` | LocalStorage | Masks balances on screen |
| `highContrast` | `writable(boolean)` | LocalStorage | Increases border contrast |
| `textScale` | `writable(string)` | LocalStorage | `100`, `110`, or `125` text size modifier |

---

## 5. API and Real-Time Client (`api.js`)

### 5.1 HTTP Client

All API requests pass through `request()` in `api.js`. Non-2xx responses throw an `Error` containing the server's `detail` message:

```javascript
async function request(path, options = {}) {
  const res = await fetch(`${API_BASE}${path}`, options);
  if (!res.ok) {
    let detail = `HTTP ${res.status}`;
    try {
      const data = await res.json();
      detail = data.detail || detail;
    } catch {}
    throw new Error(detail);
  }
  return res.json();
}
```

### 5.2 WebSockets (`/ws/finance`)

Clients connect to `/ws/finance`. When another user adds or edits an expense, the backend broadcasts an event (`expense_created`, `expense_updated`). The client re-fetches analytics and payback balances on receiving these messages.

---

## 6. Components

The application divides UI by feature:

- **`App.svelte`**: Router, sidebar navigation, month selector, and unauthenticated docs routing.
- **`AnalyticsSummary.svelte`**: Spending totals, payer breakdown cards, and budget burndown gauges.
- **`ExpenseForm.svelte` & `ExpenseList.svelte`**: Form with split percentage overrides and tag date validation; ledger table with inline delete confirmation.
- **`SplitManager.svelte`**: Category editor and batch allocation updates.
- **`JointAccountTab.svelte`**: Balance tracking, monthly deposit shares, and balance corrections.
- **`IncomeTab.svelte` & `IncomeChart.svelte`**: One-off bonuses and recurring job contracts normalized to monthly equivalents.
- **`ProjectsTab.svelte`**: Target goal progress and equity settlement calculations.
- **`TagsTab.svelte`**: Cross-month event tracking with date boundary checks.
- **`SettingsTab.svelte`**: Appearance settings, device profiles, DB export, and documentation links.

---

## 7. Styling and Accessibility

- **Tailwind CSS**: Layouts use utility classes without custom scoped CSS blocks.
- **Dark Mode**: Managed by the `.dark` class on `document.documentElement`.
- **Privacy Shield**: Replaces balance digits with dots to obscure numbers in shared spaces.
- **High Contrast**: Adds 2px borders and high-contrast color values.
- **Text Scaling**: Root font adjustments via `.text-scale-110` and `.text-scale-125`.

---

## 8. Settings Validation Matrix (Valid vs. Invalid Examples)

The following reference details schema rules and valid versus invalid configurations across settings and form payloads:

### 8.1 Category Split Allocations (`/splits`, `SplitManager.svelte`)
- **Rule**: Every user's allocation percentage must be between `0.0` and `100.0`. The sum of all active user allocations must equal `100.0` (tolerance $\pm 0.02\%$). Duplicate usernames are prohibited.

```json
// VALID: Sum equals 100.0% exactly
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

// INVALID: Negative share (HTTP 422: "Allocation percentage must be between 0.0 and 100.0")
{
  "category": "Groceries",
  "allocations": [
    { "user_name": "Alice", "pct": 110.0 },
    { "user_name": "Bob", "pct": -10.0 }
  ]
}
```

### 8.2 Tag Timelines & Expense Boundary Constraints (`/tags`, `TagsTab.svelte`)
- **Rule**: `start_date` must be $\le$ `end_date` (both formatted `YYYY-MM-DD`). Any expense tagged with `tag_id` must have `tag.start_date <= expense.expense_date <= tag.end_date`.

```json
// VALID: Chronological order
{
  "name": "Summer Vacation",
  "color": "#0ea5e9",
  "start_date": "2026-07-01",
  "end_date": "2026-07-15"
}

// INVALID: End date before start date (HTTP 422: "start_date must be less than or equal to end_date")
{
  "name": "Summer Vacation",
  "color": "#0ea5e9",
  "start_date": "2026-07-15",
  "end_date": "2026-07-01"
}
```

### 8.3 Joint Account Settings (`/joint-account`, `JointAccountTab.svelte`)
- **Rule**: `safety_margin_pct` must be an integer between `0` and `100`. `deposit_split_mode` must be one of `"even"`, `"salary"`, or `"manual"`.

```json
// VALID: Valid mode and margin within bounds
{
  "safety_margin_pct": 15,
  "deposit_split_mode": "salary"
}

// INVALID: Out-of-bounds safety margin (HTTP 422: "Input should be less than or equal to 100")
{
  "safety_margin_pct": 150,
  "deposit_split_mode": "even"
}

// INVALID: Unknown deposit split mode (HTTP 422: "Input should be 'salary', 'even' or 'manual'")
{
  "safety_margin_pct": 10,
  "deposit_split_mode": "automatic"
}
```

### 8.4 Employment Contracts & Jobs (`/jobs`, `IncomeTab.svelte`)
- **Rule**: `amount_cents` must be $> 0$. `frequency` must be `"monthly"`, `"weekly"`, `"biweekly"`, or `"annual"`. `start_date <= end_date`.

```json
// VALID: Monthly employment contract
{
  "name": "Software Engineer",
  "who": "Alice",
  "amount_cents": 420000,
  "frequency": "monthly",
  "start_date": "2026-01-01"
}

// INVALID: Zero amount (HTTP 422: "Input should be greater than 0")
{
  "name": "Software Engineer",
  "who": "Alice",
  "amount_cents": 0,
  "frequency": "monthly",
  "start_date": "2026-01-01"
}
```

### 8.5 Salary Overrides (`/income/salary-overrides`, `IncomeTab.svelte`)
- **Rule**: `amount_cents` must be $\ge 0$. `month` must match format `YYYY-MM`.

```json
// VALID: Zero cent override (e.g. unpaid leave)
{
  "user_name": "Bob",
  "month": "2026-08",
  "amount_cents": 0,
  "note": "Unpaid sabbatical"
}

// INVALID: Invalid date format (HTTP 422: "month must be in YYYY-MM format")
{
  "user_name": "Bob",
  "month": "2026/08",
  "amount_cents": 250000
}
```

### 8.6 Device Profiles and Appearance Settings (`SettingsTab.svelte`)
- **Rule**: `textScale` must be `"100"`, `"110"`, or `"125"`. `theme` must be `"dark"`, `"light"`, or `"system"`.

---

## 9. Testing

Tests run using Vitest and JSDOM:

```bash
docker run --rm \
  -v $(pwd)/frontend/src:/app/src \
  -v $(pwd)/frontend/index.html:/app/index.html \
  -v $(pwd)/frontend/tailwind.config.js:/app/tailwind.config.js \
  -v $(pwd)/frontend/vite.config.js:/app/vite.config.js \
  jizifin-frontend-test npm test
```

Test coverage includes:
- Key derivation and deterministic ciphertext generation (`crypto.test.js`)
- API error bubbling and status translation (`api.test.js`)
- Integer cents conversions and Largest Remainder splits (`numerical_precision.test.js`)
- Component rendering, form validation, and docs routing (`components/*.test.js`)
