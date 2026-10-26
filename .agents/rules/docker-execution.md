---
trigger: always_on
description: Mandates Docker-first execution so builds and checks do not depend on host tooling.
---

# 🐳 Docker Execution Rules

Host `cargo`, `npm` and `uv` may be missing or differ in version. Run everything through Docker (or the `make` targets that wrap it) from the repo root.

| Task | Command |
| :--- | :--- |
| Rust lint + type-check | `make lint-backend` |
| Rust tests | `make test-backend` (builds `jizifin-backend-test` with cargo-nextest) |
| WASM CSV parser (before any frontend build, dev run or test) | `make wasm` |
| Frontend tests | `make test-frontend` (Vitest needs `-- --run`; without it watch mode hangs) |
| Frontend build | see `AGENTS.md` §1 |
| Deploy backend | `make deploy-backend` |
| Whole stack | `docker compose up -d backend frontend caddy` |

Tips:
- Mount the named volumes `jizifin-cargo-registry` and `jizifin-cargo-target` to keep Rust builds incremental (the Makefile targets already do).
- Run `touch backend/crates/*/src/lib.rs backend/crates/*/src/main.rs` before clippy if a Docker volume keeps stale fingerprints.
- DuckDB is built from source on the first Rust build; a cold image build takes about 20 minutes, a warm one about 10.
- The live data file is `./data/finance.duckdb`. Never run experiments against it; copy it or migrate into a scratch directory.
- On Windows PowerShell, replace `$(pwd)` with `${PWD}` in the commands in the Makefile.
