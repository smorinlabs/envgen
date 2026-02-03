# PRD: `envgen` — Environment File Generator CLI

## 1. Overview

`envgen` is a standalone Rust CLI that generates `.env` files from a declarative YAML schema. Each schema file defines the variables for a single destination file (e.g., frontend `.env`, backend `functions/.env.local`). Variables can be resolved from external sources (secret managers, password vaults, arbitrary CLI commands), defined statically, or entered manually.

**Core principle:** One invocation → one schema → one output file. To populate multiple destination files, you run the tool multiple times with different schemas.

---

## 2. Problem Statement

Environment setup for this project (and projects like it) requires:

- Knowing which variables exist, which environments need them, and where to get them
- Manually running `firebase functions:secrets:access` or `op read` for each secret
- Copy-pasting values into `.env` files with no validation
- Onboarding docs that go stale because the source of truth is scattered

There is no single, machine-readable definition of "what env vars does this project need?" and no automation to populate them.

---

## 3. Goals

| Goal | Description |
|------|-------------|
| **Single source of truth** | One YAML schema per destination file defines every variable, its source, and its documentation |
| **Automated population** | `envgen pull` resolves values from configured sources and writes the `.env` file |
| **Self-documenting** | The schema file itself is the documentation — no separate docs to maintain |
| **Source-agnostic** | Works with Firebase Secret Manager, 1Password, AWS SSM, custom scripts, or any CLI command |
| **Multi-environment** | One schema defines variables across local/staging/production; select at invocation time |
| **Safe defaults** | Refuses to overwrite existing files without `--force`; dry-run shows what would happen |

---

## 4. Non-Goals (v1)

- **Push to sources** — Writing values back to secret managers is out of scope
- **Runtime secret injection** — This tool generates static files; it does not replace runtime secret managers
- **Multi-file output** — Each invocation writes one file; orchestration across files is the caller's job (e.g., a Makefile or shell script)
- **GUI or interactive wizard** — CLI only
- **Caching** — Resolved values are not cached locally; each pull fetches fresh values
- **Value validation** — No regex or enum validation on resolved values (deferred to a future version)
- **Schema inheritance** — Schemas are not composable; each schema is standalone
- **Secrets masking in output file** — The tool writes real values to the output file; masking is only applied to stdout
- **Secret rotation automation** — The schema documents *where* to rotate secrets, but the tool does not perform rotation itself
- **Cloud provider SDK integration** — No native API calls to AWS, GCP, 1Password, etc.; all external sources are invoked through shell commands
- **`.env` file parsing or import** — The tool only *writes* `.env` files; it does not read existing ones or merge with them

---

## 5. Schema Specification

### 5.1 Top-Level Structure

```yaml
# config/frontend.env-schema.yaml

schema_version: "2"

# ─── Metadata ───────────────────────────────────────────────────
metadata:
  description: "Frontend environment variables (Vite)"
  destination:
    local: ".env"
    staging: ".env"
    production: ".env.production"

# ─── Environments ───────────────────────────────────────────────
environments:
  local:
    firebase_project: "get-bank-sheets-staging"
    base_url: "http://localhost:5173"
    stripe_mode: "test"
  staging:
    firebase_project: "get-bank-sheets-staging"
    base_url: "https://appstg.getbanksheets.com"
    stripe_mode: "test"
  production:
    firebase_project: "get-bank-sheets"
    base_url: "https://app.getbanksheets.com"
    stripe_mode: "live"

# ─── Sources ────────────────────────────────────────────────────
sources:
  firebase-sm:
    command: "firebase functions:secrets:access {key} --project {firebase_project}"
  1password:
    command: "op read \"op://Engineering/BankSheets {environment}/{key}\""
  gcloud:
    command: "gcloud secrets versions access latest --secret={key} --project={firebase_project}"

# ─── Variables ──────────────────────────────────────────────────
variables:
  VITE_ENV:
    description: "Environment identifier used by frontend routing and feature flags"
    sensitive: false
    source: static
    values:
      local: "staging"
      staging: "staging"
      production: "production"

  VITE_FIREBASE_API_KEY:
    description: "Firebase Web API Key (public, safe to expose in client bundle)"
    sensitive: false
    source: manual
    source_instructions: |
      Firebase Console > Project Settings > General > Web app
      https://console.firebase.google.com/project/{firebase_project}/settings/general
    environments: [local, staging, production]

  VITE_FIREBASE_PROJECT_ID:
    description: "Firebase project ID, determines which backend this frontend talks to"
    sensitive: false
    source: static
    values:
      local: "get-bank-sheets-staging"
      staging: "get-bank-sheets-staging"
      production: "get-bank-sheets"

  VITE_GOOGLE_CLIENT_ID:
    description: "Google OAuth Client ID for Sign-In and Picker"
    sensitive: false
    source_key: GOOGLE_CLIENT_ID    # key passed into the source command template
    environments: [local, staging, production]
    resolvers:
      - environments: [local]
        source: static
        values:
          local: "local-google-client-id"
      - environments: [staging, production]
        source: gcloud

  VITE_STRIPE_PUBLIC_KEY:
    description: "Stripe publishable key (client-side, not secret)"
    sensitive: false
    source: manual
    source_instructions: |
      Stripe Dashboard > Developers > API Keys > Publishable key
      Use pk_test_* for local/staging, pk_live_* for production
    environments: [staging, production]
```

### 5.2 Field Reference

#### `schema_version` (required)
String. Version of the schema format. Allows the CLI to handle migrations.

Recognized versions:
- `"1"` — v1 format (single `source` per variable)
- `"2"` — adds per-environment variable resolvers via `variables.*.resolvers`

#### `metadata` (required)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `description` | string | yes | Human-readable purpose of this schema / the file it generates |
| `destination` | map\<env, path\> | yes | Output file path per environment. Relative to CWD. |

#### `environments` (required)
Map of environment name → key/value pairs. These values are available as **template variables** in source command templates and in static values.

Reserved keys: `environment` (the environment name itself, always available).

```yaml
environments:
  staging:
    firebase_project: "get-bank-sheets-staging"  # → {firebase_project}
    region: "us-central1"                         # → {region}
```

Any key defined here can be referenced as `{key_name}` in source commands and static values.

#### `sources` (required)
Map of source name → source definition.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `command` | string | yes | CLI command template. Available placeholders: `{key}` (the variable's source_key or variable name), `{environment}` (selected environment name), plus any key from the active environment's config. |

The special source names `static` and `manual` are built-in and must not be redefined:

- **`static`** — Value is defined inline in the variable's `values` map. No command is executed.
- **`manual`** — User provides the value interactively at the prompt (or the variable is skipped in `--non-interactive` mode).

#### `variables` (required)
Map of variable name → variable definition.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `description` | string | **yes** | — | What this variable is and why it exists. This is the documentation. |
| `sensitive` | bool | no | `true` | Whether the value is secret. Affects display in dry-run and list. |
| `source` | string | conditional | — | **Required when `resolvers` is not set.** Key into `sources`, or `static` / `manual`. |
| `source_key` | string | no | variable name | Override the `{key}` placeholder in the source command. Use when the source system names the secret differently. |
| `source_instructions` | string | no | — | Human-readable instructions for finding/creating this value. Shown during `manual` prompts and in documentation output. Template placeholders from the environment are expanded. |
| `environments` | list\<string\> | no | all environments | Which environments this variable applies to. Omit to include in all. |
| `values` | map\<env, string\> | conditional | — | **Required when `source: static`** (and `resolvers` is not set). Inline values per environment. Values may contain `{placeholder}` references to environment config. |
| `resolvers` | list\<resolver\> | conditional | — | **Schema v2 only.** Per-environment source bindings (use when a variable's source differs by environment). |
| `required` | bool | no | `true` | Whether this variable must have a value. If `false`, a missing value is a warning, not an error. |
| `notes` | string | no | — | Additional context: rotation policy, gotchas, related variables. Shown in list and docs output. |

##### `resolver` (schema_version: "2")

Each resolver binds a `source` to a specific set of environments for the variable.

Rules:
- A variable must use **either** `source` (v1-style) **or** `resolvers` (v2-style).
- Resolver `environments` must not overlap.
- Resolver environments must fully cover the variable's applicable environments.
- If a resolver uses `source: static`, it must provide `values` for its environments.

```yaml
variables:
  VITE_API_KEY:
    description: "Example: local is static, staging/prod come from Secret Manager"
    sensitive: true
    source_key: API_KEY
    resolvers:
      - environments: [local]
        source: static
        values:
          local: "API_KEY-local"
      - environments: [staging, production]
        source: gcloud
```

---

## 6. CLI Design

### 6.1 Installation

```bash
# From crates.io (future)
cargo install envgen

# Or download binary from GitHub releases
```

### 6.2 Global Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--schema <path>` | `-s` | Path to schema YAML file. **Required** for all commands. |
| `--env <name>` | `-e` | Target environment. Defaults to `local`. |
| `--help` | `-h` | Show help |
| `--version` | `-V` | Show version |

### 6.3 Commands

#### `envgen pull`

Resolve all variables for the target environment and write the destination `.env` file.

```
envgen pull --schema config/frontend.env-schema.yaml --env staging
```

| Flag | Short | Description |
|------|-------|-------------|
| `--dry-run` | `-n` | Print what would be written and which commands would run, without executing anything. Sensitive values are masked unless `--unmask` is also set. |
| `--unmask` | | Show actual sensitive values in dry-run output. |
| `--force` | `-f` | Overwrite the destination file if it already exists. Without this flag, the tool errors if the file exists. |
| `--non-interactive` | | Skip `manual` source variables instead of prompting. Emit a warning for each skipped variable. |
| `--output <path>` | `-o` | Override the destination path from the schema. |

**Behavior:**

1. Parse and validate the schema file.
2. Determine the destination file path from `metadata.destination[env]`.
3. If destination file exists and `--force` is not set → error with message.
4. For each variable applicable to the target environment:
   - Determine its effective source for that environment:
     - If `resolvers` is set (schema v2), pick the resolver whose `environments` contains the target env.
     - Otherwise, use `source` (schema v1-style).
   - `static`: Read from the appropriate `values[env]`, expand any `{placeholder}` references.
   - `manual`: Prompt for input (or skip if `--non-interactive`). Show `description` and `source_instructions`.
   - Any other source: Build the command from the source template, substituting `{key}`, `{environment}`, and environment config values. Execute it and capture stdout (trimmed).
5. If a source command fails (non-zero exit):
   - Log a warning with the variable name and stderr.
   - Continue to the next variable (the variable is omitted from output).
   - At the end, print a summary of failed variables.
   - Exit with code 1 if any required variable failed.
6. Write the output file. Include a header comment with generation metadata.
7. Print a summary: N variables written, M warnings, destination path.

**Output file format:**

```bash
# Generated by envgen v0.1.0
# Schema: config/frontend.env-schema.yaml
# Environment: staging
# Generated at: 2026-02-03T12:00:00Z
#
# DO NOT EDIT — regenerate with:
#   envgen pull -s config/frontend.env-schema.yaml -e staging --force

VITE_ENV=staging
VITE_FIREBASE_PROJECT_ID=get-bank-sheets-staging
VITE_GOOGLE_CLIENT_ID=abc123...
```

#### `envgen check`

Validate a schema file for correctness.

```
envgen check --schema config/frontend.env-schema.yaml
```

**Validates:**

- YAML syntax is valid
- `schema_version` is a recognized version
- Required top-level keys exist (`metadata`, `environments`, `sources`, `variables`)
- `metadata.destination` has at least one environment entry
- Every environment referenced in a variable's `environments` list exists in the top-level `environments` map
- Every variable's `source` references a defined source (or `static` / `manual`)
- Variables with `source: static` have a `values` map covering all their applicable environments
- Variables with `resolvers` (schema v2) cover all applicable environments exactly once (no overlaps)
- `static` resolvers include `values` for each resolver environment
- Source command templates only reference placeholders that can be resolved (from environment config + built-in keys)
- No duplicate variable names
- Every variable has a `description`

**Output on success:**
```
✓ Schema valid: 12 variables, 3 environments, 2 sources
```

**Output on failure:**
```
✗ Schema errors:
  - VITE_STRIPE_PUBLIC_KEY: source "stripe-vault" is not defined in sources
  - PLAID_SECRET: source is "static" but no values map provided
  - UNKNOWN_ENV referenced in VITE_FOO.environments but not defined in environments
```

Exit code 0 on success, 1 on failure.

#### `envgen list`

Display a table of all variables defined in the schema.

```
envgen list --schema config/backend.env-schema.yaml
```

| Flag | Description |
|------|-------------|
| `--env <name>` | Filter to variables applicable to a specific environment |
| `--format <fmt>` | Output format: `table` (default), `json` |

**Default table output:**

```
Schema: config/backend.env-schema.yaml (Backend Cloud Functions secrets)

Name                    Environment              Source
────────────────────────────────────────────────────────────────
GOOGLE_CLIENT_ID        local, staging, prod     firebase-sm
GOOGLE_CLIENT_SECRET    local, staging, prod     firebase-sm
PLAID_CLIENT_ID         local, staging, prod     firebase-sm
PLAID_SECRET_DEV        local, staging           firebase-sm
PLAID_SECRET            prod                     firebase-sm
STRIPE_SECRET_KEY       local, staging, prod     firebase-sm
STRIPE_WEBHOOK_SECRET   local, staging, prod     firebase-sm
TOKEN_ENCRYPTION_KEY    local, staging, prod     firebase-sm
OPENAI_API_KEY          local, staging, prod     firebase-sm

9 variables across 3 environments
```

**With `--env staging`:** Only rows where `staging` is in the variable's environments.

---

## 7. Schema Self-Documentation Design

The schema file **is** the documentation. Every field that serves a documentation purpose:

| Field | Documentation Role |
|-------|-------------------|
| `metadata.description` | Top-level: what this file/destination is for |
| `variables.*.description` | **Required.** What this variable is, why it exists. This is the primary documentation. |
| `variables.*.source_instructions` | How to obtain or rotate this value. Shown during manual prompts. Supports template placeholders. |
| `variables.*.notes` | Gotchas, rotation policy, cross-references to related variables. |
| `variables.*.sensitive` | Signals to readers whether this is a secret. |
| `environments.*.{key}` | Named config values document what differs per environment. |

### Recommended documentation fields per variable

A well-documented variable entry should look like:

```yaml
STRIPE_SECRET_KEY:
  description: >
    Stripe secret API key for server-side payment operations.
    Used by the Stripe SDK to create charges, manage subscriptions,
    and verify webhook signatures.
  sensitive: true
  source: firebase-sm
  source_instructions: |
    Stripe Dashboard > Developers > API Keys > Secret key
    https://dashboard.stripe.com/apikeys
    Use sk_test_* for local/staging, sk_live_* for production.
  notes: |
    Rotate if compromised. Changing this key immediately invalidates
    all in-flight API calls. Coordinate with STRIPE_WEBHOOK_SECRET.
  environments: [local, staging, production]
```

### Essential fields we recommend always including

- **`description`** — Mandatory. What is this and why do we need it?
- **`source_instructions`** — Strongly recommended for any non-static source. Where does a human go to find, create, or rotate this value?
- **`notes`** — Recommended for sensitive or critical variables. Rotation policy, dependencies on other variables, known gotchas.
- **`sensitive`** — Always set explicitly. Don't rely on the default; be clear about what's secret.
- **`required`** — Set to `false` for optional feature flags or variables only some developers need.

---

## 8. Source Command Template System

### 8.1 Template Resolution

Templates use `{placeholder}` syntax. Placeholders are resolved from three layers (in order of precedence):

1. **Variable-level overrides**: `source_key` → becomes `{key}`
2. **Environment config**: All key/value pairs from `environments.<selected_env>`
3. **Built-in values**: `{environment}` (the selected environment name), `{key}` (the variable name or `source_key`)

### 8.2 Example Resolution

Given:
```yaml
environments:
  staging:
    firebase_project: "get-bank-sheets-staging"
    op_vault: "Engineering/BankSheets Staging"

sources:
  firebase-sm:
    command: "firebase functions:secrets:access {key} --project {firebase_project}"
  1password:
    command: "op read \"op://{op_vault}/{key}\""

variables:
  GOOGLE_CLIENT_SECRET:
    source: firebase-sm
    # key = GOOGLE_CLIENT_SECRET (default, same as variable name)

  VITE_GOOGLE_CLIENT_ID:
    source: firebase-sm
    source_key: GOOGLE_CLIENT_ID   # overrides {key}
```

Running `envgen pull -e staging` resolves:

| Variable | Resolved Command |
|----------|-----------------|
| `GOOGLE_CLIENT_SECRET` | `firebase functions:secrets:access GOOGLE_CLIENT_SECRET --project get-bank-sheets-staging` |
| `VITE_GOOGLE_CLIENT_ID` | `firebase functions:secrets:access GOOGLE_CLIENT_ID --project get-bank-sheets-staging` |

### 8.3 Error on Unresolved Placeholders

If a command template contains `{foo}` and `foo` is not defined in the environment config or built-in values, the tool errors at schema validation time (`check`) and at `pull` time before executing any commands.

---

## 9. CLI Output & UX

### 9.1 Dry Run Output

```
$ envgen pull -s config/backend.env-schema.yaml -e staging --dry-run

Schema:      config/backend.env-schema.yaml
Environment: staging
Destination: functions/.env.local (does not exist)

Variables to resolve:

  GOOGLE_CLIENT_ID
    source:  firebase-sm
    command: firebase functions:secrets:access GOOGLE_CLIENT_ID --project get-bank-sheets-staging

  GOOGLE_CLIENT_SECRET
    source:  firebase-sm
    command: firebase functions:secrets:access GOOGLE_CLIENT_SECRET --project get-bank-sheets-staging

  ...

9 variables would be written to functions/.env.local
3 commands would be executed (6 static/manual)
```

### 9.2 Pull Output

```
$ envgen pull -s config/backend.env-schema.yaml -e staging --force

Pulling 9 variables for environment "staging"...

  ✓ GOOGLE_CLIENT_ID        (firebase-sm)
  ✓ GOOGLE_CLIENT_SECRET    (firebase-sm)
  ✓ PLAID_CLIENT_ID         (firebase-sm)
  ✓ PLAID_SECRET_DEV        (firebase-sm)
  ✗ LOOPS_API_KEY           (firebase-sm) — command failed: NOT_FOUND
  ✓ STRIPE_SECRET_KEY       (firebase-sm)
  ✓ STRIPE_WEBHOOK_SECRET   (firebase-sm)
  ✓ TOKEN_ENCRYPTION_KEY    (firebase-sm)
  ✓ OPENAI_API_KEY          (firebase-sm)

Wrote 8 variables to functions/.env.local
1 warning: LOOPS_API_KEY could not be resolved (required=true)

Exit code: 1
```

---

## 10. Error Handling

| Scenario | Behavior |
|----------|----------|
| Schema file not found | Error with message, exit 1 |
| Schema YAML invalid | Error with parse location, exit 1 |
| Schema validation fails | Error with all issues listed, exit 1 |
| Destination file exists (no `--force`) | Error: "Destination file already exists. Use --force to overwrite." Exit 1 |
| Source command fails | Warn, skip variable, continue. Summarize at end. Exit 1 if any required variable failed. |
| Source command times out | Default 30s timeout per command. `--timeout <seconds>` to override. Treated as failure. |
| Unresolved template placeholder | Error at validation time, before executing any commands. Exit 1. |
| Manual source in `--non-interactive` | Warn, skip. Variable omitted from output. |
| Environment not defined in schema | Error: "Environment 'foo' not found. Available: local, staging, production." Exit 1. |
| Variable has no value for target env | If `source: static` and no entry in `values` for the env → schema validation error. |

---

## 11. Project Structure (Rust)

```
envgen/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point, arg parsing (clap)
│   ├── schema/
│   │   ├── mod.rs
│   │   ├── parser.rs        # YAML deserialization (serde_yaml)
│   │   ├── validator.rs     # Schema validation logic
│   │   └── types.rs         # Schema data structures
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── pull.rs          # Pull command implementation
│   │   ├── check.rs         # Check command implementation
│   │   └── list.rs          # List command implementation
│   ├── resolver/
│   │   ├── mod.rs
│   │   ├── static_source.rs # Static value resolution
│   │   ├── manual_source.rs # Interactive prompt
│   │   └── command_source.rs# CLI command execution & template expansion
│   ├── template.rs          # {placeholder} expansion engine
│   └── output.rs            # .env file writer, table formatter
├── tests/
│   ├── fixtures/            # Sample schema files for testing
│   ├── test_pull.rs
│   ├── test_check.rs
│   ├── test_list.rs
│   └── test_template.rs
```

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing with derive macros |
| `serde` + `serde_yaml` | Schema deserialization |
| `tokio` | Async command execution with timeout |
| `comfy-table` | Table output formatting |
| `dialoguer` | Interactive prompts for manual source |
| `colored` | Terminal coloring for status output |

---

## 12. Example: Full Backend Schema

```yaml
schema_version: "2"

metadata:
  description: >
    Backend Cloud Functions secrets and configuration.
    These variables are loaded at runtime via Firebase Secret Manager
    in deployed environments, and via this generated .env file for
    local emulator development.
  destination:
    local: "functions/.env.local"
    staging: "functions/.env.local"
    production: "functions/.env.local"

environments:
  local:
    firebase_project: "get-bank-sheets-staging"
    plaid_env: "sandbox"
  staging:
    firebase_project: "get-bank-sheets-staging"
    plaid_env: "sandbox"
  production:
    firebase_project: "get-bank-sheets"
    plaid_env: "production"

sources:
  firebase-sm:
    command: "firebase functions:secrets:access {key} --project {firebase_project}"
  1password:
    command: "op read \"op://Engineering/BankSheets {environment}/{key}\""

variables:
  GOOGLE_CLIENT_ID:
    description: >
      Google OAuth 2.0 Client ID. Shared between frontend and backend.
      Used to verify Google Sign-In tokens and initiate OAuth flows
      for Google Sheets access.
    sensitive: false
    source: firebase-sm
    source_instructions: |
      GCP Console > APIs & Services > Credentials > OAuth 2.0 Client IDs
      https://console.cloud.google.com/apis/credentials?project={firebase_project}

  GOOGLE_CLIENT_SECRET:
    description: >
      Google OAuth 2.0 Client Secret. Used server-side to exchange
      authorization codes for access/refresh tokens.
    sensitive: true
    source: firebase-sm
    source_instructions: |
      GCP Console > APIs & Services > Credentials > OAuth 2.0 Client > Client secret
      https://console.cloud.google.com/apis/credentials?project={firebase_project}
    notes: "Rotate annually. Update in all environments simultaneously."

  PLAID_CLIENT_ID:
    description: >
      Plaid API Client ID. Identifies the application to Plaid's API
      for bank account linking via Plaid Link.
    sensitive: true
    source: firebase-sm
    source_instructions: |
      Plaid Dashboard > Team Settings > Keys
      https://dashboard.plaid.com/team/keys

  PLAID_SECRET_DEV:
    description: >
      Plaid sandbox/development secret. Used for testing bank connections
      without real credentials. Only applicable in non-production environments.
    sensitive: true
    source: firebase-sm
    source_instructions: "Plaid Dashboard > Team Settings > Keys > Sandbox secret"
    environments: [local, staging]

  PLAID_SECRET:
    description: >
      Plaid production secret. Used for live bank connections.
      Only deployed to the production environment.
    sensitive: true
    source: firebase-sm
    source_instructions: "Plaid Dashboard > Team Settings > Keys > Production secret"
    environments: [production]

  STRIPE_SECRET_KEY:
    description: >
      Stripe secret API key. Used for creating charges, managing
      subscriptions, and all server-side Stripe operations.
    sensitive: true
    source: firebase-sm
    source_instructions: |
      Stripe Dashboard > Developers > API Keys > Secret key
      https://dashboard.stripe.com/apikeys
      Use sk_test_* for local/staging, sk_live_* for production.
    notes: "Coordinate rotation with STRIPE_WEBHOOK_SECRET."

  STRIPE_WEBHOOK_SECRET:
    description: >
      Stripe webhook signing secret. Used to verify that incoming
      webhook events are genuinely from Stripe, not forged.
      Each environment has its own webhook endpoint and secret.
    sensitive: true
    source: firebase-sm
    source_instructions: |
      Stripe Dashboard > Developers > Webhooks > [endpoint] > Signing secret
      https://dashboard.stripe.com/webhooks

  TOKEN_ENCRYPTION_KEY:
    description: >
      AES encryption key for stored OAuth refresh tokens in Firestore.
      Tokens are encrypted at rest to limit blast radius if the
      database is compromised.
    sensitive: true
    source: firebase-sm
    source_instructions: "Generate with: openssl rand -base64 32"
    notes: |
      CRITICAL: Changing this key invalidates ALL stored user tokens.
      Users would need to re-authorize Google Sheets access.
      Coordinate carefully if rotation is needed.

  OPENAI_API_KEY:
    description: >
      OpenAI API key for AI-powered transaction categorization.
      Used by the categorization Cloud Function.
    sensitive: true
    resolvers:
      - environments: [local]
        source: 1password
      - environments: [staging, production]
        source: firebase-sm
    source_instructions: |
      OpenAI Platform > API Keys
      https://platform.openai.com/api-keys

  LOOPS_API_KEY:
    description: "Loops.so API key for transactional and marketing email delivery."
    sensitive: true
    source: firebase-sm
    source_instructions: "Loops Dashboard > Settings > API"
    required: false
    notes: "Optional — email features degrade gracefully without this."

  POSTHOG_API_KEY:
    description: "PostHog project API key for server-side analytics events."
    sensitive: true
    source: firebase-sm
    source_instructions: "PostHog > Project Settings > API Key"
    required: false
```

---

## 13. Usage Examples

```bash
# Validate the schema
envgen check -s config/frontend.env-schema.yaml

# Preview what would be generated for local dev
envgen pull -s config/frontend.env-schema.yaml --dry-run

# Generate local frontend .env
envgen pull -s config/frontend.env-schema.yaml

# Generate staging backend env (explicit env, force overwrite)
envgen pull -s config/backend.env-schema.yaml -e staging --force

# List all backend variables
envgen list -s config/backend.env-schema.yaml

# List only production variables as JSON
envgen list -s config/backend.env-schema.yaml -e production --format json

# Non-interactive pull (CI/CD — skip manual prompts)
envgen pull -s config/frontend.env-schema.yaml -e staging --non-interactive

# Override output path
envgen pull -s config/backend.env-schema.yaml -o /tmp/test.env --force
```

---

## 14. Decisions & Open Questions

### Decided

| Question | Decision |
|----------|----------|
| **Parallel command execution** | Parallel by default. Source commands run concurrently via tokio for faster resolution. |
| **Caching** | Not needed for v1. Each pull fetches fresh values. |
| **Value validation** | Not needed for v1. No regex/enum validation on resolved values. Deferred to a future version. |
| **Secrets masking in output file** | Real values are written to the output file. Masking is applied only to stdout during dry-run (unless `--unmask` is set). |
| **Schema inheritance** | One schema version field for format versioning, but no frontend/backend inheritance or composition. Each schema is standalone. |
| **Wrapper scripts** | A Makefile is included with common invocations (e.g., `make env-local` runs pull for both frontend and backend schemas). |

### Open for future versions

1. **Value validation** — Should variables support a `validate` field (regex or enum) to check resolved values? E.g., `validate: "^pk_(test|live)_"` for Stripe keys.
2. **Schema composition** — Should schemas support an `extends` field to inherit from a base schema, reducing duplication across frontend/backend?
3. **`--parallel` flag** — Should there be an opt-out `--sequential` flag for environments with rate-limited secret managers?
4. **Push to sources** — Should a future `envgen push` command write values back to secret managers?
5. **Local caching** — Should resolved values be cached in `.envgen-cache` to speed up repeated pulls during development?
6. **CI/CD pipeline integration** — Built-in GitHub Actions, Cloud Build plugins, etc. Currently the binary is meant to be called directly.
7. **Variable dependency ordering** — Support for one variable referencing another variable's resolved value (e.g., `DATABASE_URL` composed from `DB_HOST` + `DB_PORT`).
