# 0001: Add `envgen docs` command

Date: 2026-02-04

Status: Accepted

## Context

`envgen` schemas are intended to be self-documenting (variable descriptions, source instructions, notes, etc.). Today, the CLI can:

- validate schemas (`envgen check`)
- list variables (`envgen list`, table or JSON)
- generate `.env` files (`envgen pull`)

However, there is no dedicated command to render schema documentation in a shareable format (e.g., Markdown), and the default `list` output does not include per-variable documentation fields like `source_instructions` or `notes`.

## Decision

Add a new CLI command:

- `envgen docs -c <schema.yaml> [-e <env>]`

Behavior:

- Parses and validates the schema.
- Generates Markdown documentation to stdout.
- If `-e/--env` is provided, filters variables to those applicable to that environment and includes the selected environment in the header.

## Consequences

- Adds a new public CLI surface area (`envgen docs`).
- Enables generating shareable documentation without adding/maintaining separate docs files.
- Requires keeping Markdown output stable enough for humans (but does not promise a strict machine-stable schema).

## Alternatives considered

- Add a Markdown output mode to `envgen list` (would mix “inventory” and “docs” use cases).
- Add a separate “docs export” flag to `envgen check` (would overload `check` which is primarily validation-focused).

