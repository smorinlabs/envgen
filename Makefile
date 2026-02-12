# envgen — Environment File Generator CLI
# Common invocations for development and environment setup

ENVGEN := cargo run --
UV ?= uv
UVX ?= uvx
UV_CACHE_DIR ?= $(CURDIR)/.uv-cache
UV_TOOL_DIR ?= $(CURDIR)/.uv-tools
BIOME ?= npx --yes @biomejs/biome@2.3.11
CHECK_JSONSCHEMA ?= check-jsonschema
CHECK_JSONSCHEMA_VERSION ?= 0.36.1
YAMLLINT ?= yamllint
YAMLLINT_VERSION ?= 1.38.0
YAMLFMT ?= yamlfmt
YAMLFMT_VERSION ?= v0.15.0
ACTIONLINT ?= actionlint

YAML_FIXTURES := $(shell find tests/fixtures -type f \( -name '*.yaml' -o -name '*.yml' \) | LC_ALL=C sort)
CARGO_VERSION := $(shell python3 -c "import tomllib; print(tomllib.load(open('Cargo.toml','rb'))['package']['version'])")
SCHEMA_FILE := schemas/envgen.schema.v$(CARGO_VERSION).json

# ─── Build & Test ────────────────────────────────────────────────

.PHONY: build
build: ## Build the project in release mode
	cargo build --release

.PHONY: dev
dev: ## Build the project in debug mode
	cargo build

.PHONY: test
test: check-schema ## Run all tests
	cargo test

.PHONY: check
check: check-tools check-code test check-yaml-fixtures check-schema ## Run all checks (CI)

.PHONY: check-tools
check-tools: ## Verify required tooling is installed
	@command -v cargo >/dev/null 2>&1 || { echo "ERROR: cargo not found. Install Rust from https://rustup.rs/."; exit 1; }
	@cargo fmt --version >/dev/null 2>&1 || { echo "ERROR: rustfmt not found. Run: make install-rust-tools (or rustup component add rustfmt)"; exit 1; }
	@cargo clippy --version >/dev/null 2>&1 || { echo "ERROR: clippy not found. Run: make install-rust-tools (or rustup component add clippy)"; exit 1; }
	@command -v cargo-audit >/dev/null 2>&1 || { echo "ERROR: cargo-audit not found. Run: make install-cargo-tools (or cargo install cargo-audit)"; exit 1; }
	@command -v cargo-machete >/dev/null 2>&1 || { echo "ERROR: cargo-machete not found. Run: make install-cargo-tools (or cargo install cargo-machete)"; exit 1; }
	@command -v cargo-msrv >/dev/null 2>&1 || { echo "ERROR: cargo-msrv not found. Run: make install-cargo-tools (or cargo install cargo-msrv)"; exit 1; }
	@command -v typos >/dev/null 2>&1 || { echo "ERROR: typos not found. Run: make install-cargo-tools (or cargo install typos-cli)"; exit 1; }
	@command -v pre-commit >/dev/null 2>&1 || { echo "ERROR: pre-commit not found. Run: make install-pre-commit (or uv tool install pre-commit / brew install pre-commit / pipx install pre-commit)"; exit 1; }
	@command -v npx >/dev/null 2>&1 || { echo "ERROR: npx not found. Run: make install-node"; exit 1; }
	@command -v $(UVX) >/dev/null 2>&1 || { echo "ERROR: $(UVX) not found. Run: make install-uv"; exit 1; }
	@command -v $(YAMLFMT) >/dev/null 2>&1 || { echo "ERROR: $(YAMLFMT) not found. Run: make install-yaml-tools"; exit 1; }
	@command -v $(ACTIONLINT) >/dev/null 2>&1 || { echo "ERROR: $(ACTIONLINT) not found. Run: make install-actionlint"; exit 1; }

.PHONY: install-tools
install-tools: install-rust-tools install-cargo-tools install-node install-uv install-pre-commit install-yaml-tools install-actionlint ## Install all required tooling

.PHONY: install-rust-tools
install-rust-tools: ## Install Rust components (rustfmt, clippy)
	@command -v rustup >/dev/null 2>&1 || { echo "ERROR: rustup not found. Install Rust from https://rustup.rs/."; exit 1; }
	rustup component add rustfmt clippy

.PHONY: install-cargo-tools
install-cargo-tools: ## Install cargo-audit, cargo-machete, cargo-msrv, and typos
	@command -v cargo >/dev/null 2>&1 || { echo "ERROR: cargo not found. Install Rust from https://rustup.rs/."; exit 1; }
	cargo install cargo-audit cargo-machete cargo-msrv typos-cli@1.32.0

.PHONY: install-pre-commit
install-pre-commit: ## Install pre-commit (prefers uv, then pipx, then brew, then pip)
	@if command -v pre-commit >/dev/null 2>&1; then \
		echo "pre-commit already installed."; \
	elif command -v $(UV) >/dev/null 2>&1; then \
		UV_CACHE_DIR=$(UV_CACHE_DIR) UV_TOOL_DIR=$(UV_TOOL_DIR) $(UV) tool install pre-commit; \
	elif command -v pipx >/dev/null 2>&1; then \
		pipx install pre-commit; \
	elif command -v brew >/dev/null 2>&1; then \
		brew install pre-commit; \
	elif command -v pip >/dev/null 2>&1; then \
		pip install --user pre-commit; \
	else \
		echo "ERROR: pre-commit not found. Install with: uv tool install pre-commit (recommended), or brew install pre-commit, or pipx install pre-commit."; \
		exit 1; \
	fi

.PHONY: install-node
install-node: ## Ensure Node.js (npx) is available
	@command -v npx >/dev/null 2>&1 || { echo "ERROR: npx not found. Install Node.js (includes npx)."; exit 1; }

.PHONY: install-uv
install-uv: ## Install uv (provides uvx)
	@if command -v $(UVX) >/dev/null 2>&1; then \
		echo "$(UVX) already installed."; \
	elif command -v brew >/dev/null 2>&1; then \
		brew install uv; \
	elif command -v curl >/dev/null 2>&1; then \
		curl -LsSf https://astral.sh/uv/install.sh | sh; \
	else \
		echo "ERROR: uv not found. Install from https://astral.sh/uv/ or install curl."; \
		exit 1; \
	fi

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

# ─── Pre-commit ────────────────────────────────────────────────

.PHONY: pre-commit-setup
pre-commit-setup: check-tools ## Install git pre-commit hook
	pre-commit install

# ─── GitHub Actions Lint ────────────────────────────────────────

.PHONY: install-actionlint
install-actionlint: ## Install actionlint
	curl -sSL https://raw.githubusercontent.com/rhysd/actionlint/main/scripts/download-actionlint.bash | bash -s -- -b ~/.local/bin

.PHONY: lint-actions
lint-actions: ## Lint GitHub Actions workflows
	$(ACTIONLINT)

# ─── YAML Lint & Format (Fixtures) ──────────────────────────────

.PHONY: install-yamllint
install-yamllint: ## Fetch yamllint via uvx
	@command -v $(UVX) >/dev/null 2>&1 || { echo "ERROR: $(UVX) not found. Install uv to use yamllint via uvx."; exit 1; }
	UV_CACHE_DIR=$(UV_CACHE_DIR) UV_TOOL_DIR=$(UV_TOOL_DIR) $(UVX) --from $(YAMLLINT)==$(YAMLLINT_VERSION) $(YAMLLINT) --version >/dev/null

.PHONY: install-yamlfmt
install-yamlfmt: ## Install yamlfmt (Go)
	go install github.com/google/yamlfmt/cmd/yamlfmt@$(YAMLFMT_VERSION)

.PHONY: install-yaml-tools
install-yaml-tools: install-yamllint install-yamlfmt ## Install YAML lint/format tools

.PHONY: yaml-lint-fixtures
yaml-lint-fixtures: ## Lint YAML schemas in tests/fixtures
	@command -v $(UVX) >/dev/null 2>&1 || { echo "ERROR: $(UVX) not found. Install uv to use yamllint via uvx."; exit 1; }
	UV_CACHE_DIR=$(UV_CACHE_DIR) UV_TOOL_DIR=$(UV_TOOL_DIR) $(UVX) --from $(YAMLLINT)==$(YAMLLINT_VERSION) $(YAMLLINT) -c .yamllint.yml $(YAML_FIXTURES)

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

# ─── JSON Schema Validation ─────────────────────────────────────

.PHONY: check-schema check-schema-biome check-schema-meta fmt-schema

check-schema: check-schema-biome check-schema-meta ## Validate the JSON Schema file (all layers)
	@echo "✓ Schema passed all checks"

check-schema-biome: ## Layer 1: JSON lint/format check (Biome)
	@echo "Checking schema formatting and linting (Biome)... (first run may download the tool)"
	$(BIOME) check $(SCHEMA_FILE)

check-schema-meta: ## Layer 2: Meta-schema validation (Draft 2020-12)
	@echo "Validating schema against Draft 2020-12 meta-schema... (first run may download the tool)"
	@command -v $(UVX) >/dev/null 2>&1 || { echo "ERROR: $(UVX) not found. Install uv to run check-jsonschema via uvx."; exit 1; }
	UV_CACHE_DIR=$(UV_CACHE_DIR) UV_TOOL_DIR=$(UV_TOOL_DIR) $(UVX) --from $(CHECK_JSONSCHEMA)==$(CHECK_JSONSCHEMA_VERSION) $(CHECK_JSONSCHEMA) --check-metaschema $(SCHEMA_FILE)

fmt-schema: ## Auto-format the schema file (Biome)
	@echo "Formatting schema (Biome)... (first run may download the tool)"
	$(BIOME) format --write $(SCHEMA_FILE)

# ─── Commit Message ──────────────────────────────────────────────

CLAUDE ?= claude

.PHONY: commit-message
commit-message: ## Generate a conventional commit message for staged/changed files
	@if [ -z "$$(git diff --cached --name-only)" ] && [ -z "$$(git diff --name-only)" ]; then \
		echo "No staged or changed files found."; \
		exit 1; \
	fi
	@{ \
		echo "=== Staged files ==="; \
		git diff --cached --stat 2>/dev/null; \
		echo ""; \
		echo "=== Staged diff ==="; \
		git diff --cached 2>/dev/null; \
		echo ""; \
		echo "=== Unstaged changes ==="; \
		git diff --stat 2>/dev/null; \
		echo ""; \
		echo "=== Unstaged diff ==="; \
		git diff 2>/dev/null; \
	} | $(CLAUDE) -p \
		"Based on the git diff provided via stdin, write a single conventional commit message. \
		Use the Conventional Commits format: type(scope): description. \
		Valid types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert. \
		Include a scope when it is clear from the changes. \
		The subject line must be lowercase, imperative mood, no period at the end, max 72 chars. \
		If the changes are substantial, add a blank line followed by a short body (max 3 bullet points). \
		Output ONLY the commit message, nothing else — no markdown fences, no explanation."

# ─── MSRV (Minimum Supported Rust Version) ──────────────────────
# Requires: cargo install cargo-msrv

.PHONY: msrv-verify msrv-find msrv-list

msrv-verify: ## Verify the crate builds with the declared MSRV (runs tests too)
	cargo msrv verify -- cargo test

msrv-find: ## Find the actual MSRV by bisecting through Rust versions
	cargo msrv find

msrv-list: ## List all compatible Rust versions
	cargo msrv list

# ─── Help ───────────────────────────────────────────────────────

.PHONY: help
help: ## Show this help message
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

.DEFAULT_GOAL := help
