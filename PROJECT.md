# Jizifin: project status

**Stack:** Rust (axum) + embedded DuckDB 2.0, in-process Auditor, thin Svelte client with a WASM CSV parser. See [AGENTS.md](AGENTS.md) for the working rules and [docs/RUST_MIGRATION_PLAN.md](docs/RUST_MIGRATION_PLAN.md) for the design, API tables and history.

## Milestones

| Phase | Scope | Status |
| :--- | :--- | :--- |
| A | Rust/DuckDB backend, Auditor, thin frontend, deploy wiring, spike gate, reference capture | Done |
| C | Cutover: legacy SQLite data migrated, Rust stack serving the public ports, legacy code removed (`git show legacy-python`) | Done 2026-10-26 |
| B | Rust test suite (Auditor unit tests, API integration tests, WASM tests), frontend suite cleanup, parity tests against `backend/tests/reference/legacy/` | Open |
| D | Lakehouse (`ducklake`); blocked until a DuckDB crate release has published extensions | Future |

## Known gaps

- No Rust tests yet; the Vitest suite still has suites for removed modules (plan §12.6).
- The captured reference data has no income, settlements or joint accounts, so those paths need hand-built fixtures.
- In-app documentation pages were rewritten for the new stack but should be re-read against the code when Phase B starts.
