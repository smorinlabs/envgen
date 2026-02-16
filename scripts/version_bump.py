#!/usr/bin/env python3
"""Release version automation for crate + schema streams.

This script intentionally separates bumping files from creating/pushing tags.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import shlex
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CARGO_TOML = ROOT / "Cargo.toml"
CHANGELOG = ROOT / "CHANGELOG.md"
SCHEMA_CHANGELOG = ROOT / "SCHEMA_CHANGELOG.md"
SCHEMA_VERSION_FILE = ROOT / "SCHEMA_VERSION"
SCHEMA_DIR = ROOT / "schemas"

SEMVER_RE = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$")
UNRELEASED_RE = re.compile(r"(?ms)^## \[Unreleased\]\n(?P<body>.*?)(?=^## |\Z)")

CRATE_SECTIONS = ["Added", "Changed", "Deprecated", "Removed", "Fixed", "Security"]
SCHEMA_SECTIONS = [
    "Added",
    "Changed",
    "Deprecated",
    "Removed",
    "Fixed",
    "Compatibility",
]

TRUTHY_ENV_VALUES = {"1", "true", "yes", "on"}
FALSY_ENV_VALUES = {"", "0", "false", "no", "off"}

NEXT_STEP_STAGES = (
    "crate-after-bump",
    "crate-after-check-release",
    "crate-after-tag",
    "crate-after-push-tag",
    "schema-after-bump",
    "schema-after-check-schema",
    "schema-after-tag",
    "schema-after-push-tag",
)

RELEASE_WORKFLOW_URL = (
    "https://github.com/smorinlabs/envgen/actions/workflows/release.yml"
)
LOCKFILE_SYNC_ARGS = ["cargo", "+1.88.0", "generate-lockfile"]


class BumpError(RuntimeError):
    """Raised when the bump flow should fail with a user-facing error."""


def fail(message: str) -> None:
    raise BumpError(message)


def env_var_truthy(name: str) -> bool:
    value = os.environ.get(name)
    if value is None:
        return False
    return value.strip().lower() not in FALSY_ENV_VALUES


def parse_hint_override() -> bool | None:
    value = os.environ.get("ENVGEN_HINTS")
    if value is None:
        return None

    normalized = value.strip().lower()
    if normalized in TRUTHY_ENV_VALUES:
        return True
    if normalized in FALSY_ENV_VALUES:
        return False
    return None


def hints_enabled() -> bool:
    override = parse_hint_override()
    if override is not None:
        return override
    if env_var_truthy("CI"):
        return False
    return sys.stdout.isatty()


def render_next_step(
    stage: str,
    *,
    crate_version: str | None = None,
    schema_version: str | None = None,
    tag_name: str | None = None,
    lockfile_synced: bool | None = None,
) -> tuple[str, list[str]]:
    if stage == "crate-after-bump":
        resolved_crate_version = crate_version or read_cargo_version()
        lines: list[str] = []
        if lockfile_synced is True:
            lines.append("Cargo.lock synchronized for locked checks.")
        elif lockfile_synced is False:
            lines.append("Cargo.lock sync runs only in non-dry-run bump mode.")
        lines.append("$ make check-release")
        return (
            f"Crate release prep updated to v{resolved_crate_version}.",
            lines,
        )

    if stage == "crate-after-check-release":
        resolved_crate_version = crate_version or read_cargo_version()
        return (
            f"Release readiness checks passed for crate v{resolved_crate_version}.",
            [
                "$ git add Cargo.toml Cargo.lock CHANGELOG.md",
                f'$ git commit -m "chore(release): bump crate to v{resolved_crate_version}"',
                "$ git push origin main",
                "$ make tag-crate",
            ],
        )

    if stage == "crate-after-tag":
        resolved_crate_version = crate_version or read_cargo_version()
        resolved_tag_name = tag_name or f"v{resolved_crate_version}"
        return (
            f"Local crate tag created: {resolved_tag_name}.",
            ["$ make push-tag-crate"],
        )

    if stage == "crate-after-push-tag":
        resolved_crate_version = crate_version or read_cargo_version()
        resolved_tag_name = tag_name or f"v{resolved_crate_version}"
        return (
            f"Crate tag pushed to origin: {resolved_tag_name}.",
            [
                "Release workflow should trigger automatically from this tag push.",
                f"Monitor: {RELEASE_WORKFLOW_URL}",
            ],
        )

    if stage == "schema-after-bump":
        resolved_schema_version = schema_version or read_schema_version_file()
        return (
            f"Schema release prep updated to v{resolved_schema_version}.",
            ["$ make check-schema"],
        )

    if stage == "schema-after-check-schema":
        resolved_schema_version = schema_version or read_schema_version_file()
        schema_file = f"schemas/envgen.schema.v{resolved_schema_version}.json"
        return (
            f"Schema checks passed for artifact v{resolved_schema_version}.",
            [
                f"$ git add SCHEMA_VERSION SCHEMA_CHANGELOG.md {schema_file}",
                f'$ git commit -m "chore(schema): schema-v{resolved_schema_version}"',
                "$ git push origin main",
                "$ make tag-schema",
            ],
        )

    if stage == "schema-after-tag":
        resolved_schema_version = schema_version or read_schema_version_file()
        resolved_tag_name = tag_name or f"schema-v{resolved_schema_version}"
        return (
            f"Local schema tag created: {resolved_tag_name}.",
            ["$ make push-tag-schema"],
        )

    if stage == "schema-after-push-tag":
        resolved_schema_version = schema_version or read_schema_version_file()
        resolved_tag_name = tag_name or f"schema-v{resolved_schema_version}"
        return (
            f"Schema tag pushed to origin: {resolved_tag_name}.",
            [
                "Schema tag pushes do not trigger crates.io publishing.",
                "Create and push a crate tag (vX.Y.Z) when you want a crate release.",
            ],
        )

    fail(f"Unsupported next-step stage: {stage}")


def emit_next_step(
    stage: str,
    *,
    crate_version: str | None = None,
    schema_version: str | None = None,
    tag_name: str | None = None,
    lockfile_synced: bool | None = None,
) -> None:
    if not hints_enabled():
        return

    summary, lines = render_next_step(
        stage,
        crate_version=crate_version,
        schema_version=schema_version,
        tag_name=tag_name,
        lockfile_synced=lockfile_synced,
    )
    print("")
    print(f"Hint: {summary}")
    print("Next:")
    for line in lines:
        print(f"  {line}")


def write_atomic(path: Path, content: str) -> None:
    tmp_path = path.with_suffix(path.suffix + ".tmp")
    tmp_path.write_text(content, encoding="utf-8")
    tmp_path.replace(path)


def validate_semver(version: str) -> str:
    if not SEMVER_RE.fullmatch(version):
        fail(f"Invalid version '{version}'. Expected strict semver X.Y.Z")
    return version


def bump_semver(version: str, level: str) -> str:
    major_s, minor_s, patch_s = validate_semver(version).split(".")
    major = int(major_s)
    minor = int(minor_s)
    patch = int(patch_s)

    if level == "patch":
        patch += 1
    elif level == "minor":
        minor += 1
        patch = 0
    elif level == "major":
        major += 1
        minor = 0
        patch = 0
    else:
        fail(f"Unsupported level '{level}'. Use patch|minor|major")

    return f"{major}.{minor}.{patch}"


def resolve_next_version(current: str, level: str | None, version: str | None) -> str:
    if bool(level) == bool(version):
        fail("Provide exactly one of --level or --version")
    if version:
        return validate_semver(version)
    assert level is not None
    return bump_semver(current, level)


def read_cargo_version() -> str:
    with CARGO_TOML.open("rb") as fh:
        parsed = tomllib.load(fh)
    version = parsed.get("package", {}).get("version")
    if not version:
        fail("Could not read [package].version from Cargo.toml")
    return validate_semver(version)


def update_cargo_version(new_version: str, dry_run: bool) -> tuple[str, str]:
    old_version = read_cargo_version()
    if old_version == new_version:
        fail("New crate version matches current version; nothing to do")

    text = CARGO_TOML.read_text(encoding="utf-8")
    lines = text.splitlines(keepends=True)

    package_start = None
    for index, line in enumerate(lines):
        if line.strip() == "[package]":
            package_start = index
            break
    if package_start is None:
        fail("[package] section not found in Cargo.toml")

    version_index = None
    version_match = None
    for index in range(package_start + 1, len(lines)):
        stripped = lines[index].strip()
        if stripped.startswith("[") and stripped.endswith("]"):
            break
        match = re.match(
            r'^(\s*version\s*=\s*")([^"]+)(".*?)(\r?\n?)$',
            lines[index],
        )
        if match:
            version_index = index
            version_match = match
            break
    if version_index is None or version_match is None:
        fail("version entry not found in [package] section of Cargo.toml")

    lines[version_index] = (
        f"{version_match.group(1)}{new_version}{version_match.group(3)}"
        f"{version_match.group(4)}"
    )
    updated = "".join(lines)
    try:
        tomllib.loads(updated)
    except tomllib.TOMLDecodeError as exc:
        fail(f"Generated invalid Cargo.toml while updating version: {exc}")

    if dry_run:
        print(f"[dry-run] update {CARGO_TOML} version {old_version} -> {new_version}")
    else:
        write_atomic(CARGO_TOML, updated)

    return old_version, new_version


def read_schema_version_file() -> str:
    if not SCHEMA_VERSION_FILE.exists():
        fail(f"Missing schema version file: {SCHEMA_VERSION_FILE}")
    version = SCHEMA_VERSION_FILE.read_text(encoding="utf-8").strip()
    return validate_semver(version)


def update_schema_changelog_version(
    schema_json_text: str,
    new_version: str,
) -> str:
    updated, count = re.subn(
        r'("x-envgen-schema-version"\s*:\s*")([^"]+)(")',
        lambda match: f"{match.group(1)}{new_version}{match.group(3)}",
        schema_json_text,
        count=1,
    )
    if count != 1:
        fail('Schema JSON does not contain exactly one "x-envgen-schema-version" field')

    try:
        json.loads(updated)
    except json.JSONDecodeError as exc:
        fail(f"Updated schema JSON is invalid: {exc}")
    return updated


def changelog_has_entries(body: str) -> bool:
    for line in body.splitlines():
        stripped = line.strip()
        if not stripped:
            continue
        if stripped.startswith("### "):
            continue
        return True
    return False


def rotate_changelog(
    path: Path,
    new_version: str,
    default_sections: list[str],
    allow_empty: bool,
    dry_run: bool,
    make_override_command: str | None = None,
) -> None:
    text = path.read_text(encoding="utf-8")
    match = UNRELEASED_RE.search(text)
    if not match:
        fail(f"Missing '## [Unreleased]' section in {path}")

    body = match.group("body")
    headings = re.findall(r"^### (.+)$", body, flags=re.MULTILINE)
    if not headings:
        headings = default_sections

    if not changelog_has_entries(body) and not allow_empty:
        if make_override_command:
            fail(
                f"Unreleased section in {path} has no entries.\n"
                "Override from Make target:\n"
                f"  {make_override_command}"
            )
        fail(f"Unreleased section in {path} has no entries.")

    unreleased_block = "## [Unreleased]\n\n"
    unreleased_block += "".join(f"### {heading}\n\n" for heading in headings)
    unreleased_block = unreleased_block.rstrip() + "\n"

    clean_body = body.strip("\n")
    release_block = f"## [{new_version}] - {dt.date.today().isoformat()}\n\n"
    if clean_body:
        release_block += clean_body + "\n"
    release_block += "\n"

    replacement = unreleased_block + "\n" + release_block
    updated = text[: match.start()] + replacement + text[match.end() :]

    if dry_run:
        print(f"[dry-run] rotate changelog section in {path} for {new_version}")
    else:
        write_atomic(path, updated)


def validate_release_section(path: Path, version: str) -> None:
    pattern = re.compile(rf"^## \[{re.escape(version)}\] - ", flags=re.MULTILINE)
    if not pattern.search(path.read_text(encoding="utf-8")):
        fail(f"Missing release section '## [{version}] - YYYY-MM-DD' in {path}")


def local_tag_exists(tag_name: str) -> bool:
    result = subprocess.run(
        ["git", "show-ref", "--verify", "--quiet", f"refs/tags/{tag_name}"],
        cwd=ROOT,
        check=False,
    )
    return result.returncode == 0


def remote_tag_exists(tag_name: str) -> bool:
    result = subprocess.run(
        ["git", "ls-remote", "--tags", "origin", f"refs/tags/{tag_name}"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        fail(
            f"Failed to query remote tags for '{tag_name}': "
            f"{result.stderr.strip() or 'unknown git error'}"
        )
    return bool(result.stdout.strip())


def run_git_command(args: list[str], dry_run: bool) -> None:
    quoted = shlex.join(["git", *args])
    if dry_run:
        print(f"[dry-run] {quoted}")
        return

    result = subprocess.run(["git", *args], cwd=ROOT, check=False)
    if result.returncode != 0:
        fail(f"Command failed: {quoted}")


def sync_cargo_lockfile(dry_run: bool) -> None:
    command = shlex.join(LOCKFILE_SYNC_ARGS)
    if dry_run:
        print(f"[dry-run] would run: {command}")
        return

    result = subprocess.run(LOCKFILE_SYNC_ARGS, cwd=ROOT, check=False)
    if result.returncode != 0:
        fail(
            "Failed to synchronize Cargo.lock after crate version bump.\n"
            "Run: make sync-lockfile"
        )


def create_tag(tag_name: str, message: str, dry_run: bool) -> None:
    if local_tag_exists(tag_name):
        fail(f"Local tag already exists: {tag_name}")
    run_git_command(["tag", "-a", tag_name, "-m", message], dry_run)


def push_tag(tag_name: str, dry_run: bool) -> None:
    if not local_tag_exists(tag_name):
        fail(f"Local tag does not exist: {tag_name}. Create it first.")
    if remote_tag_exists(tag_name):
        fail(f"Remote tag already exists on origin: {tag_name}")
    run_git_command(["push", "origin", f"refs/tags/{tag_name}"], dry_run)


def resolve_tag_crate_version(*, require_release_section: bool) -> str:
    cargo_version = read_cargo_version()
    override = os.environ.get("VERSION", "").strip()
    resolved = validate_semver(override) if override else cargo_version

    if resolved != cargo_version:
        fail(
            "VERSION override does not match Cargo.toml version. "
            f"Cargo.toml has {cargo_version}, override requested {resolved}."
        )

    if require_release_section:
        validate_release_section(CHANGELOG, resolved)
    return resolved


def resolve_tag_schema_version(*, require_release_section: bool) -> str:
    schema_version = read_schema_version_file()
    override = os.environ.get("SCHEMA_VERSION", "").strip()
    resolved = validate_semver(override) if override else schema_version

    if resolved != schema_version:
        fail(
            "SCHEMA_VERSION override does not match SCHEMA_VERSION file value. "
            f"SCHEMA_VERSION has {schema_version}, override requested {resolved}."
        )

    if require_release_section:
        validate_release_section(SCHEMA_CHANGELOG, resolved)
    return resolved


def do_status(_args: argparse.Namespace) -> None:
    crate_version = read_cargo_version()
    schema_version = read_schema_version_file()
    schema_path = SCHEMA_DIR / f"envgen.schema.v{schema_version}.json"

    print(f"crate_version={crate_version}")
    print(f"schema_version={schema_version}")
    print(f"schema_file={schema_path}")
    print(f"schema_file_exists={'yes' if schema_path.exists() else 'no'}")


def do_bump_crate(args: argparse.Namespace) -> None:
    if args.level:
        make_override_command = (
            f"make bump-crate-{args.level} ALLOW_EMPTY_CHANGELOG=1"
        )
    elif args.version:
        make_override_command = (
            f"make bump-crate VERSION={args.version} ALLOW_EMPTY_CHANGELOG=1"
        )
    else:
        make_override_command = "make bump-crate ALLOW_EMPTY_CHANGELOG=1"

    old, new = update_cargo_version(
        resolve_next_version(read_cargo_version(), args.level, args.version),
        dry_run=args.dry_run,
    )

    rotate_changelog(
        CHANGELOG,
        new,
        default_sections=CRATE_SECTIONS,
        allow_empty=args.allow_empty_changelog,
        dry_run=args.dry_run,
        make_override_command=make_override_command,
    )

    sync_cargo_lockfile(args.dry_run)

    print(f"crate version: {old} -> {new}")
    print(f"updated: {CARGO_TOML}")
    print(f"updated: {CHANGELOG}")
    if not args.dry_run:
        print(f"updated: {ROOT / 'Cargo.lock'}")
    emit_next_step(
        "crate-after-bump",
        crate_version=new,
        lockfile_synced=not args.dry_run,
    )


def do_bump_schema(args: argparse.Namespace) -> None:
    if args.level:
        make_override_command = (
            f"make bump-schema-{args.level} ALLOW_EMPTY_SCHEMA_CHANGELOG=1"
        )
    elif args.version:
        make_override_command = (
            f"make bump-schema VERSION={args.version} "
            "ALLOW_EMPTY_SCHEMA_CHANGELOG=1"
        )
    else:
        make_override_command = "make bump-schema ALLOW_EMPTY_SCHEMA_CHANGELOG=1"

    old = read_schema_version_file()
    new = resolve_next_version(old, args.level, args.version)
    if old == new:
        fail("New schema version matches current version; nothing to do")

    old_schema_path = SCHEMA_DIR / f"envgen.schema.v{old}.json"
    new_schema_path = SCHEMA_DIR / f"envgen.schema.v{new}.json"

    if not old_schema_path.exists():
        fail(f"Current schema file does not exist: {old_schema_path}")
    if new_schema_path.exists():
        fail(f"Target schema file already exists: {new_schema_path}")

    updated_schema = update_schema_changelog_version(
        old_schema_path.read_text(encoding="utf-8"),
        new,
    )

    rotate_changelog(
        SCHEMA_CHANGELOG,
        new,
        default_sections=SCHEMA_SECTIONS,
        allow_empty=args.allow_empty_changelog,
        dry_run=args.dry_run,
        make_override_command=make_override_command,
    )

    if args.dry_run:
        print(f"[dry-run] write schema file: {new_schema_path}")
        print(f"[dry-run] remove schema file: {old_schema_path}")
        print(f"[dry-run] update {SCHEMA_VERSION_FILE} -> {new}")
    else:
        write_atomic(new_schema_path, updated_schema)
        old_schema_path.unlink()
        write_atomic(SCHEMA_VERSION_FILE, f"{new}\n")

    print(f"schema version: {old} -> {new}")
    print(f"updated: {SCHEMA_VERSION_FILE}")
    print(f"updated: {SCHEMA_CHANGELOG}")
    print(f"renamed: {old_schema_path} -> {new_schema_path}")
    emit_next_step("schema-after-bump", schema_version=new)


def do_tag_crate(args: argparse.Namespace) -> None:
    version = resolve_tag_crate_version(require_release_section=True)
    tag_name = f"v{version}"
    create_tag(tag_name, f"release {tag_name}", args.dry_run)
    print(f"created local tag: {tag_name}")
    emit_next_step("crate-after-tag", crate_version=version, tag_name=tag_name)


def do_push_tag_crate(args: argparse.Namespace) -> None:
    version = resolve_tag_crate_version(require_release_section=False)
    tag_name = f"v{version}"
    push_tag(tag_name, args.dry_run)
    print(f"pushed tag: {tag_name}")
    emit_next_step("crate-after-push-tag", crate_version=version, tag_name=tag_name)


def do_tag_schema(args: argparse.Namespace) -> None:
    version = resolve_tag_schema_version(require_release_section=True)
    tag_name = f"schema-v{version}"
    create_tag(tag_name, f"schema release {tag_name}", args.dry_run)
    print(f"created local tag: {tag_name}")
    emit_next_step("schema-after-tag", schema_version=version, tag_name=tag_name)


def do_push_tag_schema(args: argparse.Namespace) -> None:
    version = resolve_tag_schema_version(require_release_section=False)
    tag_name = f"schema-v{version}"
    push_tag(tag_name, args.dry_run)
    print(f"pushed tag: {tag_name}")
    emit_next_step("schema-after-push-tag", schema_version=version, tag_name=tag_name)


def do_next_step(args: argparse.Namespace) -> None:
    emit_next_step(args.stage)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Bump versions and manage release tags")
    subparsers = parser.add_subparsers(dest="command", required=True)

    status = subparsers.add_parser("status", help="Show current crate/schema versions")
    status.set_defaults(func=do_status)

    next_step = subparsers.add_parser(
        "next-step",
        help="Print guided next-step release hints for a flow stage",
    )
    next_step.add_argument("--stage", required=True, choices=NEXT_STEP_STAGES)
    next_step.set_defaults(func=do_next_step)

    bump_crate = subparsers.add_parser("bump-crate", help="Bump crate version + CHANGELOG")
    bump_crate.add_argument("--level", choices=["patch", "minor", "major"])
    bump_crate.add_argument("--version")
    bump_crate.add_argument("--allow-empty-changelog", action="store_true")
    bump_crate.add_argument("--dry-run", action="store_true")
    bump_crate.set_defaults(func=do_bump_crate)

    bump_schema = subparsers.add_parser(
        "bump-schema", help="Bump schema artifact version + SCHEMA_CHANGELOG"
    )
    bump_schema.add_argument("--level", choices=["patch", "minor", "major"])
    bump_schema.add_argument("--version")
    bump_schema.add_argument("--allow-empty-changelog", action="store_true")
    bump_schema.add_argument("--dry-run", action="store_true")
    bump_schema.set_defaults(func=do_bump_schema)

    tag_crate = subparsers.add_parser("tag-crate", help="Create local annotated crate tag")
    tag_crate.add_argument("--dry-run", action="store_true")
    tag_crate.set_defaults(func=do_tag_crate)

    push_tag_crate = subparsers.add_parser(
        "push-tag-crate", help="Push existing crate tag to origin"
    )
    push_tag_crate.add_argument("--dry-run", action="store_true")
    push_tag_crate.set_defaults(func=do_push_tag_crate)

    tag_schema = subparsers.add_parser("tag-schema", help="Create local annotated schema tag")
    tag_schema.add_argument("--dry-run", action="store_true")
    tag_schema.set_defaults(func=do_tag_schema)

    push_tag_schema = subparsers.add_parser(
        "push-tag-schema", help="Push existing schema tag to origin"
    )
    push_tag_schema.add_argument("--dry-run", action="store_true")
    push_tag_schema.set_defaults(func=do_push_tag_schema)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()

    try:
        args.func(args)
        return 0
    except BumpError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
