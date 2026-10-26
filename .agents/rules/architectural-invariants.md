---
trigger: always_on
description: Non-negotiable architectural, cryptographic (zero-knowledge Base64URL), database and verification standards for Jizifin.
---

# 🏛️ Jizifin Architectural & Code Invariants

Authoritative detail lives in `AGENTS.md` and `docs/RUST_MIGRATION_PLAN.md`; these are the rules that must never be broken.

## 🔒 1. Cryptographic split (zero-knowledge)

1. AES-256-GCM key from the passphrase via PBKDF2 (100,000 iterations, SHA-256, static salt `"jizifin-salt-pbkdf2"`); static 12-byte IV `"jizifin-cryp"`.
2. Ciphertext is **Base64URL without padding**. Never standard Base64 (`+`, `/`, `=` corrupt URLs).
3. On encrypted (`*_ct`) columns only equality, `IN`, joins and `GROUP BY` are allowed. Range filters, `LIKE` and collation `ORDER BY` are forbidden; sorting by name happens in the browser.
4. Amounts, dates, ids and flags are plaintext so they can be indexed and aggregated.
5. The server never learns the passphrase: login uses `encryptText("FinanceTrackerAuth")` as proof and the server stores only its SHA-256.

## 🖥️ 2. Backend and database

1. DuckDB only, native SQL in repositories, no ORM, no other engine, no fallback.
2. Money is integer cents (`BIGINT` / `Cents(i64)`). Decimal formatting happens only in the browser.
3. Dates are `DATE`; keep `entry_date` filters SARGable (compare to bounds, no function on the column).
4. No foreign keys (DuckDB blocks updating a UNIQUE column of a referenced parent); repositories enforce integrity. Writes go through `Database::exclusive()`.
5. Contentious calculations live only in `jizifin-auditor` and return `AuditedResult` with verified invariants. Plain aggregates are `/views/*`.
6. Clippy limits are hard gates: ≤ 20 lines per function, ≤ 4 parameters, cognitive complexity ≤ 8, all warnings denied.
7. No legacy and no redundancy: no shims, no dead code, no duplicated helpers.

## 🌐 3. Frontend

1. The browser does no money arithmetic: no sums, averages, percentages or rounding of amounts.
2. State in Svelte stores (`stores.js`); styling only with Tailwind utilities (scoped `<style>` only for canvas/keyframes); raw Chart.js on `<canvas>`.
3. `request()` in `api.js` throws `ApiError` with the server's `detail`; components bind `err.message` to banners.

## 🧮 4. Domain invariants

1. Signed Hare-Niemeyer allocation with `.floor()`; ties broken by SHA-256 of `"{seed}:{member_id}"`.
2. Share precedence: personal pay, entry override, income ratio, agreement in force (bounded beats open-ended), equal split.
3. Settlements are isolated per connected component and never include joint-account expenses.
4. Tag windows: `tag.valid_from <= entry.date <= tag.valid_to`, violations are 422; a tag window may not shrink past assigned entries.
5. Sealed months reject entry writes (409) and report drift against the stored result.

## 🛡️ 5. Verification and data hygiene

1. Run `make lint-backend` after Rust changes and the production frontend build after frontend changes. Do not run test suites for documentation-only or git-only requests.
2. The Rust test suite is not written yet (plan §12); do not claim tests passed that do not exist.
3. Never commit real personal names, production data files (`finance.db`, `*.duckdb`, `backups/`) or the passphrase. Reference captures stay git-ignored.
