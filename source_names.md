# PRD: `envgen` — Source Names, URLs, and Resolver Descriptions

## 1. Overview

`envgen` schemas are meant to be self-documenting, but the current schema only captures documentation at the variable level. There is no place to attach a human-friendly **name** or **URL** for a source (e.g., a dashboard or docs page), and no way to provide **resolver-specific descriptions** when multiple resolvers point to different places.

This PRD proposes **optional documentation fields** for sources and resolvers to make schemas clearer without affecting runtime behavior.

---

## 2. Problem Statement

Today:
- `sources.*` only allows `command`, so there is no way to attach a friendly name, link, or description for the source itself.
- `variables.*.resolvers[]` has no documentation fields, so resolver-specific context must be crammed into the variable description.
- When a variable uses multiple resolvers (different systems or dashboards), there is no place to document which resolver points to which location.

---

## 3. Goals

1. **Optional documentation fields** for sources and resolvers (no required changes to existing schemas).
2. **Source-level name + URL** so a source can link to a dashboard, console, or homepage.
3. **Resolver-level name + URL + description** so resolver-specific docs can be attached when multiple resolvers exist.
4. **Schema-only change** (purely additive; no behavioral changes to `envgen pull`).
5. **Clear naming options** with a final recommendation based on existing YAML field patterns.
6. **Update example schema** update envgen.sample.yaml with an additional example with documentation at the resolver level

---

## 4. Non-Goals

- Changing how sources are executed or how values are resolved.
- Making these fields required.
- Enforcing strict URL validation beyond basic string checks.
- Adding UI/CLI output changes in this PRD (documentation-only proposal).

---

## 5. Proposed Schema Additions (Additive)

### 5.1 Top-Level `sources.*` additions

Add **optional** documentation fields to each source definition:

- `label` (string, optional): Human-friendly display name for the source.
- `url` (string, optional): Link to a dashboard, admin console, or homepage.
- `description` (string, optional): Short explanation of what the source is used for.

### 5.2 Resolver-Level `variables.*.resolvers[]` additions

Add **optional** documentation fields to each resolver entry:

- `label` (string, optional): Human-friendly label for this resolver’s source/location.
- `url` (string, optional): Link to the resolver-specific dashboard or web page.
- `description` (string, optional): Resolver-specific explanation or usage notes.

### 5.3 Precedence in Documentation (Recommended)

When docs are rendered:
- Prefer resolver-level `label`/`url`/`description` when present.
- Otherwise fall back to source-level fields (if any).
- Variable-level `description` remains the primary description of the variable itself.


### 5.4.Update envgen.sample.yaml
- envgen.sample.yaml with an additional example with documentation url and label at the resolver level
---

## 6. Field Naming Options for the URL Field

The existing YAML uses snake_case and descriptive names like `source_key`, `source_instructions`, and `notes`. Based on that, there are multiple reasonable options for the URL field name.

**Option A: `url` (Recommended)**
- Pros: Short, consistent with existing unprefixed fields like `description`.
- Cons: Slightly ambiguous outside of a source/resolver context.

**Recommendation:** Use `url` for both `sources.*` and `resolvers[]`. The object itself already scopes the meaning, and it aligns with the unprefixed style of `description`.

---

## 7. Examples

### 7.1 Recommended Field Names (`name` + `url` + `description`)

```yaml
sources:
  gcloud:
    command: "gcloud secrets versions access latest --secret={key} --project={firebase_project}"
    label: "Google Cloud Secrets Manager"
    url: "https://console.cloud.google.com/security/secret-manager"
    description: "Primary secret store for production environments."

variables:
  VITE_GOOGLE_CLIENT_ID:
    description: "Google OAuth Client ID for Sign-In and Picker"
    environments: [local, staging, production]
    resolvers:
      - environments: [local]
        source: manual
        label: "Local dev Google Cloud Console"
        url: "https://console.cloud.google.com/apis/credentials"
        description: "Create a test OAuth client for local development."
      - environments: [staging, production]
        source: gcloud
        label: "Staging/Prod Secrets"
        url: "https://console.cloud.google.com/security/secret-manager"
        description: "Read from GCP Secret Manager for non-local envs."
```

---

## 8. Compatibility

- Existing schemas remain valid because all new fields are optional.
- No behavioral changes are implied for `envgen pull`, `envgen check`, or `envgen list`.
- JSON Schema would be extended to allow these new optional fields in `sources.*` and `resolvers[]`.

---

## 9. Open Questions

1. Should URL values be validated with `format: uri`, or remain simple strings? simple string 
2. Should we allow documentation fields on built-in `static` and `manual` sources via a separate top-level map (e.g., `source_docs`), or only within resolvers?
3. Should `name` be renamed to `label` to avoid confusion with the map key? Yes
