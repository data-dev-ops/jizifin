---
trigger: always_on
description: Mandates Docker-first execution across all platforms (Linux, macOS, Windows/WSL2) to eliminate host dependency drift and command failures.
---

# 🐳 Universal Docker Execution Rules

To eliminate host dependency drift, missing executables (e.g. `npm: command not found`, missing `uv`), and PATH inconsistencies across Linux, macOS, and Windows/WSL2, **Docker containers are the primary, default execution standard** for this workspace.

---

## ⚙️ 1. Execution Priority Hierarchy

1. **Priority 1 (Universal Default - All Platforms)**:
   Pre-built Docker test containers with live volume mounting. Guarantees identical Python 3.14 and Node 22 runtime parity without requiring host tooling:
   - Backend Container: `jizifin-backend-test`
   - Frontend Container: `jizifin-frontend-test`
2. **Priority 2 (Secondary Convenience Only)**:
   Local host CLI (`uv run ...`, `npm ...`) **only** when verified present on `PATH` in the current shell. Never hardcode user-specific paths (e.g. `~/.nvm/...`).
3. **Cluster Orchestration**:
   Multi-container stack (`backend`, `frontend`, `caddy`, `sonarqube`) runs via `docker compose`. Ignore `sonarqube` unless requested.

---

## 📦 2. One-Time Test Container Build

On a fresh clone or whenever dependencies in `pyproject.toml` or `package.json` are modified, build or rebuild the test images:

```bash
# Build Backend Test Image (Python 3.14 + Pytest + uv dependencies)
docker build -t jizifin-backend-test -f backend/Dockerfile.test backend/

# Build Frontend Test Image (Node 22 + Vite + Vitest + dependencies)
docker build -t jizifin-frontend-test frontend/
```

---

## 🧪 3. Live Volume-Mounted Test Commands

Mount working source directories into the container so test suites immediately run against live edits without container rebuilds:

### Backend Pytest (342 Tests)
- **Linux / macOS / WSL (Bash/Zsh)**:
  ```bash
  docker run --rm -v $(pwd)/backend/app:/app/app -v $(pwd)/backend/tests:/app/tests jizifin-backend-test pytest
  ```
- **Windows (PowerShell)**:
  ```powershell
  docker run --rm -v ${PWD}/backend/app:/app/app -v ${PWD}/backend/tests:/app/tests jizifin-backend-test pytest
  ```
- **Single Test Suite / Targeted Run**:
  ```bash
  docker run --rm -v $(pwd)/backend/app:/app/app -v $(pwd)/backend/tests:/app/tests jizifin-backend-test pytest tests/test_scenarios_integration.py
  ```

### Frontend Vitest (384 Tests)
> [!IMPORTANT]
> **Mandatory `--run` Flag**: Always pass `-- --run` to Vitest in containers or non-interactive shells. Omitting `--run` launches interactive watch mode, hanging execution indefinitely.

- **Linux / macOS / WSL (Bash/Zsh)**:
  ```bash
  docker run --rm -v $(pwd)/frontend/src:/app/src -v $(pwd)/frontend/index.html:/app/index.html -v $(pwd)/frontend/tailwind.config.js:/app/tailwind.config.js -v $(pwd)/frontend/vite.config.js:/app/vite.config.js jizifin-frontend-test npm test -- --run
  ```
- **Windows (PowerShell)**:
  ```powershell
  docker run --rm -v ${PWD}/frontend/src:/app/src -v ${PWD}/frontend/index.html:/app/index.html -v ${PWD}/frontend/tailwind.config.js:/app/tailwind.config.js -v ${PWD}/frontend/vite.config.js:/app/vite.config.js jizifin-frontend-test npm test -- --run
  ```
- **Single Component / File Run**:
  ```bash
  docker run --rm -v $(pwd)/frontend/src:/app/src -v $(pwd)/frontend/index.html:/app/index.html -v $(pwd)/frontend/tailwind.config.js:/app/tailwind.config.js -v $(pwd)/frontend/vite.config.js:/app/vite.config.js jizifin-frontend-test npx vitest run src/test/components/ExpenseForm.test.js
  ```

---

## 🚀 4. Full-Stack Development Cluster

To spin up the entire application stack:
```bash
docker compose up --build -d backend frontend caddy
```
- Web Application: `http://localhost` (or `https://localhost`)
- API Direct Access: `http://localhost:8000`
- Vite Dev Server: `http://localhost:5173`
