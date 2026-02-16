#!/usr/bin/env python3
"""Validate release tag against Cargo.toml version and emit the version."""

from __future__ import annotations

import argparse
import pathlib
import re
import sys

try:
    import tomllib
except ModuleNotFoundError:
    print("ERROR: Python tomllib not available", file=sys.stderr)
    raise SystemExit(1)


TAG_PATTERN = re.compile(r"^v\d+\.\d+\.\d+.*$")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate a vX.Y.Z-style tag matches Cargo.toml [package].version."
    )
    parser.add_argument("--tag", required=True, help="Tag value (for example: v0.1.0)")
    parser.add_argument(
        "--cargo-toml",
        default="Cargo.toml",
        help="Path to Cargo.toml (default: Cargo.toml)",
    )
    return parser.parse_args()


def read_package_version(cargo_toml_path: pathlib.Path) -> str:
    if not cargo_toml_path.exists():
        print(f"ERROR: Cargo manifest not found: {cargo_toml_path}", file=sys.stderr)
        raise SystemExit(1)

    with cargo_toml_path.open("rb") as manifest_file:
        manifest = tomllib.load(manifest_file)

    version = manifest.get("package", {}).get("version")
    if not version:
        print(
            "ERROR: Could not find [package].version in Cargo.toml",
            file=sys.stderr,
        )
        raise SystemExit(1)
    return version


def main() -> int:
    args = parse_args()
    tag = args.tag.strip()

    if not TAG_PATTERN.match(tag):
        print(f"ERROR: tag '{tag}' does not look like vX.Y.Z", file=sys.stderr)
        return 1

    version = read_package_version(pathlib.Path(args.cargo_toml))
    expected = f"v{version}"
    if tag != expected:
        print(
            "ERROR: tag "
            f"'{tag}' does not match Cargo.toml version '{version}' "
            f"(expected '{expected}')",
            file=sys.stderr,
        )
        return 1

    print(f"OK: {tag} matches Cargo.toml version {version}", file=sys.stderr)
    print(version)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
