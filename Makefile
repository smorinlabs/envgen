# envgen — Environment File Generator CLI
# Common invocations for development and environment setup

ENVGEN := cargo run --
YAMLLINT ?= yamllint
YAMLFMT ?= yamlfmt

YAML_FIXTURES := $(shell find tests/fixtures -type f \( -name '*.yaml' -o -name '*.yml' \) | LC_ALL=C sort)

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
install: ## Install envgen to ~/.cargo/bin
	cargo install --path .

# ─── YAML Lint & Format (Fixtures) ──────────────────────────────

.PHONY: install-yamllint
install-yamllint: ## Install yamllint (Python)
	python3 -m pip install --user yamllint

.PHONY: install-yamlfmt
install-yamlfmt: ## Install yamlfmt (Go)
	go install github.com/google/yamlfmt/cmd/yamlfmt@latest

.PHONY: install-yaml-tools
install-yaml-tools: install-yamllint install-yamlfmt ## Install YAML lint/format tools

.PHONY: yaml-lint-fixtures
yaml-lint-fixtures: ## Lint YAML schemas in tests/fixtures
	@command -v $(YAMLLINT) >/dev/null 2>&1 || { echo "ERROR: $(YAMLLINT) not found. Run: make install-yamllint"; exit 1; }
	$(YAMLLINT) -c .yamllint.yml $(YAML_FIXTURES)

.PHONY: yaml-fmt-fixtures
yaml-fmt-fixtures: ## Format YAML schemas in tests/fixtures
	@command -v $(YAMLFMT) >/dev/null 2>&1 || { echo "ERROR: $(YAMLFMT) not found. Run: make install-yamlfmt"; exit 1; }
	$(YAMLFMT) -no_global_conf -conf .yamlfmt $(YAML_FIXTURES)

.PHONY: yaml-fmt-check-fixtures
yaml-fmt-check-fixtures: ## Check YAML formatting in tests/fixtures
	@command -v $(YAMLFMT) >/dev/null 2>&1 || { echo "ERROR: $(YAMLFMT) not found. Run: make install-yamlfmt"; exit 1; }
	$(YAMLFMT) -no_global_conf -conf .yamlfmt -lint $(YAML_FIXTURES)

.PHONY: check-yaml-fixtures
check-yaml-fixtures: yaml-lint-fixtures yaml-fmt-check-fixtures ## Lint + format-check YAML schemas in tests/fixtures

# ─── Schema Validation ──────────────────────────────────────────

.PHONY: check-frontend
check-frontend: ## Validate the frontend schema
	$(ENVGEN) check -s config/frontend.env-schema.yaml

.PHONY: check-backend
check-backend: ## Validate the backend schema
	$(ENVGEN) check -s config/backend.env-schema.yaml

.PHONY: check-all
check-all: check-frontend check-backend ## Validate all schemas

# ─── Environment Generation ─────────────────────────────────────

.PHONY: env-local
env-local: ## Generate all local .env files
	$(ENVGEN) pull -s config/frontend.env-schema.yaml -e local --force
	$(ENVGEN) pull -s config/backend.env-schema.yaml -e local --force

.PHONY: env-staging
env-staging: ## Generate all staging .env files
	$(ENVGEN) pull -s config/frontend.env-schema.yaml -e staging --force
	$(ENVGEN) pull -s config/backend.env-schema.yaml -e staging --force

.PHONY: env-production
env-production: ## Generate all production .env files
	$(ENVGEN) pull -s config/frontend.env-schema.yaml -e production --force
	$(ENVGEN) pull -s config/backend.env-schema.yaml -e production --force

# ─── Dry Runs ───────────────────────────────────────────────────

.PHONY: dry-run-local
dry-run-local: ## Preview local .env generation
	$(ENVGEN) pull -s config/frontend.env-schema.yaml -e local --dry-run
	@echo ""
	$(ENVGEN) pull -s config/backend.env-schema.yaml -e local --dry-run

.PHONY: dry-run-staging
dry-run-staging: ## Preview staging .env generation
	$(ENVGEN) pull -s config/frontend.env-schema.yaml -e staging --dry-run
	@echo ""
	$(ENVGEN) pull -s config/backend.env-schema.yaml -e staging --dry-run

# ─── Listing ────────────────────────────────────────────────────

.PHONY: list-frontend
list-frontend: ## List all frontend variables
	$(ENVGEN) list -s config/frontend.env-schema.yaml

.PHONY: list-backend
list-backend: ## List all backend variables
	$(ENVGEN) list -s config/backend.env-schema.yaml

.PHONY: list-all
list-all: list-frontend list-backend ## List all variables from all schemas

# ─── Help ───────────────────────────────────────────────────────

.PHONY: help
help: ## Show this help message
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

.DEFAULT_GOAL := help
