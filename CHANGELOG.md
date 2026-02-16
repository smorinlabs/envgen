# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [1.0.2] - 2026-02-16

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [1.0.1] - 2026-02-16

### Added

### Changed

- Added a separate `homebrew-tap` release flow: `make homebrew-*` targets, `scripts/homebrew/tap_release.py`, and an automated tap PR workflow.

### Deprecated

### Removed

### Fixed

### Security

## [1.0.0] - 2026-02-16

### Added

### Changed

- Stabilized the project for the `1.0.0` release and updated README status to `v1.x`.

### Deprecated

### Removed

### Fixed

### Security

## [0.3.8] - 2026-02-16

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.3.6] - 2026-02-16

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.3.3] - 2026-02-16

### Added

### Changed

- Refined crates.io package metadata keywords/categories for clearer discoverability.

### Deprecated

### Removed

### Fixed

### Security

## [0.3.0] - 2026-02-13

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.2.1] - 2026-02-13

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.2.0] - 2026-02-13

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
