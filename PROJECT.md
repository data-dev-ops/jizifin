# Personal Finance Tracker Test Suite & Project Architecture

## Architecture
- **Backend**: FastAPI / Python 3.14 / SQLite (`aiosqlite` WAL mode) / `pytest` + `httpx` test harness in `backend/tests/`
- **Frontend**: Svelte / Tailwind CSS / Chart.js / `vitest` + JSDOM test suite in `frontend/src/test/`
- **Cryptography**: AES-GCM 256-bit with PBKDF2 (100,000 iterations, SHA-256, static salt `"jizifin-salt-pbkdf2"`, static IV `"jizifin-cryp"`)
- **Execution Strategy**:
  1. Primary Host CLI (`uv run --directory backend ...` / `npm --prefix frontend ...`)
  2. NVM fallback for Node/NPM
  3. Pre-built Docker test containers (`jizifin-backend-test`, `jizifin-frontend-test`)

---

## Milestones

| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | Test Architecture & Matrix Analysis | Analyze codebase & map 90+ specifications to test suites | None | DONE |
| 2 | Backend Pytest Harness & Core Suites | Implement `conftest.py`, crypto test helpers, & core Pytest files | M1 | DONE |
| 3 | Frontend Vitest Test Suite Enhancement | Implement Vitest unit & component test files under `frontend/src/test/` | M1 | DONE |
| 4 | Integration Verification & Forensic Audit | Run 100% test suites, verify zero regressions, conduct audit | M2, M3 | DONE |
| 5 | Multi-Joint Accounts & SCD2 Agreements | Support multiple isolated joint accounts and SCD2 split timeline overrides | M4 | DONE |
| 6 | Project Equity Settlements & Tag Windows | Backend-driven project liability/funding balance sheets & tag date bounds | M5 | DONE |
| 7 | Shared Mock DB & Sanitized Integration Harness | Build `generate_test_db.py`, git-track `test.db`, eradicate real user data | M6 | DONE |
| 8 | Bank CSV Importer & Duplicate Detection | Client-side bank statement parser, batch creation, & duplicate prompt | M7 | DONE |

---

## Interface Contracts

### Backend Pytest Fixtures (`backend/tests/conftest.py`)
- `test_db`: Isolated temporary SQLite database file initialized with schema & views per unit test
- `client`: FastAPI `httpx.AsyncClient` dependency-overridden with isolated `test_db`
- `integration_db`: Isolated per-test clone of the pre-populated `backend/tests/test.db` database
- `integration_client`: FastAPI `httpx.AsyncClient` dependency-overridden with `integration_db`
- `crypto_helpers`: `derive_key(password)`, `encrypt_text(plaintext, key)`, `decrypt_text(ciphertext, key)` matching client-side AES-GCM

### Frontend Vitest Setup (`frontend/src/test/setup.js`)
- `globalThis.crypto` WebCrypto PBKDF2 / AES-GCM polyfill
- JSDOM DOM mocks (Canvas 2D context, ResizeObserver, matchMedia, fetch router)
- Svelte component testing via `@testing-library/svelte`

---

## Code Layout & Test Catalog

### Backend Test Suite (342 Tests / 21 Test Files)
Directory: `backend/tests/`

- **Harness & Database**:
  - `conftest.py`: Async client, database fixtures, crypto routines
  - `generate_test_db.py`: Deterministic generator for `test.db`
  - `test.db`: Git-tracked populated SQLite database (5 users, 2 joint accounts, 5 scenario months)
- **Domain & Specification Suites**:
  - `test_budgeting_engine.py` (28 tests): Monthly category budget limits, rolling thresholds, alerts
  - `test_categories_tags.py` (15 tests): Category creation, metadata, and tag tagging
  - `test_category_rename_delete.py` (5 tests): `ON UPDATE CASCADE` renames and delete protection (HTTP 409)
  - `test_concurrency_security.py` (13 tests): Concurrent write transactions, WAL locks, SQL injection prevention
  - `test_currency_exchange.py` (13 tests): Multi-currency conversions, base currency representations
  - `test_database_init.py` (4 tests): DDL migrations, views regeneration, PRAGMA verification
  - `test_expenses_batch.py` (3 tests): Batch expense creation with encryption and category validation
  - `test_import_export_analytics.py` (30 tests): AES-GCM encrypted database export, streaming, and import
  - `test_jobs_and_salary.py` (5 tests): Employment contracts, frequency normalization, salary overrides
  - `test_ledger_transfers.py` (20 tests): Direct user transfers, bank adjustments, balance tracking
  - `test_multi_household_couples.py` (5 tests): Multi-couple isolation and co-housing settlements
  - `test_mutations_reclassification.py` (16 tests): Transaction modifications, category reassignment
  - `test_networth_reporting.py` (20 tests): Net worth aggregation, assets vs liabilities
  - `test_numerical_precision.py` (22 tests): Signed Hare-Niemeyer floor math, deterministic tie-breakers
  - `test_reconciliation_locking.py` (15 tests): Month settlements, locked period mutation rejection (HTTP 400)
  - `test_recurrence_scheduling.py` (17 tests): Recurring expense templates, DOM generation
  - `test_scd2_category_splits.py` (3 tests): SCD2 date-bounded timeline agreements & overrides
  - `test_scenarios_integration.py` (12 tests): Multi-scenario integration tests against `test.db`
  - `test_spec_financial_cases.py` (102 tests): Exhaustive financial edge cases & boundary conditions
  - `test_splits_allocations.py` (16 tests): Basis-point proportional allocations, validation

### Frontend Test Suite (384 Tests / 43 Test Files)
Directory: `frontend/src/test/`

- **Core & Domain Logic Suites (19 files)**:
  - `setup.js`: Polyfills and environment setup
  - `api.test.js` (17 tests): Central HTTP client, automatic encryption/decryption, error propagation
  - `crypto.test.js` (12 tests): PBKDF2 key derivation, AES-GCM static IV, Base64URL encoding
  - `csvParser.test.js` (14 tests): RFC 4180 tokenizer, ING Belgium parsing, category cascading
  - `duplicateDetector.test.js` (7 tests): Category and cent amount duplicate detection matching
  - `colorUtils.test.js` (7 tests): HSL color generation and contrast utilities
  - `numerical_precision.test.js` (5 tests): Float-to-cent conversion, currency display formatting
  - `ledger_transfers.test.js` (19 tests): Client ledger mutations and store synchronization
  - `budgeting_engine.test.js` (22 tests): Client-side budget progress tracking
  - `networth_reporting.test.js` (18 tests): Assets, liabilities, and net worth summaries
  - `recurrence_scheduling.test.js` (6 tests): Recurring expense schedules
  - `mutations_reclassification.test.js` (4 tests): Expense editing in local reactive stores
  - `categories_tags.test.js` (9 tests): Dynamic tags and category filtering
  - `customization_personas.test.js` (7 tests): UI presets (Streamlined, High Legibility, Power User)
  - `device_profiles.test.js` (7 tests): Desktop vs Mobile local storage profile persistence
  - `import_export_analytics.test.js` (22 tests): Client backup and restore flows
  - `multicurrency.test.js` (18 tests): Currency symbols, decimal rounding settings
  - `reconciliation_locking.test.js` (10 tests): Historical month lock enforcement
  - `splits_allocations.test.js` (7 tests): Split allocation percentage validation
  - `concurrency_security.test.js` (3 tests): Client-side auth guard and token isolation
- **UI Component Suites (`frontend/src/test/components/`, 24 files)**:
  - `AnalyticsSummary.test.js` (7 tests)
  - `App.test.js` (3 tests)
  - `BankCsvImportModal.test.js` (6 tests)
  - `BudgetManager.test.js` (4 tests)
  - `Docs.test.js` (16 tests)
  - `DrilldownModal.test.js` (9 tests)
  - `EndToEndUIFlow.test.js` (2 tests)
  - `ExpenseForm.test.js` (10 tests)
  - `ExpenseList.test.js` (7 tests)
  - `IncomeChart.test.js` (2 tests)
  - `IncomeTab.test.js` (8 tests)
  - `JointAccountTab.test.js` (23 tests)
  - `Login.test.js` (5 tests)
  - `MultiHouseholdCouples.test.js` (11 tests)
  - `PaybackVisual.test.js` (6 tests)
  - `ProjectsTab.test.js` (4 tests)
  - `QueryConsole.test.js` (7 tests)
  - `RealtimeChart.test.js` (1 test)
  - `RecurringManager.test.js` (5 tests)
  - `ScenariosFrontendUI.test.js` (5 tests)
  - `SettingsTab.test.js` (9 tests)
  - `SplitManager.test.js` (10 tests)
  - `TagsTab.test.js` (5 tests)
  - `UserManager.test.js` (5 tests)
