# envgen

Generate `.env` files from a declarative YAML schema.

`envgen` is a small Rust CLI for keeping environment variable definitions (docs + sources + per-environment behavior) in one place, then generating `.env` files safely and repeatably.

- **Single source of truth**: variables + documentation live in YAML
- **Source-agnostic**: resolve values via `static`, `manual`, or arbitrary shell commands
- **Multi-environment**: one schema can cover local/staging/production
- **One schema → one output file**: run multiple times to populate multiple `.env` files
- **Safe defaults**: won’t overwrite existing files without `--force`
- **Tooling-friendly**: list variables, validate schemas, export a JSON Schema for editor autocomplete

**Status:** pre-1.0 (`v0.x`). The CLI and schema may change between minor versions.

## Demo

`envgen init` writes a commented sample schema (`env.dev.yaml`). Here’s what it looks like to use:

```bash
envgen list -c env.dev.yaml
```

```text
Schema: env.dev.yaml (Sample envgen schema. Replace with your project description.)

+-----------+-----------------------+--------------+
| Name      | Environments          | Source       |
+==================================================+
| API_TOKEN | dev, local, prod, stg | manual       |
|-----------+-----------------------+--------------|
| APP_NAME  | dev, local, prod, stg | static       |
|-----------+-----------------------+--------------|
| BUILD_ID  | local, dev, stg, prod | command_name |
+-----------+-----------------------+--------------+

3 variables across 4 environments
```

Preview what would happen before writing anything:

```bash
envgen pull -c env.dev.yaml -e dev --dry-run --interactive
```

```text
Schema:      env.dev.yaml
Environment: dev
Destination: .env.dev (does not exist)

Variables to resolve:

  API_TOKEN
    source:  manual (interactive prompt)
    instructions:
      Example (GitHub fine-grained personal access token):
      1) Go to https://github.com and sign in.
      2) Click your avatar (top-right) -> Settings.
      3) Scroll to the bottom of the left sidebar -> Developer settings.
      4) Personal access tokens -> Fine-grained tokens -> Generate new token.
      5) Set an expiration date, limit repository access, and grant least-privilege permissions.
      6) Click Generate token and copy it immediately (you won't be able to see it again).
      7) Paste the token here when prompted by envgen.
      Tip: Store it in a password manager and rotate it regularly.

  APP_NAME
    source:  static
    value:   Envgen Dev

  BUILD_ID
    source:  command_name
    command: bash -lc 'echo envgen-demo-dev-BUILD_ID-$(date +%Y%m%d%H%M%S)'

3 variables would be written to .env.dev
1 command would be executed (2 static/manual)
```

## Install

### Homebrew (homebrew-core)

```bash
brew install envgen
```

### Cargo

```bash
# From crates.io (if published):
cargo install envgen --locked

# Or from git:
cargo install --git https://github.com/smorinlabs/envgen --locked
```

### Prebuilt binaries

Download the appropriate archive from [GitHub Releases](https://github.com/smorinlabs/envgen/releases) and place `envgen` on your `PATH`.

### From source

```bash
cargo build --release --locked
./target/release/envgen --help
```

## Quickstart

Create a sample schema:

```bash
envgen init
# writes ./env.dev.yaml
```

Validate it:

```bash
envgen check -c env.dev.yaml
```

Generate a `.env` file (preview first, then write):

```bash
envgen pull -c env.dev.yaml -e dev --dry-run --interactive
envgen pull -c env.dev.yaml -e dev --interactive --force
# prompts for any `source: manual` variables when `--interactive` is set
```

List variables:

```bash
envgen list -c env.dev.yaml
```

Generate Markdown docs:

```bash
envgen docs -c env.dev.yaml
```

## Command summary

- `envgen init`: write a sample schema
- `envgen check`: validate a schema file
- `envgen list`: show variables (table by default; `--format json` is available)
- `envgen docs`: generate Markdown documentation for a schema
- `envgen pull`: resolve variables and write the destination `.env` file
- `envgen schema`: export the embedded JSON Schema used for structural validation and editor autocomplete
- `envgen readme`: print the embedded README.md to stdout

Useful flags:

- `pull`: `--dry-run`, `--force`, `--destination`, `--source-timeout`, `--interactive`, `--show-secrets`, `--write-on-error`
- `pull --source-timeout <seconds>`: hard timeout per source command; timed-out commands are terminated
- `list`: `--env`, `--format`

## Schema format (YAML)

At a high level:

- `environments`: defines env names and config values usable as `{placeholders}`
- `sources`: defines named shell command templates to fetch values
- `variables`: defines each variable’s description, sensitivity, applicability, and source

Optional documentation fields:

- `sources.*`: `label`, `url`, `description`
- `variables.*.resolvers[]`: `label`, `url`, `description`

Supported schema version:

- `"2"`: one `source` per variable, plus optional per-environment `resolvers`

Placeholders available in templates:

- `{environment}`: the selected environment name
- `{key}`: the variable’s `source_key` (or the variable name)
- Any key from the active `environments.<env>` map (e.g., `{firebase_project}`)

Minimal example (schema v2):

```yaml
schema_version: "2"

metadata:
  description: "Frontend env vars"
  destination:
    local: ".env"
    production: ".env.production"

environments:
  local:
    firebase_project: "my-app-staging"
    base_url: "http://localhost:5173"
  production:
    firebase_project: "my-app"
    base_url: "https://app.example.com"

sources:
  gcloud:
    command: "gcloud secrets versions access latest --secret={key} --project={firebase_project}"
    label: "Google Cloud Secret Manager"
    url: "https://console.cloud.google.com/security/secret-manager"

variables:
  VITE_BASE_URL:
    description: "Base URL for the app"
    sensitive: false
    source: static
    values:
      local: "{base_url}"
      production: "{base_url}"

  VITE_API_KEY:
    description: "API key (prompt locally; prod comes from Secret Manager)"
    source_key: API_KEY
    source_instructions: |
      Example (GitHub fine-grained personal access token):
      1) Go to https://github.com and sign in.
      2) Click your avatar (top-right) -> Settings.
      3) Scroll to the bottom of the left sidebar -> Developer settings.
      4) Personal access tokens -> Fine-grained tokens -> Generate new token.
      5) Set an expiration date, limit repository access, and grant least-privilege permissions.
      6) Click Generate token and copy it immediately (you won't be able to see it again).
      7) Save it somewhere secure (you won't be able to view it again later).
    environments: [local, production]
    resolvers:
      - environments: [local]
        source: manual
        label: "Local OAuth client"
        url: "https://console.cloud.google.com/apis/credentials"
      - environments: [production]
        source: gcloud
        label: "Secret Manager"
```

## JSON Schema (editor validation)

Export the embedded JSON Schema (for YAML Language Server, CI, etc.). This is also the schema `envgen`
uses for **structural** validation; `envgen check` adds **semantic** validation on top. Structural
validation is implemented via the `jsonschema` crate (Draft 2020-12) on the YAML parsed into
`serde_json::Value`.

```bash
envgen schema
# writes ./envgen.schema.vX.Y.Z.json
```

Point the YAML Language Server at it (top of your schema file):

```yaml
# yaml-language-server: $schema=./envgen.schema.vX.Y.Z.json
```

## Safety notes

- **Command sources are executed via `sh -c`**. Treat schemas as code; don’t run untrusted schemas.
- `envgen pull` refuses to overwrite existing files unless you pass `--force`.
- By default, `envgen pull` does not write the destination file when write-blocking failures occur (any command-source failure, or required static/manual failure). Use `--write-on-error` to write resolved variables anyway.
- `--dry-run` masks `sensitive: true` values unless you pass `--show-secrets`.

## Development

```bash
make check
make test
make fmt
```

See `RELEASING.md` for the release process.

## License

MIT (see `LICENSE`).
 