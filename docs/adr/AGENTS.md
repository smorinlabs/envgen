# ADRs (Architecture Decision Records)

This directory contains Architecture Decision Records (ADRs): short, durable documents that capture a single significant decision, why it was made, and the consequences.

## When to create an ADR

Create an ADR when a change is likely to matter in 3–6 months to someone new to the codebase, especially:

- New or changed public behavior (CLI/API/config defaults, file formats)
- Introducing/removing a major dependency or toolchain choice
- Architectural changes (module boundaries, data flow, persistence, threading/concurrency)
- Security, privacy, or compliance decisions
- Build/release/CI strategy changes

## Naming and numbering

- File name format: `NNNN-kebab-case-title.md`
- `NNNN` is a 4-digit, zero-padded sequence number (e.g., `0001`, `0002`, …)
- Use the next available number; never reuse or renumber existing ADRs

## Template

Use this structure (add sections only if helpful):

- Title: `# NNNN: <Decision title>`
- Date: `YYYY-MM-DD`
- Status: `Proposed` | `Accepted` | `Superseded`
- Context
- Decision
- Consequences
- Alternatives considered (optional)
- References/links (optional)

## Updating and superseding

- Prefer immutability: do not rewrite accepted ADRs except for minor clarifications or adding links.
- If a decision changes: create a new ADR that supersedes the old one and update the old ADR's status to `Superseded` with a pointer to the new ADR.

