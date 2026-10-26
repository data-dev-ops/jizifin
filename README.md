# Personal Finance Tracker (Jizifin)

A monorepo personal finance tracking application designed for managing shared household expenses, employment incomes, budgets, joint accounts, and paybacks across multi-member and multi-couple households. Built with a Rust (axum) backend on embedded DuckDB 2.0, an in-process Auditor that owns every contentious calculation, and a thin Vanilla Svelte client that only encrypts, decrypts and renders.

---

## 🚀 Key Features

- **Dashboard & Analytics:** Comprehensive overview of balances, monthly totals, category breakdowns, and dynamic real-time spending charts with multi-select household scope filtering (Everyone, individual members, and joint accounts).
- **Jobs & Period-Specific Salary Adjustments:** Define employment contracts and regular income streams per person with customizable frequency (monthly, weekly, bi-weekly, annual), timeline start/end dates, and 1-click raise/promotion/leave adjustments. Supports month-specific salary overrides (`salary_overrides`) for temporary overtime, unpaid leave, or sickness—dynamically recalculating proportional salary ratios strictly for that period without modifying base contracts.
- **Device Display Profiles (Desktop vs. Mobile):** Distinct persistent profiles stored independently in `localStorage` (`jizifin_profile_desktop` and `jizifin_profile_mobile`). Automatically detects client device hardware with live status badge, supporting profile switching, explicit saving, profile cloning, and default resetting.
- **Public Privacy Shield (Stealth Blur):** Frosted-glass CSS blur with hover/tap peek unmasking. Toggleable via a top header action button (`👁️` / `🕶️`) or Settings, protecting account balances, salary details, and transaction amounts in public environments.
- **Quick Display & Workflow Presets:** 1-Click functional interface presets:
  - `⚡ Streamlined`: Minimal form inputs and compact single-line ledger rows for rapid day-to-day logging.
  - `👓 High Legibility`: 115% larger typography, high-contrast borders, and whole-unit summary rounding.
  - `🏡 Standard / Balanced`: Default comprehensive layout with compact density and core household modules.
  - `📊 Detailed / Power User`: Exact `.00` precision everywhere, detailed metadata chips, and advanced tabs (Query Console, Budgets).
- **View Depth & Rapid Logging Accelerators:** Configurable project lifecycle filters (`Active Only`, `In Progress`, `All Archive`), ledger row density (`Minimal`, `Compact`, `Detailed`), currency precision mode, smart form memory (`Remember Last Used`, `Static Defaults`, `Empty`), and granular form field toggles.
- **Exact Basis-Point & Float Percentage Splits:** Configure category split allocations with full basis-point float precision (`0.01%`), supporting proportional income splits (e.g. $26.6667\% / 13.3333\% / 25.9259\% / 11.1111\% / 18.5185\%$) without forced integer coercion.
- **Signed Hare-Niemeyer / Largest Remainder Math:** Integer-cent split distribution supporting positive transactions, zero, and negative refunds/credit memos using a mathematical floor, paired with a deterministic transaction-salted SHA-256 tie-breaker to prevent alphabetical bias.
- **Multi-Tenant Joint Accounts:** Isolated joint accounts owned by specific household sub-groups (e.g., Couple AB, Couple CD) with custom safety margins, expected monthly costs, per-user monthly deposit schedules, and signed balance corrections.
- **Payback Calculator & Graph Decomposition:** Computes exact net balances based on payer, category shares, and joint exclusions. Employs connected-component graph decomposition to isolate subgroup debts from cross-household transfers before executing greedy debt simplification.
- **Dynamic Tag Timelines & Boundary Enforcement:** Open-ended color-coded tag labeling system with `start_date` and `end_date` active windows. Validates that expenses fall strictly within active tag milestones and prevents retrospective tag window shrinkage.
- **Projects & Settlement Equity Balance Sheets:** Long-term project budget targets with estimated completion dates and dedicated point-in-time participant equity balance sheets (`GET /projects/{id}/settlement`), tracking effective funding vs assigned liability.
- **Expense Tracking & Full Management:** Log and manage shared/personal expenses with split percentage overrides, project allocations, and tag associations. Features a live search/filter toolbar, quick inline tag popover assignment, a full-featured Edit Expense modal, and date-locking for settled historical months.
- **Bank Statement CSV Importer & Duplicate Detection:** Upload and review bank statements (supporting ING Belgium format with KBC selection). Features clean beneficiary extraction from Debit Mastercard, Bancontact, SEPA Direct Debits, and credit transfers. Implements dual-stream routing (positive credits to Income, debits to Expenses), real-time category auto-cascading per beneficiary, 1-click micro-expense bundling, and strict categorization validation (no default fallback to "other"). The CSV parser runs in the browser as WebAssembly; a server-side duplicate check (same category and cent amount in the same month) runs when you press Import and on manual entry, with a confirmation prompt to cancel, add anyway, or skip duplicates.
- **Centralized Settings & Personalization:** Comprehensive settings panel managing household members and color palettes, feature modules, navigation tab visibility, entry defaults with 1-click currency presets (€, $, £, CHF, ¥, kr), chart & split visualization styles, mobile layout density, and zero-knowledge encrypted database backups.
- **Zero-Knowledge Privacy:** Client-side AES-GCM 256-bit encryption (via Web Crypto API) ensures all names, descriptions, notes, and category labels are stored encrypted at rest on the server, with secure in-place server-side database export/import utilities.
- **Modern UI & Design System:** Modern typography powered by Plus Jakarta Sans and JetBrains Mono, `tabular-nums` formatting on all currency and financial amounts, and glassmorphic depth.

---

## 🛠 Tech Stack

**Backend:**
- Rust (axum, tokio), organised as a Cargo workspace of small crates (`domain`, `auditor`, `store`, `api`, `wasm`)
- DuckDB 2.0 (embedded, single file, one serialised writer, pooled readers); native SQL, no ORM
- The **Auditor**: settlements, deposit proposals, budgets, income, split resolution, cash flow and project equity are calculated in-process, verified against invariants, and returned with an input hash
- clippy-enforced small methods (≤ 20 lines, ≤ 4 parameters, low cognitive complexity)

**Frontend:**
- Vanilla Svelte (Flat component architecture, no heavy SSR framework)
- Tailwind CSS (Utility-first styling, glassmorphism tokens)
- Chart.js (Native Canvas 2D rendering)
- Vite build tool & development server
- Rust-to-WASM CSV statement parser
- Vitest with `@testing-library/svelte` and JSDOM

**Infrastructure & Orchestration:**
- Docker Compose
- Caddy Server (Reverse Proxy, automatic HTTPS/TLS termination, and WebSocket proxying)
- SonarQube static code quality analysis (optional)

---

## 📦 Installation & Setup

### Option 1: Docker Compose (Recommended Cluster Setup)
You can run the full production-like stack using Docker Compose. The setup consists of containers in a shared bridge network (`app-network`):
- **`finance-tracker-backend`**: Runs the Rust API on port 8000; the DuckDB file lives in `./data` (bind-mounted at `/data`).
- **`finance-tracker-frontend`**: Serves the built Svelte application on port 5173.
- **`finance-tracker-caddy`**: Serves as the single entry gateway, binding host ports `80` and `443`.
- **`finance-tracker-sonarqube`**: Local SonarQube server on port 9000 (for quality scans).

#### The Role of Caddy Reverse Proxy
Caddy orchestrates routing and traffic control for the cluster:
- **TLS Termination:** Automatically provisions and renews SSL certificates for `jizifin.duckdns.org` over HTTPS, while serving `http://localhost` and `http://127.0.0.1` over HTTP for local development.
- **API Routing:** Proxies all paths matching `/api/*` to the backend service at `http://backend:8000` (stripping the `/api` prefix).
- **Frontend Routing:** Proxies all other requests to the frontend service at `http://frontend:5173`.
- **WebSocket Upgrade:** Forwards HTTP connection upgrade headers automatically, allowing client WebSockets to connect to `/ws/finance` transparently.

#### Running the Cluster
1. Ensure ports 80 and 443 are free.
2. Run the build and start command from the project root:
   ```bash
   docker compose up --build -d
   ```
3. Access the application at `https://jizifin.duckdns.org` in production or `http://localhost` when running locally.
4. Inspect logs using:
   ```bash
   docker compose logs -f
   ```

---

### Option 2: Local Development Setup

Everything runs in Docker; host `cargo`/`npm` are not required.

```bash
make wasm        # build the CSV parser into frontend/src/lib/wasm
make dev         # dev stack with hot reload
make lint-backend
```

Deploying a new backend swaps the running container only when the image build succeeds; Caddy retries requests for up to 30 seconds during the swap:

```bash
make deploy-backend
```

---

## 🧪 Testing & Quality Assurance

- **Lint and type-check (Rust):** `make lint-backend` runs clippy with `-D warnings` over the whole workspace.
- **Backend tests:** `make test-backend` runs `cargo nextest` in the `jizifin-backend-test` image. The Rust test suite is still to be written (plan §12); there are currently no backend tests.
- **Frontend tests:** `make test-frontend` runs Vitest (`-- --run`). Several suites still target modules that were removed in the thin-client rewrite and are scheduled for deletion (plan §12.6).

See [docs/RUST_MIGRATION_PLAN.md](docs/RUST_MIGRATION_PLAN.md) for the full test plan and [.agents/rules/](.agents/rules/) for workspace execution rules.

---

## 🛠️ Developer CLI & Makefile Commands

A unified [Makefile](file:///home/jim/Documents/jizifin/Makefile) is provided for easy local and remote operations:

| Command | Action | Description |
| :--- | :--- | :--- |
| `make help` | Help overview | Displays all available targets and descriptions. |
| `make build` | Build Production | Builds the production images (frontend + Rust backend). |
| `make run` / `make up` | Start Production | Starts `backend`, `frontend`, and `caddy` in background. |
| `make dev` | Start Development | Starts dev stack with hot-reloading and live code volume mounting. |
| `make full` | Start Stack + SonarQube | Starts all services plus SonarQube via Compose profile `full`. |
| `make stop` / `make down` | Stop Stack | Stops all active containers. |
| `make clean` | Clean Workspace | Stops containers and purges build caches. |
| `make test` | Run All Tests | Executes the backend (nextest) and frontend (vitest) test suites. |
| `make lint-backend` | Lint Rust | Clippy with `-D warnings` over the workspace. |
| `make wasm` | Build WASM | Builds the CSV parser into `frontend/src/lib/wasm`. |
| `make deploy-backend` | Swap backend | Builds, then replaces the running backend only on success. |
| `make logs` | Stream Logs | Streams unified log output from all services. |
| `make deploy-macbook` | Deploy to MacBook | Pulls latest commit and triggers production rebuild on MacBook server. |

---

## 🏗 Architecture & Design Principles

- **Integer Cents Precision:** All currencies are represented as whole `INTEGER` cents at the database layer to eliminate floating-point rounding errors. Decimal formatting occurs strictly at the presentation boundary.
- **Mathematical Exactness:** Split distributions use Hare-Niemeyer / Largest Remainder with a signed floor and SHA-256 tie-breaking salted by the entry id and member id.
- **No ORMs:** Repositories execute native DuckDB SQL directly.
- **Auditor:** Contentious maths lives only in the in-process Auditor; the browser does no money arithmetic.
- **Zero-Knowledge Privacy:** Client derives a 256-bit AES-GCM key from the user passphrase. Sensitive text columns are encrypted before transmission. Deterministic encryption enables exact matching, joins and grouping without plaintext exposure on the server disk.

---

## 📜 License
Private.

