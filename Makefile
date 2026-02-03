# envtool — Environment File Generator CLI
# Common invocations for development and environment setup

ENVTOOL := cargo run --

# ─── Build & Test ────────────────────────────────────────────────

.PHONY: build
build: ## Build the project in release mode
	cargo build --release

.PHONY: dev
dev: ## Build the project in debug mode
	cargo build

.PHONY: test
test: ## Run all tests
	cargo test

.PHONY: check-code
check-code: ## Run clippy and format check
	cargo clippy -- -D warnings
	cargo fmt --check

.PHONY: fmt
fmt: ## Format all code
	cargo fmt

.PHONY: install
install: ## Install envtool to ~/.cargo/bin
	cargo install --path .

# ─── Schema Validation ──────────────────────────────────────────

.PHONY: check-frontend
check-frontend: ## Validate the frontend schema
	$(ENVTOOL) check -s config/frontend.env-schema.yaml

.PHONY: check-backend
check-backend: ## Validate the backend schema
	$(ENVTOOL) check -s config/backend.env-schema.yaml

.PHONY: check-all
check-all: check-frontend check-backend ## Validate all schemas

# ─── Environment Generation ─────────────────────────────────────

.PHONY: env-local
env-local: ## Generate all local .env files
	$(ENVTOOL) pull -s config/frontend.env-schema.yaml -e local --force
	$(ENVTOOL) pull -s config/backend.env-schema.yaml -e local --force

.PHONY: env-staging
env-staging: ## Generate all staging .env files
	$(ENVTOOL) pull -s config/frontend.env-schema.yaml -e staging --force
	$(ENVTOOL) pull -s config/backend.env-schema.yaml -e staging --force

.PHONY: env-production
env-production: ## Generate all production .env files
	$(ENVTOOL) pull -s config/frontend.env-schema.yaml -e production --force
	$(ENVTOOL) pull -s config/backend.env-schema.yaml -e production --force

# ─── Dry Runs ───────────────────────────────────────────────────

.PHONY: dry-run-local
dry-run-local: ## Preview local .env generation
	$(ENVTOOL) pull -s config/frontend.env-schema.yaml -e local --dry-run
	@echo ""
	$(ENVTOOL) pull -s config/backend.env-schema.yaml -e local --dry-run

.PHONY: dry-run-staging
dry-run-staging: ## Preview staging .env generation
	$(ENVTOOL) pull -s config/frontend.env-schema.yaml -e staging --dry-run
	@echo ""
	$(ENVTOOL) pull -s config/backend.env-schema.yaml -e staging --dry-run

# ─── Listing ────────────────────────────────────────────────────

.PHONY: list-frontend
list-frontend: ## List all frontend variables
	$(ENVTOOL) list -s config/frontend.env-schema.yaml

.PHONY: list-backend
list-backend: ## List all backend variables
	$(ENVTOOL) list -s config/backend.env-schema.yaml

.PHONY: list-all
list-all: list-frontend list-backend ## List all variables from all schemas

# ─── Help ───────────────────────────────────────────────────────

.PHONY: help
help: ## Show this help message
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

.DEFAULT_GOAL := help
