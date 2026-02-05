# PRD: `envgen` — YAML Schema Validation (JSON Schema + CLI)

## 1. Overview

`envgen` YAML files are themselves “schemas” (they describe `.env` outputs). To reduce typos, improve editor UX, and enable CI checks, we want a **schema validation file** (a JSON Schema document) that validates `envgen` YAML files.

This PRD covers:
- What we want to validate (structural + semantic)
- Where the validation schema lives in the repo
- How it ships inside the `envgen` binary (compile-time embedded)
- A CLI command to export the schema file with a sensible default name

---

## 2. Goals

1. **Editor validation**: Make YAML authoring safer with autocomplete + red squiggles via JSON Schema (e.g., YAML Language Server).
2. **Strictness**: Catch common issues early (missing required fields, invalid shapes, unknown keys/typos).
3. **Single source of truth**: The canonical JSON Schema used for validation ships with the `envgen` binary.
4. **Easy distribution**: Provide a command to export/publish the schema file for use in repos, CI, and tooling.

---

## 3. Non-Goals (initially)

- Validating *resolved secret values* (regex/enum validation of the final value) — this is different from validating the YAML structure.
- Proving cross-field invariants exclusively via JSON Schema (some rules are better enforced in Rust).
- Hosting/publishing the schema at a public URL (we’ll enable export; publication mechanism is project-specific).

---

## 4. Proposed Artifacts

### 4.1 JSON Schema file (repo)

- Location: `schemas/envgen.schema.v0.1.0.json`
- Format: JSON Schema (draft 2020-12)
- Scope: Validate `schema_version: "2"` schemas

### 4.2 Embedded in binary (compile time)

The schema file should be included in the binary at build time so:
- `envgen schema` works even when no repo files are present
- exported schema always matches the version of `envgen` you’re running

### 4.3 CLI export command

Add a CLI command that writes the embedded JSON Schema to a file.

**Command name recommendation:** `envgen schema`

Rationale: “schema” is intuitive and discoverable, and avoids overloading `check` (which validates a *user schema file*).

**Default output filename recommendation:** `envgen.schema.v0.1.0.json`

Rationale: widely-recognized convention (`*.schema.json`), easy to find, and supports multiple tooling integrations.

---

## 5. Validation We Want (Suggestions)

Think of validation in two layers:

### 5.1 Structural validation (JSON Schema)

Best for editor tooling and fast feedback:

- Required top-level fields exist: `schema_version`, `metadata`, `environments`, `variables`
- `schema_version` is `"2"`
- `metadata.description` is a non-empty string
- `metadata.destination` is a map of env name → path, with at least one entry
- `environments` is a map of env name → config map (strings)
- `sources` is a map of source name → `{ command: string, label?: string, url?: string, description?: string }`
  - Disallow reserved source names: `static`, `manual`
- `variables` is a map of variable name → variable definition
  - Each variable requires `description`
  - Each variable uses either:
    - a single `source`, or
    - `resolvers` (and must not also specify `source` / variable-level `values`)
  - If `source: static` then `values` must be present
  - If a resolver `source: static` then resolver `values` must be present
  - Resolver entries may include `label`, `url`, and `description` for documentation
- Reject unknown keys (where possible) to catch typos early

### 5.2 Semantic validation (Rust `envgen check`)

Rules that are hard/awkward in JSON Schema, or that need cross-field context:

- `metadata.destination` environments must exist in `environments`
- Variable `environments` entries must exist in `environments`
- Variable `source` / resolver `source` must be `static` / `manual` or exist in `sources`
- For `static`, ensure values cover all applicable environments
- For v2 `resolvers`:
  - resolvers must not overlap environment coverage
  - resolvers must fully cover all applicable environments
- Source command templates only reference resolvable placeholders for each environment

---

## 6. CLI UX Proposal

### 6.1 `envgen schema`

Exports the embedded JSON Schema to a file (defaulting to `envgen.schema.v0.1.0.json`).

Suggested flags:

- `-o, --output <path>`: output path (file or directory)
- `-o, --output -`: print schema to stdout instead of writing a file
- `-f, --force`: overwrite existing file
- `-q, --quiet`: suppress success output

### 6.2 Usage examples

```bash
# Write to default file name in CWD:
envgen schema

# Write to a specific file:
envgen schema -o schemas/envgen.schema.v0.1.0.json

# Print to stdout (for CI or redirects):
envgen schema --output - > envgen.schema.v0.1.0.json
```

---

## 7. Editor Integration (recommended)

Once published/exported into a repo, YAML Language Server can use the schema for validation/autocomplete.

Example (top of an envgen YAML file):

```yaml
# yaml-language-server: $schema=./schemas/envgen.schema.v0.1.0.json
```

---

## 8. Future Enhancements (optional)

- **Hosted `$id` / URL**: publish schema to a stable URL so repos can reference it without vendoring.
- **Versioned schemas**: `envgen.schema.v1.json`, `envgen.schema.v2.json`, or include `$defs` keyed by version.
- **Versioned schemas**: `envgen.schema.v2.json` (and future versions), or include `$defs` keyed by version.
- **Value validators**: optional `validate` blocks (regex/enum/url/etc.) checked during `pull` and/or `check`.
- **Path validation**: validate `metadata.destination.*` are relative paths (or explicitly allow absolute paths).
