# ==============================================================================
# Jizifin Management Makefile
# ==============================================================================

SHELL := /bin/bash
.DEFAULT_GOAL := help

COMPOSE_PROD := docker compose
COMPOSE_DEV  := docker compose -f docker-compose.yml -f docker-compose.dev.yml

MACBOOK_HOST := 192.168.0.128
MACBOOK_USER := jim
MACBOOK_PATH := /opt/jizifin

# ------------------------------------------------------------------------------
# Help
# ------------------------------------------------------------------------------
.PHONY: help
help: ## Show available commands
	@echo "========================================================================"
	@echo " Jizifin Personal Finance Tracker — CLI Commands"
	@echo "========================================================================"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# ------------------------------------------------------------------------------
# Build
# ------------------------------------------------------------------------------
.PHONY: build
build: ## Build production containers (hardened multi-stage Nginx frontend + FastAPI)
	$(COMPOSE_PROD) build

.PHONY: build-dev
build-dev: ## Build development containers
	$(COMPOSE_DEV) build

# ------------------------------------------------------------------------------
# Run / Start / Stop
# ------------------------------------------------------------------------------
.PHONY: run up
run: up ## Start production stack in background (backend, frontend, caddy)
up:
	$(COMPOSE_PROD) up -d

.PHONY: dev
dev: ## Start development stack with live reload & volume mounts
	$(COMPOSE_DEV) up

.PHONY: full
full: ## Start full stack including SonarQube (profile: full)
	$(COMPOSE_PROD) --profile full up -d

.PHONY: stop down
stop: down ## Stop running containers
down:
	$(COMPOSE_PROD) --profile full down

.PHONY: restart
restart: ## Restart production stack
	$(COMPOSE_PROD) restart

.PHONY: ps status
status: ps ## Show container status
ps:
	$(COMPOSE_PROD) --profile full ps

# ------------------------------------------------------------------------------
# Logs
# ------------------------------------------------------------------------------
.PHONY: logs
logs: ## Follow all container logs
	$(COMPOSE_PROD) logs -f

.PHONY: logs-frontend
logs-frontend: ## Follow frontend container logs
	$(COMPOSE_PROD) logs -f frontend

.PHONY: logs-backend
logs-backend: ## Follow backend container logs
	$(COMPOSE_PROD) logs -f backend

.PHONY: logs-caddy
logs-caddy: ## Follow Caddy container logs
	$(COMPOSE_PROD) logs -f caddy

# ------------------------------------------------------------------------------
# Testing
# ------------------------------------------------------------------------------
.PHONY: test
test: test-backend test-frontend ## Run both backend (pytest) and frontend (vitest) test suites

.PHONY: test-backend
test-backend: ## Run backend pytest suite
	@if command -v uv >/dev/null 2>&1; then \
		uv run --directory backend pytest; \
	elif docker image inspect jizifin-backend-test >/dev/null 2>&1; then \
		docker run --rm -v "$$(pwd)/backend/app:/app/app" -v "$$(pwd)/backend/tests:/app/tests" jizifin-backend-test pytest; \
	else \
		docker build -t jizifin-backend-test -f backend/Dockerfile.test backend/ && \
		docker run --rm -v "$$(pwd)/backend/app:/app/app" -v "$$(pwd)/backend/tests:/app/tests" jizifin-backend-test pytest; \
	fi

.PHONY: test-frontend
test-frontend: ## Run frontend vitest suite
	@if command -v npm >/dev/null 2>&1; then \
		npm --prefix frontend test -- --run; \
	elif docker image inspect jizifin-frontend-test >/dev/null 2>&1; then \
		docker run --rm -v "$$(pwd)/frontend/src:/app/src" -v "$$(pwd)/frontend/index.html:/app/index.html" -v "$$(pwd)/frontend/tailwind.config.js:/app/tailwind.config.js" -v "$$(pwd)/frontend/vite.config.js:/app/vite.config.js" jizifin-frontend-test npm test -- --run; \
	else \
		docker build -t jizifin-frontend-test frontend/ && \
		docker run --rm -v "$$(pwd)/frontend/src:/app/src" -v "$$(pwd)/frontend/index.html:/app/index.html" -v "$$(pwd)/frontend/tailwind.config.js:/app/tailwind.config.js" -v "$$(pwd)/frontend/vite.config.js:/app/vite.config.js" jizifin-frontend-test npm test -- --run; \
	fi

# ------------------------------------------------------------------------------
# Clean
# ------------------------------------------------------------------------------
.PHONY: clean
clean: ## Remove temporary build files, test caches, coverage reports, and stopped containers
	$(COMPOSE_PROD) --profile full down --remove-orphans
	rm -rf frontend/dist frontend/coverage backend/coverage.xml backend/.pytest_cache
	find . -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
	find . -type f -name "*.pyc" -delete 2>/dev/null || true

.PHONY: clean-all
clean-all: clean ## Deep clean including docker volumes and dangling images
	$(COMPOSE_PROD) --profile full down -v --remove-orphans
	docker image prune -f

# ------------------------------------------------------------------------------
# Remote Deployment (MacBook Server)
# ------------------------------------------------------------------------------
.PHONY: deploy-macbook
deploy-macbook: ## Deploy latest git commit and rebuild on MacBook server
	@echo "Deploying to $(MACBOOK_USER)@$(MACBOOK_HOST)..."
	ssh $(MACBOOK_USER)@$(MACBOOK_HOST) "export PATH=\"/opt/homebrew/bin:/opt/homebrew/sbin:\$$HOME/.orbstack/bin:\$$PATH\"; cd $(MACBOOK_PATH) && git pull && docker compose build && docker compose up -d"
