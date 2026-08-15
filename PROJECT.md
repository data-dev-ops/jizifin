# Personal Finance Tracker Test Suite Project

## Architecture
- **Backend**: FastAPI / Python 3.14 / SQLite (`aiosqlite` WAL mode) / `pytest` + `httpx` test harness in `backend/tests/`
- **Frontend**: Svelte / Tailwind CSS / Chart.js / `vitest` + JSDOM test suite in `frontend/src/test/`
- **Cryptography**: AES-GCM 256-bit with PBKDF2 (100,000 iterations, static salt `"jizifin-salt-pbkdf2"`, static IV `"jizifin-cryp"`)

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | Test Architecture & Matrix Analysis | Analyze codebase & map 90+ specifications to test suites | None | DONE |
| 2 | Backend Pytest Harness & Test Suite | Implement `backend/tests/conftest.py` & Pytest files for specs | M1 | DONE |
| 3 | Frontend Vitest Test Suite Enhancement | Implement/enhance Vitest test files for specs under `frontend/src/test/` | M1 | DONE |
| 4 | Integration Verification & Forensic Audit | Run 100% full test suites, verify zero regressions, conduct audit | M2, M3 | DONE |

## Interface Contracts
### Backend Pytest Fixtures (`backend/tests/conftest.py`)
- `async_client`: FastAPI `httpx.AsyncClient` initialized with active app & test DB
- `db_session`: `aiosqlite` connection with in-memory SQLite, initialized WAL & FKs, tables/views
- `auth_headers` / `crypto_helpers`: AES-GCM helper functions matching client-side deterministic encryption

### Frontend Vitest Setup (`frontend/src/test/setup.js`)
- `globalThis.crypto` WebCrypto PBKDF2/AES-GCM polyfill
- JSDOM DOM mocks (Canvas 2D context, ResizeObserver, matchMedia, fetch router)

## Code Layout
- `backend/tests/`
  - `conftest.py`
  - `test_numerical_precision.py`
  - `test_ledger_transfers.py`
  - `test_splits_allocations.py`
  - `test_currency_exchange.py`
  - `test_budgeting_engine.py`
  - `test_networth_reporting.py`
  - `test_recurrence_scheduling.py`
  - `test_mutations_reclassification.py`
  - `test_pending_reconciliation_locking.py`
  - `test_categories_tags.py`
  - `test_concurrency_security.py`
  - `test_import_export_extended.py`
- `frontend/src/test/`
  - `setup.js`
  - `crypto.test.js`
  - `api.test.js`
  - `colorUtils.test.js`
  - `components/*.test.js`
  - `numerical_precision.test.js`
  - `ledger_transfers.test.js`
  - `budgeting_engine.test.js`
  - `networth_reporting.test.js`
  - `recurrence_scheduling.test.js`
  - `mutations_reclassification.test.js`
  - `categories_tags.test.js`
