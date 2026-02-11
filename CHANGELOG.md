# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `envgen readme` subcommand (prints the embedded `README.md` to stdout)

### Changed

- `envgen pull` now uses transactional write gating: by default it does not write the destination file when write-blocking failures occur (any command-source failure, or required non-command failure).
- Added `envgen pull --write-on-error` to allow writing resolved variables even when write-blocking failures occurred (exit code remains non-zero).

### Deprecated

### Removed

- Support for `schema_version: "1"` (schemas must use `"2"`)

### Fixed

- Static `values` templates now expand `{key}` to the variable’s effective `source_key` (consistent with command/manual sources).

### Security
