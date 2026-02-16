# envgen — Environment File Generator CLI
# Common invocations for development and environment setup

ENVGEN := cargo run --
UV ?= uv
UVX ?= uvx
UV_CACHE_DIR ?= $(CURDIR)/.uv-cache
UV_TOOL_DIR ?= $(CURDIR)/.uv-tools
BIN_DIR ?= $(CURDIR)/.bin
BIOME ?= npx --yes @biomejs/biome@2.3.11
CHECK_JSONSCHEMA ?= check-jsonschema
CHECK_JSONSCHEMA_VERSION ?= 0.36.1
YAMLLINT ?= yamllint
YAMLLINT_VERSION ?= 1.38.0
YAMLFMT ?= $(BIN_DIR)/yamlfmt
YAMLFMT_VERSION ?= v0.15.0
ACTIONLINT ?= $(BIN_DIR)/actionlint

YAML_FIXTURES := $(shell find tests/fixtures -type f \( -name '*.yaml' -o -name '*.yml' \) | LC_ALL=C sort)
SCHEMA_ARTIFACT_VERSION := $(shell tr -d '\r\n' < SCHEMA_VERSION)
SCHEMA_FILE := schemas/envgen.schema.v$(SCHEMA_ARTIFACT_VERSION).json
VERSION_BUMP_SCRIPT := scripts/version_bump.py
HOMEBREW_TAP_SCRIPT := scripts/homebrew/tap_release.py
HOMEBREW_SOURCE_REPO ?= smorinlabs/envgen
HOMEBREW_TAP_REPO ?= smorinlabs/homebrew-tap
HOMEBREW_TAP_REPO_DIR ?= /Users/stevemorin/c/homebrew-tap
HOMEBREW_TAP_FORMULA ?= Formula/envgen.rb
HOMEBREW_SOURCE_JSON ?= $(CURDIR)/.homebrew/source-$(TAG).json
TAP_REPO_DIR ?= $(HOMEBREW_TAP_REPO_DIR)

# ─── Build & Test ────────────────────────────────────────────────

.PHONY: build
build: ## Build the project in release mode
	cargo build --release

.PHONY: dev
dev: ## Build the project in debug mode
	cargo build

.PHONY: test
test: ## Run all tests
	cargo test --locked

.PHONY: check
check: check-core check-msrv check-security ## Run all checks

.PHONY: check-tools
check-tools: check-tools-core check-tools-msrv check-tools-security ## Verify all required tooling is installed

.PHONY: check-tools-core
check-tools-core: ## Verify required tooling for core checks
	@command -v cargo >/dev/null 2>&1 || { echo "ERROR: cargo not found. Install Rust from https://rustup.rs/."; exit 1; }
	@command -v python3 >/dev/null 2>&1 || { echo "ERROR: python3 not found. Install Python 3."; exit 1; }
	@cargo fmt --version >/dev/null 2>&1 || { echo "ERROR: rustfmt not found. Run: make install-rust-tools (or rustup component add rustfmt)"; exit 1; }
	@cargo clippy --version >/dev/null 2>&1 || { echo "ERROR: clippy not found. Run: make install-rust-tools (or rustup component add clippy)"; exit 1; }
	@command -v npx >/dev/null 2>&1 || { echo "ERROR: npx not found. Run: make install-node"; exit 1; }
	@command -v $(UVX) >/dev/null 2>&1 || { echo "ERROR: $(UVX) not found. Run: make install-uv"; exit 1; }
	@[ -x "$(YAMLFMT)" ] || { echo "ERROR: $(YAMLFMT) not found/executable. Run: make install-yaml-tools"; exit 1; }
	@[ -x "$(ACTIONLINT)" ] || { echo "ERROR: $(ACTIONLINT) not found/executable. Run: make install-actionlint"; exit 1; }

.PHONY: check-tools-msrv
check-tools-msrv: ## Verify required tooling for MSRV checks
	@command -v cargo-msrv >/dev/null 2>&1 || { echo "ERROR: cargo-msrv not found. Run: make install-tools-msrv (or cargo install cargo-msrv)"; exit 1; }

.PHONY: check-tools-security
check-tools-security: ## Verify required tooling for security checks
	@command -v cargo-audit >/dev/null 2>&1 || { echo "ERROR: cargo-audit not found. Run: make install-tools-security (or cargo install cargo-audit)"; exit 1; }
	@command -v cargo-machete >/dev/null 2>&1 || { echo "ERROR: cargo-machete not found. Run: make install-tools-security (or cargo install cargo-machete)"; exit 1; }
	@command -v typos >/dev/null 2>&1 || { echo "ERROR: typos not found. Run: make install-tools-security (or cargo install typos-cli)"; exit 1; }

.PHONY: install-tools
install-tools: install-tools-core install-tools-msrv install-tools-security ## Install all required tooling

.PHONY: install-tools-core
install-tools-core: install-rust-tools install-node install-uv install-yaml-tools install-actionlint ## Install core check tooling

.PHONY: install-tools-msrv
install-tools-msrv: install-cargo-msrv ## Install MSRV check tooling

.PHONY: install-tools-security
install-tools-security: install-cargo-audit install-cargo-machete install-typos ## Install security check tooling

.PHONY: install-rust-tools
install-rust-tools: ## Install Rust components (rustfmt, clippy)
	@command -v rustup >/dev/null 2>&1 || { echo "ERROR: rustup not found. Install Rust from https://rustup.rs/."; exit 1; }
	rustup component add rustfmt clippy

.PHONY: install-cargo-audit
install-cargo-audit: ## Install cargo-audit
	@command -v cargo >/dev/null 2>&1 || { echo "ERROR: cargo not found. Install Rust from https://rustup.rs/."; exit 1; }
	cargo install cargo-audit --locked

.PHONY: install-cargo-machete
install-cargo-machete: ## Install cargo-machete
	@command -v cargo >/dev/null 2>&1 || { echo "ERROR: cargo not found. Install Rust from https://rustup.rs/."; exit 1; }
	cargo install cargo-machete --locked

.PHONY: install-cargo-msrv
install-cargo-msrv: ## Install cargo-msrv
	@command -v cargo >/dev/null 2>&1 || { echo "ERROR: cargo not found. Install Rust from https://rustup.rs/."; exit 1; }
	cargo install cargo-msrv --locked

.PHONY: install-typos
install-typos: ## Install typos-cli
	@command -v cargo >/dev/null 2>&1 || { echo "ERROR: cargo not found. Install Rust from https://rustup.rs/."; exit 1; }
	cargo install typos-cli@1.32.0 --locked

.PHONY: install-cargo-tools
install-cargo-tools: install-cargo-audit install-cargo-machete install-cargo-msrv install-typos ## Install all cargo tools

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
	cargo clippy --locked -- -D warnings -A clippy::uninlined_format_args
	cargo fmt --check

.PHONY: fmt
fmt: ## Format all code
	cargo fmt

.PHONY: check-rust
check-rust: check-code test ## Rust checks used by CI and release

.PHONY: check-package-contents
check-package-contents: ## Ensure packaged crate excludes local cache/tool directories
	@tmp_file="$$(mktemp)"; \
	trap 'rm -f "$$tmp_file"' EXIT; \
	cargo package --allow-dirty --list > "$$tmp_file"; \
	if grep -E '(^|/)\.uv-cache/|(^|/)\.uv-tools/' "$$tmp_file" >/dev/null; then \
		echo "ERROR: packaged crate includes local uv cache/tool directories" >&2; \
		grep -E '(^|/)\.uv-cache/|(^|/)\.uv-tools/' "$$tmp_file" >&2 || true; \
		exit 1; \
	fi

.PHONY: check-core
check-core: check-tools-core check-rust check-yaml-fixtures check-schema lint-actions check-package-contents ## Core checks shared by CI and release

.PHONY: check-msrv
check-msrv: check-tools-msrv msrv-verify ## MSRV checks

.PHONY: check-security
check-security: check-tools-security ## Security/dependency checks
	cargo audit
	cargo machete
	typos

.PHONY: sync-lockfile
sync-lockfile: ## Regenerate Cargo.lock using pinned Rust toolchain (1.88.0)
	cargo +1.88.0 generate-lockfile

.PHONY: check-lockfile
check-lockfile: ## Validate lockfile parity using pinned Rust toolchain (1.88.0)
	cargo +1.88.0 check --locked

.PHONY: check-release
check-release: ## Release readiness checks
	@tmp_file="$$(mktemp)"; \
	trap 'rm -f "$$tmp_file"' EXIT; \
	if ENVGEN_HINTS=0 $(MAKE) check-core > "$$tmp_file" 2>&1; then \
		cat "$$tmp_file"; \
	else \
		rc=$$?; \
		cat "$$tmp_file"; \
		if grep -E 'lock file .* needs to be updated but --locked was passed' "$$tmp_file" >/dev/null; then \
			echo "ERROR: lockfile is out of sync with Cargo.toml. Run: make sync-lockfile" >&2; \
		fi; \
		exit $$rc; \
	fi
	cargo publish --dry-run --locked --allow-dirty
	@python3 $(VERSION_BUMP_SCRIPT) status | awk -F= '/^crate_version=/{print "✓ Release readiness checks passed for crate v"$$2}'
	@python3 $(VERSION_BUMP_SCRIPT) next-step --stage crate-after-check-release

.PHONY: install
install: ## Install envgen to ~/.cargo/bin
	cargo install --path .

# ─── Pre-commit ────────────────────────────────────────────────

.PHONY: pre-commit-setup pre-commit-staged pre-commit-all precommit-fast prepush-full
pre-commit-setup: install-pre-commit ## Install git pre-commit/pre-push hooks
	pre-commit install --hook-type pre-commit --hook-type pre-push

pre-commit-staged: install-pre-commit ## Run pre-commit hooks on staged files
	pre-commit run --hook-stage pre-commit

pre-commit-all: install-pre-commit ## Run pre-commit hooks on all files
	pre-commit run --hook-stage pre-commit --all-files

precommit-fast: check-tools-core check-code lint-actions check-package-contents ## Fast local checks for commit hooks

prepush-full: check-core check-security check-msrv ## Full local checks for pre-push/manual runs

# ─── GitHub Actions Lint ────────────────────────────────────────

.PHONY: install-actionlint
install-actionlint: ## Install actionlint
	@mkdir -p $(BIN_DIR)
	curl -sSL https://raw.githubusercontent.com/rhysd/actionlint/main/scripts/download-actionlint.bash | bash -s -- latest $(BIN_DIR)

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
	@mkdir -p $(BIN_DIR)
	GOBIN=$(BIN_DIR) go install github.com/google/yamlfmt/cmd/yamlfmt@$(YAMLFMT_VERSION)

.PHONY: install-yaml-tools
install-yaml-tools: install-yamllint install-yamlfmt ## Install YAML lint/format tools

.PHONY: yaml-lint-fixtures
yaml-lint-fixtures: ## Lint YAML schemas in tests/fixtures
	@command -v $(UVX) >/dev/null 2>&1 || { echo "ERROR: $(UVX) not found. Install uv to use yamllint via uvx."; exit 1; }
	UV_CACHE_DIR=$(UV_CACHE_DIR) UV_TOOL_DIR=$(UV_TOOL_DIR) $(UVX) --from $(YAMLLINT)==$(YAMLLINT_VERSION) $(YAMLLINT) -c .yamllint.yml $(YAML_FIXTURES)

.PHONY: yaml-fmt-fixtures
yaml-fmt-fixtures: ## Format YAML schemas in tests/fixtures
	@[ -x "$(YAMLFMT)" ] || { echo "ERROR: $(YAMLFMT) not found/executable. Run: make install-yamlfmt"; exit 1; }
	$(YAMLFMT) -no_global_conf -conf .yamlfmt $(YAML_FIXTURES)

.PHONY: yaml-fmt-check-fixtures
yaml-fmt-check-fixtures: ## Check YAML formatting in tests/fixtures
	@[ -x "$(YAMLFMT)" ] || { echo "ERROR: $(YAMLFMT) not found/executable. Run: make install-yamlfmt"; exit 1; }
	$(YAMLFMT) -no_global_conf -conf .yamlfmt -lint $(YAML_FIXTURES)

.PHONY: check-yaml-fixtures
check-yaml-fixtures: yaml-lint-fixtures yaml-fmt-check-fixtures ## Lint + format-check YAML schemas in tests/fixtures

# ─── JSON Schema Validation ─────────────────────────────────────

.PHONY: check-schema check-schema-biome check-schema-meta fmt-schema

check-schema: check-schema-biome check-schema-meta ## Validate the JSON Schema file (all layers)
	@echo "✓ Schema passed all checks"
	@python3 $(VERSION_BUMP_SCRIPT) next-step --stage schema-after-check-schema

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

# ─── Versioning & Tagging ───────────────────────────────────────

.PHONY: version-status
version-status: ## Show crate/schema versions and expected schema file
	python3 $(VERSION_BUMP_SCRIPT) status

.PHONY: bump-crate
bump-crate: ## Bump crate version + CHANGELOG.md (LEVEL=patch|minor|major or VERSION=X.Y.Z)
	python3 $(VERSION_BUMP_SCRIPT) bump-crate $(if $(LEVEL),--level $(LEVEL),) $(if $(VERSION),--version $(VERSION),) $(if $(ALLOW_EMPTY_CHANGELOG),--allow-empty-changelog,) $(if $(DRY_RUN),--dry-run,)

.PHONY: bump-crate-patch
bump-crate-patch: ## Convenience crate patch bump
	$(MAKE) bump-crate LEVEL=patch $(if $(DRY_RUN),DRY_RUN=$(DRY_RUN),)

.PHONY: bump-crate-minor
bump-crate-minor: ## Convenience crate minor bump
	$(MAKE) bump-crate LEVEL=minor $(if $(DRY_RUN),DRY_RUN=$(DRY_RUN),)

.PHONY: bump-crate-major
bump-crate-major: ## Convenience crate major bump
	$(MAKE) bump-crate LEVEL=major $(if $(DRY_RUN),DRY_RUN=$(DRY_RUN),)

.PHONY: bump-schema
bump-schema: ## Bump schema version + SCHEMA_CHANGELOG.md (LEVEL=patch|minor|major or VERSION=A.B.C)
	python3 $(VERSION_BUMP_SCRIPT) bump-schema $(if $(LEVEL),--level $(LEVEL),) $(if $(VERSION),--version $(VERSION),) $(if $(ALLOW_EMPTY_SCHEMA_CHANGELOG),--allow-empty-changelog,) $(if $(DRY_RUN),--dry-run,)

.PHONY: bump-schema-patch
bump-schema-patch: ## Convenience schema patch bump
	$(MAKE) bump-schema LEVEL=patch $(if $(DRY_RUN),DRY_RUN=$(DRY_RUN),)

.PHONY: bump-schema-minor
bump-schema-minor: ## Convenience schema minor bump
	$(MAKE) bump-schema LEVEL=minor $(if $(DRY_RUN),DRY_RUN=$(DRY_RUN),)

.PHONY: bump-schema-major
bump-schema-major: ## Convenience schema major bump
	$(MAKE) bump-schema LEVEL=major $(if $(DRY_RUN),DRY_RUN=$(DRY_RUN),)

.PHONY: bump-dry-run
bump-dry-run: ## Dry-run bump (MODE=crate|schema; plus LEVEL=... or VERSION=...)
	@if [ -z "$(MODE)" ]; then \
		echo "ERROR: MODE is required (crate|schema)"; \
		exit 1; \
	fi
	@if [ "$(MODE)" = "crate" ]; then \
		python3 $(VERSION_BUMP_SCRIPT) bump-crate $(if $(LEVEL),--level $(LEVEL),) $(if $(VERSION),--version $(VERSION),) $(if $(ALLOW_EMPTY_CHANGELOG),--allow-empty-changelog,) --dry-run; \
	elif [ "$(MODE)" = "schema" ]; then \
		python3 $(VERSION_BUMP_SCRIPT) bump-schema $(if $(LEVEL),--level $(LEVEL),) $(if $(VERSION),--version $(VERSION),) $(if $(ALLOW_EMPTY_SCHEMA_CHANGELOG),--allow-empty-changelog,) --dry-run; \
	else \
		echo "ERROR: MODE must be crate or schema"; \
		exit 1; \
	fi

.PHONY: tag-crate
tag-crate: ## Create local crate tag vX.Y.Z (VERSION can override file-derived value)
	VERSION="$(VERSION)" python3 $(VERSION_BUMP_SCRIPT) tag-crate $(if $(DRY_RUN),--dry-run,)

.PHONY: push-tag-crate
push-tag-crate: ## Push crate tag vX.Y.Z to origin (VERSION can override file-derived value)
	VERSION="$(VERSION)" python3 $(VERSION_BUMP_SCRIPT) push-tag-crate $(if $(DRY_RUN),--dry-run,)

.PHONY: tag-schema
tag-schema: ## Create local schema tag schema-vA.B.C (SCHEMA_VERSION can override file-derived value)
	SCHEMA_VERSION="$(SCHEMA_VERSION)" python3 $(VERSION_BUMP_SCRIPT) tag-schema $(if $(DRY_RUN),--dry-run,)

.PHONY: push-tag-schema
push-tag-schema: ## Push schema tag schema-vA.B.C to origin (SCHEMA_VERSION can override file-derived value)
	SCHEMA_VERSION="$(SCHEMA_VERSION)" python3 $(VERSION_BUMP_SCRIPT) push-tag-schema $(if $(DRY_RUN),--dry-run,)

# ─── Homebrew Tap Release ───────────────────────────────────────

.PHONY: homebrew-status
homebrew-status: ## Show Homebrew release/tap status for TAG=vX.Y.Z
	@if [ -z "$(TAG)" ]; then \
		echo "ERROR: TAG is required (example: make homebrew-status TAG=v1.0.0)"; \
		exit 1; \
	fi
	python3 $(HOMEBREW_TAP_SCRIPT) status \
		--tag "$(TAG)" \
		--source-repo "$(HOMEBREW_SOURCE_REPO)" \
		--source-json "$(HOMEBREW_SOURCE_JSON)" \
		--tap-repo "$(HOMEBREW_TAP_REPO)" \
		--tap-repo-dir "$(HOMEBREW_TAP_REPO_DIR)" \
		--formula-path "$(HOMEBREW_TAP_FORMULA)"

.PHONY: homebrew-source
homebrew-source: ## Download and hash source tarball for TAG=vX.Y.Z
	@if [ -z "$(TAG)" ]; then \
		echo "ERROR: TAG is required (example: make homebrew-source TAG=v1.0.0)"; \
		exit 1; \
	fi
	python3 $(HOMEBREW_TAP_SCRIPT) resolve-source \
		--tag "$(TAG)" \
		--source-repo "$(HOMEBREW_SOURCE_REPO)" \
		--out-json "$(HOMEBREW_SOURCE_JSON)"

.PHONY: homebrew-sync-formula
homebrew-sync-formula: ## Sync tap formula from source metadata (TAG=vX.Y.Z TAP_REPO_DIR=/path/to/homebrew-tap)
	@if [ -z "$(TAG)" ]; then \
		echo "ERROR: TAG is required (example: make homebrew-sync-formula TAG=v1.0.0 TAP_REPO_DIR=/path/to/homebrew-tap)"; \
		exit 1; \
	fi
	@if [ -z "$(TAP_REPO_DIR)" ]; then \
		echo "ERROR: TAP_REPO_DIR is required"; \
		exit 1; \
	fi
	python3 $(HOMEBREW_TAP_SCRIPT) sync-formula \
		--tag "$(TAG)" \
		--formula-path "$(TAP_REPO_DIR)/$(HOMEBREW_TAP_FORMULA)" \
		--source-json "$(HOMEBREW_SOURCE_JSON)" \
		$(if $(DRY_RUN),--dry-run,)

.PHONY: homebrew-verify-formula
homebrew-verify-formula: ## Run brew style/audit/install/test on tap formula (TAG=vX.Y.Z TAP_REPO_DIR=/path/to/homebrew-tap)
	@if [ -z "$(TAG)" ]; then \
		echo "ERROR: TAG is required (example: make homebrew-verify-formula TAG=v1.0.0 TAP_REPO_DIR=/path/to/homebrew-tap)"; \
		exit 1; \
	fi
	@if [ -z "$(TAP_REPO_DIR)" ]; then \
		echo "ERROR: TAP_REPO_DIR is required"; \
		exit 1; \
	fi
	python3 $(HOMEBREW_TAP_SCRIPT) verify-formula \
		--tag "$(TAG)" \
		--tap-repo-dir "$(TAP_REPO_DIR)" \
		--tap-repo "$(HOMEBREW_TAP_REPO)" \
		--formula-path "$(HOMEBREW_TAP_FORMULA)"

.PHONY: homebrew-open-tap-pr
homebrew-open-tap-pr: ## Open/update tap PR for TAG=vX.Y.Z (requires GH auth; TAP_REPO_DIR=/path/to/homebrew-tap)
	@if [ -z "$(TAG)" ]; then \
		echo "ERROR: TAG is required (example: make homebrew-open-tap-pr TAG=v1.0.0 TAP_REPO_DIR=/path/to/homebrew-tap)"; \
		exit 1; \
	fi
	@if [ -z "$(TAP_REPO_DIR)" ]; then \
		echo "ERROR: TAP_REPO_DIR is required"; \
		exit 1; \
	fi
	python3 $(HOMEBREW_TAP_SCRIPT) open-pr \
		--tag "$(TAG)" \
		--tap-repo "$(HOMEBREW_TAP_REPO)" \
		--tap-repo-dir "$(TAP_REPO_DIR)" \
		--formula-path "$(HOMEBREW_TAP_FORMULA)" \
		$(if $(DRY_RUN),--dry-run,)

.PHONY: homebrew-release-tap
homebrew-release-tap: ## End-to-end tap release flow (TAG=vX.Y.Z TAP_REPO_DIR=/path/to/homebrew-tap)
	@if [ -z "$(TAG)" ]; then \
		echo "ERROR: TAG is required (example: make homebrew-release-tap TAG=v1.0.0 TAP_REPO_DIR=/path/to/homebrew-tap)"; \
		exit 1; \
	fi
	@if [ -z "$(TAP_REPO_DIR)" ]; then \
		echo "ERROR: TAP_REPO_DIR is required"; \
		exit 1; \
	fi
	$(MAKE) homebrew-source TAG="$(TAG)" HOMEBREW_SOURCE_JSON="$(HOMEBREW_SOURCE_JSON)" HOMEBREW_SOURCE_REPO="$(HOMEBREW_SOURCE_REPO)"
	$(MAKE) homebrew-sync-formula TAG="$(TAG)" TAP_REPO_DIR="$(TAP_REPO_DIR)" HOMEBREW_SOURCE_JSON="$(HOMEBREW_SOURCE_JSON)" HOMEBREW_TAP_FORMULA="$(HOMEBREW_TAP_FORMULA)" $(if $(DRY_RUN),DRY_RUN=$(DRY_RUN),)
	$(MAKE) homebrew-verify-formula TAG="$(TAG)" TAP_REPO_DIR="$(TAP_REPO_DIR)" HOMEBREW_TAP_FORMULA="$(HOMEBREW_TAP_FORMULA)"
	$(MAKE) homebrew-open-tap-pr TAG="$(TAG)" TAP_REPO_DIR="$(TAP_REPO_DIR)" HOMEBREW_TAP_REPO="$(HOMEBREW_TAP_REPO)" HOMEBREW_TAP_FORMULA="$(HOMEBREW_TAP_FORMULA)" $(if $(DRY_RUN),DRY_RUN=$(DRY_RUN),)

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
	cargo msrv verify -- cargo test --locked

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
