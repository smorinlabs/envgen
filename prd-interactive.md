# PRD: `envgen` — Interactive Schema Editor (TUI)

> This PRD proposes a **vNext** feature. `prd.md` lists “GUI or interactive wizard” as a v1 non-goal.

## 1. Overview

`envgen` schemas are powerful, but authoring YAML by hand can be slow and error-prone. Interactive mode adds a **simple TUI** (keyboard-driven menus + prompts) to help create and extend **schema v2** files while enforcing the repo’s schema rules and preserving existing YAML comments.

This PRD covers **authoring the schema YAML** (the “source of truth”), not generating `.env` files (that remains `envgen pull`).

## 2. Goals

1. **Schema v2 only**: Interactive mode only operates on `schema_version: "2"` schemas (and `init --interactive` always generates `"2"`).
2. **Add-only UX**: Users can add new entries; editing/deleting existing entries is out of scope.
3. **Comment-preserving writes**: Updating an existing schema must preserve existing YAML comments (formatting may change only in inserted sections).
4. **Guardrails**: Enforce required fields and key naming rules per the embedded JSON Schema (`schemas/envgen.schema.v0.1.0.json`) and semantic rules enforced by `envgen check`.
5. **Two-step confirmation**:
   - Confirm each new entry after a YAML snippet preview.
   - Confirm once more before writing the file.
6. **Safe output controls**:
   - Default: write in place.
   - `--dry-run`: don’t write.
   - `--print`: print the full updated YAML; don’t write.

## 3. Non-Goals (vNext MVP)

- Editing or deleting existing environments/sources/variables/env keys.
- Supporting schema versions other than `"2"`.
- Supporting schema versions beyond `"2"` until the codebase and JSON Schema recognize them.
- Reformatting/canonicalizing the entire file (this would break comment preservation).
- Validating resolved secret *values* (regex/enum checks on final `.env` contents).
- Interactively running `pull` (this PRD is schema authoring only).

## 4. Terminology

- **Schema file**: The YAML file consumed by `envgen` (e.g., `config/frontend.env-schema.yaml`).
- **Environment**: A top-level entry under `environments:<env_name>`.
- **Destination mapping**: A top-level entry under `metadata.destination:<env_name>` used by `pull` to choose output path.
- **Environment config key**: A key/value pair under `environments.<env_name>` used as a `{placeholder}` in templates.
- **Source**: A top-level entry under `sources:<source_name>` containing `command: "...{key}..."`.
- **Variable**: A top-level entry under `variables:<VAR_NAME>`.

## 5. Constraints (Must Match Current Code + JSON Schema)

### 5.1 Version constraint

- Interactive mode requires `schema_version: "2"`. If the file is not `"2"`, error with guidance (“Interactive mode supports v2 only.”).

### 5.2 Naming constraints

These must match the embedded JSON Schema and template placeholder constraints:

- **Variable names**: `^[A-Za-z_][A-Za-z0-9_]*$`
- **Environment config keys**: `^[A-Za-z_][A-Za-z0-9_]*$`
- **Source names**: `^[A-Za-z0-9][A-Za-z0-9_-]*$` and must not be `static` or `manual`
- **Reserved environment config keys**: `environment` and `key` are reserved placeholders and must not be added as config keys.

### 5.3 Variable shape constraints (schema v2)

Interactive must generate one of these valid shapes:

1) **Single source** (`variables.*.source`):
- `description` is required
- `source` is required
- if `source: static`, then `values` is required

2) **Resolvers** (`variables.*.resolvers`):
- `description` is required
- `resolvers` is required and non-empty
- `source` and variable-level `values` must not be set
- resolver environment coverage must be:
  - no overlaps
  - full coverage of applicable environments
  - if resolver `source: static`, `values` is required

### 5.4 Destination pairing constraint (behavioral)

Even though `envgen check` does not require every environment to have a destination mapping, `envgen pull -e <env>` **fails** if `metadata.destination[env]` is missing. Therefore:

- When interactive mode adds a new environment, it must also prompt for and add `metadata.destination.<env_name>`.

## 6. Proposed CLI

### 6.1 `envgen init --interactive`

Creates a new schema v2 file via a guided TUI.

Proposed flags (existing flags retained):

- `-o, --output <path>`: output path (file or directory)
- `-f, --force`: overwrite output file if it exists
- `-q, --quiet`: suppress success output
- `-i, --interactive`: enable TUI flow (new)

Notes:
- Default output name should remain consistent with the existing `init` behavior (`env.dev.yaml`).
- The generated file must satisfy the JSON Schema requirement that `environments` and `variables` are non-empty.

### 6.2 `envgen add --interactive`

Adds new entries to an existing schema v2 file.

Proposed flags:

- `-c, --config <path>`: path to schema YAML file (required; existing convention)
- `-i, --interactive`: enable TUI flow (new)
- `-n, --dry-run`: do not write changes
- `--print`: print the full updated YAML to stdout; do not write changes

Behavior notes:
- Hard error if any new key would collide with an existing key.
- Preserve existing comments by inserting new YAML instead of re-serializing the entire file.

## 7. TUI UX (High-Level)

### 7.1 Session model

An interactive session stages 1..N entries, then writes once (unless `--dry-run` / `--print`).

For each entry:
1. Collect required fields (and optional fields when applicable).
2. Show a **YAML snippet** preview of exactly what will be inserted.
3. Confirm “Add this entry?” (Yes/No).

At end of session:
1. Show a summary (N staged changes; target file).
2. Confirm “Write changes to disk?” (Yes/No).
3. If writing: apply comment-preserving patches.
4. Run schema validation (same rules as `envgen check`). If validation fails, do not write (or roll back), and show errors.

### 7.2 Entry type menu (vNext MVP)

Interactive `add` must offer:

1. Add **environment** (+ destination mapping)
2. Add **environment config key**
3. Add **source**
4. Add **variable**

## 8. Detailed Flows

### 8.1 Add Environment (+ destination)

Required prompts:
- `env_name` (must not already exist)
- `metadata.destination.<env_name>` destination path (non-empty string)

Optional prompts:
- “Add initial environment config keys now?” (loop through Add Environment Config Key flow scoped to this env)

Snippet preview example:

```yaml
environments:
  stg:
    app_slug: "my-app"

metadata:
  destination:
    stg: ".env.stg"
```

Collision rules:
- Error if `environments.<env_name>` exists.
- Error if `metadata.destination.<env_name>` exists.

### 8.2 Add Environment Config Key

Required prompts:
- `key_name` (pattern-constrained; must not be `environment` or `key`)
- Select environments to apply (multi-select)
- Provide value(s):
  - Option A: “Same value for all selected envs”
  - Option B: “Enter per-environment values”

Collision rules:
- Error if the chosen key already exists in any selected environment.

Snippet preview example (multi-env):

```yaml
environments:
  local:
    firebase_project: "my-proj-dev"
  production:
    firebase_project: "my-proj-prod"
```

### 8.3 Add Source

Required prompts:
- `source_name` (pattern-constrained; must not already exist; must not be `static` or `manual`)
- `command` (non-empty string)

Optional prompts:
- Show help text listing known placeholders:
  - built-ins: `{key}`, `{environment}`
  - plus any env config keys currently present

Snippet preview example:

```yaml
sources:
  firebase-sm:
    command: "firebase functions:secrets:access {key} --project {firebase_project}"
```

Collision rules:
- Error if `sources.<source_name>` exists.

### 8.4 Add Variable (schema v2)

Required prompts:
- `var_name` (pattern-constrained; must not already exist)
- `description` (non-empty)
- `sensitive` (default: true)
- `required` (default: true)
- Applicable environments:
  - Option A: “All environments” (omit `environments:` field)
  - Option B: “Select environments” (write `environments: [...]`)

Then choose one of:

#### 8.4.1 Variable with single source

Prompts:
- `source` (select from: `static`, `manual`, plus entries in `sources`)
- If `static`: prompt for `values` (same for all / per env), covering all applicable envs
- Optional: `source_key`
- Optional: `source_instructions`
- Optional: `notes`

Snippet preview example (static):

```yaml
variables:
  APP_NAME:
    description: "App display name"
    sensitive: false
    source: static
    values:
      local: "My App (Local)"
      production: "My App"
```

#### 8.4.2 Variable with resolvers

Prompts:
- For each applicable environment, select a source (static/manual/custom source).
- Optionally group environments that share a source into a single resolver (or auto-group in the UI).
- For any resolver where `source: static`, prompt for `values` for each env in that resolver.
- Optional: variable-level `source_key` (and optionally per-resolver `source_key` overrides in an “Advanced” toggle)
- Optional: `source_instructions`
- Optional: `notes`

Validation:
- No overlaps; full coverage of applicable envs.

Snippet preview example:

```yaml
variables:
  OPENAI_API_KEY:
    description: "OpenAI API key"
    sensitive: true
    resolvers:
      - environments: [local]
        source: 1password
      - environments: [production]
        source: firebase-sm
```

Collision rules:
- Error if `variables.<var_name>` exists.

## 9. Validation & Errors

### 9.1 Pre-flight

Before starting an interactive add session:
- File must exist and be valid YAML.
- Parsed schema must be `schema_version: "2"`.

### 9.2 Per-entry validation

Before allowing an entry to be staged:
- Validate naming constraints and required fields.
- Validate v2-specific resolver rules when applicable.
- Hard error on collisions (no “replace”).

### 9.3 Final validation

Before writing (or before printing):
- Re-parse the full updated YAML and run the same semantic validation as `envgen check`.
- If validation fails, do not write; display all errors.

## 10. Comment-Preserving Write Strategy

Requirement: keep existing comments in the schema file.

Guiding approach:
- Treat the existing YAML file as the source of truth for comments.
- Apply **minimal text insertions** into the appropriate top-level mapping (`metadata.destination`, `environments`, `sources`, `variables`).
- Do not reorder or rewrite existing keys.
- Insert new entries at the end of the relevant mapping to minimize disruption.

Open engineering detail (implementation choice):
- Use a YAML editing approach that can preserve comments (token-aware), or implement a conservative text patcher that inserts new blocks based on scanning indentation and top-level section boundaries.

## 11. Examples (CLI)

```bash
# Create a new v2 schema with a TUI
envgen init --interactive -o config/frontend.env-schema.yaml

# Add entries to an existing schema v2 file
envgen add --interactive -c config/frontend.env-schema.yaml

# Dry run: no write
envgen add -i -c config/frontend.env-schema.yaml --dry-run

# Print full updated YAML; no write
envgen add -i -c config/frontend.env-schema.yaml --print
```

## 12. Open Questions

1. Should `envgen add -i` require the input schema to already pass `envgen check`, or only require parseable v2 YAML (and enforce validation only at final write)?
2. Should `--print` also write by default, or remain “print-only” (this PRD proposes print-only)?
3. Should resolver authoring default to “pick a source per env” (simplest) and auto-group, or ask users to define groups explicitly?
4. What is the best-in-class Rust dependency/approach for comment-preserving YAML edits (vs. minimal insertion patching)?
