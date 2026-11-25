# Developer entry points.
#
# All targets perform real work: no placeholders and no silent fallbacks. The
# Compose targets run with `--env-file`, resolved once below, so the commands
# work on a fresh checkout (`make env` creates `.env` from `.env.example`).

SHELL := /bin/bash
.DEFAULT_GOAL := help

COMPOSE := docker compose

# Prefer a real `.env` (with secrets) and fall back to the placeholder file so a
# fresh clone can still build, run and be validated.
ENV_FILE := $(if $(wildcard .env),$(if $(shell [ -s .env ] && echo yes),.env,.env.example),.env.example)
COMPOSE_ENV := --env-file $(ENV_FILE)

# Publish the dependency ports on loopback for host-based development.
COMPOSE_DEV := -f compose.yaml -f compose.dev.yaml
COMPOSE_DEV_ENV := -f compose.yaml -f compose.dev.yaml $(COMPOSE_ENV)

.PHONY: help env up down restart logs logs-backend logs-frontend ps \
	dev-deps migrate test lint build fmt check clean clean-volumes \
	openapi backend frontend smoke

help: ## Show this help
	@grep -hE '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) \
		| sort \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'

env: ## Create .env from .env.example when .env does not exist yet
	@if [ -s .env ]; then \
		echo ".env already exists and is not empty; leaving it untouched."; \
	else \
		cp .env.example .env; \
		echo "Created .env from .env.example. Review it before exposing the stack."; \
	fi

up: ## Build and start the full stack (postgres, redis, backend, frontend)
	$(COMPOSE) $(COMPOSE_ENV) up --build --detach --wait

down: ## Stop and remove the containers, keeping the named volumes
	$(COMPOSE) $(COMPOSE_ENV) down --remove-orphans

restart: ## Recreate every service
	$(COMPOSE) $(COMPOSE_ENV) up --build --detach --wait --force-recreate

ps: ## Show service and health status
	$(COMPOSE) $(COMPOSE_ENV) ps

logs: ## Follow the logs of every service
	$(COMPOSE) $(COMPOSE_ENV) logs --follow --tail=100

logs-backend: ## Follow the backend logs
	$(COMPOSE) $(COMPOSE_ENV) logs --follow --tail=100 backend

logs-frontend: ## Follow the frontend logs
	$(COMPOSE) $(COMPOSE_ENV) logs --follow --tail=100 frontend

dev-deps: ## Start only PostgreSQL and Redis, published on loopback
	$(COMPOSE) $(COMPOSE_DEV_ENV) up --detach --wait postgres redis

migrate: ## Apply migrations from the host against the dev dependencies
	cargo run --quiet --package platform-backend --bin platform-migrate

test: ## Run the backend and frontend test suites
	cargo test --workspace
	pnpm --filter @platform/frontend test

lint: ## Run Clippy, rustfmt check, ESLint and Prettier check
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	cargo fmt --all -- --check
	pnpm --filter @platform/frontend typecheck
	pnpm --filter @platform/frontend lint
	pnpm --filter @platform/frontend format:check

build: ## Build the backend workspace and the frontend bundle
	cargo build --workspace --all-targets
	pnpm --filter @platform/frontend build

fmt: ## Format Rust and frontend sources
	cargo fmt --all
	pnpm --filter @platform/frontend format

check: lint test ## Run every check required before a commit

clean: ## Remove build artifacts. Volumes and data are preserved
	cargo clean
	rm -rf apps/frontend/dist apps/frontend/node_modules/.vite
	@echo "Build artifacts removed. PostgreSQL and Redis volumes were NOT touched."
	@echo "Use 'make clean-volumes' to delete them explicitly."

clean-volumes: ## Delete the PostgreSQL and Redis volumes (destroys data)
	@echo "This permanently deletes the PostgreSQL and Redis data volumes:"
	@echo "  platform-postgres-data"
	@echo "  platform-redis-data"
	@read -r -p "Type 'delete-platform-data' to continue: " answer; \
	if [ "$$answer" != "delete-platform-data" ]; then \
		echo "Aborted. Nothing was deleted."; \
		exit 1; \
	fi
	$(COMPOSE) $(COMPOSE_ENV) down --volumes --remove-orphans
	docker volume rm -f platform-postgres-data platform-redis-data

openapi: ## Regenerate the OpenAPI document and the frontend types
	cargo run --quiet --package platform-backend --bin platform-openapi -- apps/frontend/src/shared/api/generated/openapi.json
	pnpm --filter @platform/frontend api:types

smoke: ## Verify a running stack end to end (API, OpenAPI, WebSocket, frontend)
	node tests/integration/smoke.mjs

backend: ## Run the backend on the host against the dev dependencies
	cargo run --package platform-backend

frontend: ## Run the frontend dev server on the host
	pnpm --filter @platform/frontend dev
