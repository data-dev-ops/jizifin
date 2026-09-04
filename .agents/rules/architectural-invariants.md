---
trigger: always_on
description: Establishes non-negotiable architectural, cryptographic (Zero-Knowledge Base64URL), database, and testing standards for Jizifin.
---

# 🏛️ Jizifin Architectural & Code Invariants

All developers and AI agents must strictly adhere to these zero-regression technical rules, database schemas, and cryptographic boundaries.

---

## 🔒 1. Client-Server Cryptographic Split (Zero-Knowledge Privacy)

1. **Key Derivation & Static IV**:
   - 256-bit AES-GCM `CryptoKey` derived from user passphrase using PBKDF2 (100,000 iterations, SHA-256, static salt `"jizifin-salt-pbkdf2"`).
   - Static 12-byte IV: `"jizifin-cryp"` (`[106, 105, 122, 105, 102, 105, 110, 45, 99, 114, 121, 112]`).
2. **Mandatory Base64URL Encoding**:
   - **Never use standard Base64**. Standard Base64 characters (`+`, `/`, `=`) corrupt HTTP query parameters and URL paths.
   - All encrypted strings must be encoded in **Base64URL without padding** (`-` instead of `+`, `_` instead of `/`, no `=`).
3. **Deterministic Querying Invariant**:
   - **Permitted on Encrypted Columns**: Exact matches (`col = ?`, `IN (...)`), equality joins (`ON a.col = b.col`), `GROUP BY`, and foreign key cascade.
   - **FORBIDDEN on Encrypted Columns**: Range filters (`<`, `>`, `BETWEEN`), `LIKE` wildcards, and text collation `ORDER BY`.
4. **Plaintext Boundary**:
   - Amounts (`cost_cents`, `amount_cents`), dates (`YYYY-MM-DD`, `YYYY-MM`), integer IDs, foreign keys, and booleans (`is_joint`, `is_active`) remain in plaintext to permit indexing and mathematical aggregation.

---

## 🖥️ 2. Database & SQL Rules

1. **No ORMs**: All endpoints must write native, optimized, ANSI-compliant SQL directly via `aiosqlite`.
2. **SQLite WAL & Foreign Keys**:
   - Always enforce `PRAGMA journal_mode=WAL;` and `PRAGMA foreign_keys=ON;`.
   - Avoid long-lived locks; ensure database connections close cleanly.
3. **Currency Invariant**:
   - Currency is represented **strictly as `INTEGER` cents** at the database, backend model, and API layer.
   - Decimal representations (`cents / 100.0`) are allowed exclusively at the UI presentation boundary.
4. **Date Invariant & SARGability**:
   - Dates must be formatted as ISO `YYYY-MM-DD` (and `YYYY-MM` for settlement logs).
   - Maintain query SARGability: Use index range scans (`WHERE expense_date >= 'YYYY-MM-01' AND expense_date <= 'YYYY-MM-31'`) rather than calling SQL functions like `strftime()` on index columns.

---

## 🌐 3. Frontend Svelte & Styling Guidelines

1. **State**: Svelte writable stores (`stores.js`) serve as the reactive data bridge.
2. **Styling**: Exclusively utility-first Tailwind CSS. Scoped `<style>` blocks are prohibited unless strictly necessary for canvas elements or keyframes.
3. **Visualization**: Raw Chart.js rendered on `<canvas>` elements and updated via `chart.update()`. Do not introduce heavy wrapper libraries.
4. **Error Handling**: `request()` in `api.js` throws standard `Error` objects on non-2xx status codes; components catch and bind `err.message` to local reactive error banners.

---

## 🧮 4. Complex Domain Invariants

1. **Signed Hare-Niemeyer Cent Allocation**:
   - Cent shares are computed via mathematical floor (`math.floor(share)`), distributing remainder cents in descending order of remainder.
   - Deterministic tie-breaking uses SHA-256 of `f"{tx_salt}:{user}"` to prevent alphabetical bias.
2. **SCD2 Split Agreements**:
   - Active split agreements take precedence over baseline agreements for their specific timeline window (`start_date` to `end_date`).
3. **Connected-Component Graph Isolation**:
   - Decomposes household members into disjoint subgraphs based on transaction splits before executing greedy debt simplification, ensuring debts never cross outside participating groups.
4. **Tag Active Window Boundaries**:
   - Validates that `tag.start_date <= expense.expense_date <= tag.end_date`. Violations return `HTTP 422 Unprocessable Content`.
   - Timeline shrinkage is rejected if existing assigned transactions would be orphaned outside proposed dates.

---

## 🛡️ 5. Zero-Regression & Data Sanitization Mandate

1. **Test Verification Scope**:
   - Running full test suites is required **only** when modifying application source code (`backend/app`, `frontend/src/lib`), API schemas, database DDL/migrations, or core dependencies.
   - **DO NOT run test suites** when the user requests documentation updates, markdown edits, git commits, repository rule adjustments, or local exploration/inspection.
   - When required, test verification must confirm:
     - Backend: All 342 pytest tests across 21 test files pass.
     - Frontend: All 384 vitest tests across 43 test files pass.
2. **Data Sanitization Rule**:
   - **Never** commit real personal names, production databases (`finance.db`), or real financial statements to git.
   - Integration tests must strictly utilize the sanitized mock database (`backend/tests/test.db`) and portable fixtures (`frontend/src/test/fixtures/`).
