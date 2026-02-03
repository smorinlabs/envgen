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
envgen list -s env.dev.yaml
```

```text
+-----------+-----------------------+--------------+
| Name      | Environments          | Source       |
+==================================================+
| API_TOKEN | dev, local, prod, stg | manual       |
| APP_NAME  | dev, local, prod, stg | static       |
| BUILD_ID  | local, dev, stg, prod | command_name |
+-----------+-----------------------+--------------+
```

Preview what would happen before writing anything:

```bash
envgen pull -s env.dev.yaml -e dev --dry-run
```

```text
  BUILD_ID
    source:  command_name
    command: bash -lc 'echo envgen-demo-dev-BUILD_ID-$(date +%Y%m%d%H%M%S)'
```

## Install

### Cargo

```bash
# From crates.io (if published):
cargo install envgen --locked

# Or from git:
cargo install --git https://github.com/smorinlabs/envgen --locked
```

### Prebuilt binaries

Download the appropriate archive from GitHub Releases and place `envgen` on your `PATH`.

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
envgen check -s env.dev.yaml
```

Generate a `.env` file (preview first, then write):

```bash
envgen pull -s env.dev.yaml -e dev --dry-run
envgen pull -s env.dev.yaml -e dev --force
# prompts for any `source: manual` variables
```

List variables:

```bash
envgen list -s env.dev.yaml
```

## Schema format (YAML)

At a high level:

- `environments`: defines env names and config values usable as `{placeholders}`
- `sources`: defines named shell command templates to fetch values
- `variables`: defines each variable’s description, sensitivity, applicability, and source

Supported schema versions:

- `"1"`: one `source` per variable
- `"2"`: adds per-environment `resolvers`

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

variables:
  VITE_BASE_URL:
    description: "Base URL for the app"
    sensitive: false
    source: static
    values:
      local: "{base_url}"
      production: "{base_url}"

  VITE_API_KEY:
    description: "API key (local is static; prod comes from Secret Manager)"
    source_key: API_KEY
    environments: [local, production]
    resolvers:
      - environments: [local]
        source: static
        values:
          local: "API_KEY-local"
      - environments: [production]
        source: gcloud
```

## JSON Schema (editor validation)

Export the embedded JSON Schema (for YAML Language Server, CI, etc.):

```bash
envgen schema -o schemas/
# writes ./schemas/envgen.schema.vX.Y.Z.json
```

Point the YAML Language Server at it (top of your schema file):

```yaml
# yaml-language-server: $schema=./schemas/envgen.schema.vX.Y.Z.json
```

## Safety notes

- **Command sources are executed via `sh -c`**. Treat schemas as code; don’t run untrusted schemas.
- `envgen pull` refuses to overwrite existing files unless you pass `--force`.
- `--dry-run` masks `sensitive: true` values unless you pass `--unmask`.

## Development

```bash
make check
make test
make fmt
```

See `RELEASING.md` for the release process.

## License

MIT (see `LICENSE`).
