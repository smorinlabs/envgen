#!/usr/bin/env python3
"""Homebrew tap release helpers for envgen."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SOURCE_DIR = ROOT / ".homebrew"
DEFAULT_SOURCE_REPO = "smorinlabs/envgen"
DEFAULT_TAP_REPO = "smorinlabs/homebrew-tap"
DEFAULT_TAP_REPO_DIR = Path("/Users/stevemorin/c/homebrew-tap")
DEFAULT_TAP_FORMULA = "Formula/envgen.rb"
DOWNLOAD_CHUNK_SIZE = 1024 * 1024
TAG_PATTERN = re.compile(r"^v(\d+\.\d+\.\d+)(?:[.-].*)?$")
TRUTHY_ENV_VALUES = {"1", "true", "yes", "on"}
FALSY_ENV_VALUES = {"", "0", "false", "no", "off"}
NEXT_STEP_STAGES = (
    "tap-after-source",
    "tap-after-sync",
    "tap-after-verify",
    "tap-after-pr",
)


class TapReleaseError(RuntimeError):
    """Raised when the tap release flow should fail with a user-facing error."""


def fail(message: str) -> None:
    raise TapReleaseError(message)


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


def parse_tag(tag: str) -> str:
    match = TAG_PATTERN.fullmatch(tag.strip())
    if not match:
        fail(f"Invalid tag '{tag}'. Expected vX.Y.Z")
    return match.group(1)


def sanitized_tag_for_filename(tag: str) -> str:
    return re.sub(r"[^A-Za-z0-9._-]", "_", tag)


def default_source_json_path(tag: str) -> Path:
    safe_tag = sanitized_tag_for_filename(tag)
    return DEFAULT_SOURCE_DIR / f"source-{safe_tag}.json"


def requested_tarball_url(source_repo: str, tag: str) -> str:
    return f"https://github.com/{source_repo}/archive/refs/tags/{tag}.tar.gz"


def tap_repo_to_tap_name(tap_repo: str) -> str:
    if "/" not in tap_repo:
        fail(f"Invalid tap repo '{tap_repo}'. Expected owner/repo")
    owner, repo = tap_repo.split("/", 1)
    if repo.startswith("homebrew-"):
        repo = repo[len("homebrew-") :]
    return f"{owner}/{repo}"


def download_tarball_with_retries(
    *,
    url: str,
    destination: Path,
    attempts: int,
    sleep_seconds: float,
) -> tuple[str, int]:
    destination.parent.mkdir(parents=True, exist_ok=True)

    last_error: Exception | None = None
    for attempt in range(1, attempts + 1):
        try:
            request = urllib.request.Request(
                url,
                headers={"User-Agent": "envgen-homebrew-tap-release/1.0"},
            )
            with urllib.request.urlopen(request, timeout=60) as response:
                final_url = response.geturl()
                with destination.open("wb") as output_file:
                    while True:
                        chunk = response.read(DOWNLOAD_CHUNK_SIZE)
                        if not chunk:
                            break
                        output_file.write(chunk)
            return final_url, destination.stat().st_size
        except Exception as exc:  # noqa: BLE001
            last_error = exc
            if attempt == attempts:
                break
            time.sleep(sleep_seconds)

    assert last_error is not None
    fail(
        "Failed to download source tarball "
        f"after {attempts} attempts: {last_error}"
    )


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as input_file:
        for chunk in iter(lambda: input_file.read(DOWNLOAD_CHUNK_SIZE), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_json_atomic(path: Path, data: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp_path = path.with_suffix(path.suffix + ".tmp")
    tmp_path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    tmp_path.replace(path)


def read_source_metadata(path: Path) -> dict[str, object]:
    if not path.exists():
        fail(
            "Source metadata JSON does not exist: "
            f"{path}. Run `make homebrew-source TAG=vX.Y.Z` first."
        )
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        fail(f"Failed to parse source metadata JSON {path}: {exc}")

    required_fields = [
        "tag",
        "version",
        "requested_url",
        "sha256",
        "download_path",
    ]
    for field in required_fields:
        if field not in raw:
            fail(f"Source metadata missing required field '{field}': {path}")
    return raw


def formula_template(*, source_url: str, sha256: str) -> str:
    return f"""class Envgen < Formula
  desc \"Generate .env files from declarative YAML schemas\"
  homepage \"https://github.com/smorinlabs/envgen\"
  url \"{source_url}\"
  sha256 \"{sha256}\"
  license \"MIT\"
  head \"https://github.com/smorinlabs/envgen.git\", branch: \"main\"

  depends_on \"rust\" => :build

  def install
    system \"cargo\", \"install\", *std_cargo_args
  end

  test do
    (testpath/\"envgen.yaml\").write <<~YAML
      schema_version: \"2\"
      metadata:
        description: \"Homebrew test schema\"
        destination:
          local: \".env.local\"
      environments:
        local: {{}}
      sources: {{}}
      variables:
        APP_NAME:
          description: \"App name\"
          source: static
          values:
            local: \"envgen\"
    YAML

    system bin/\"envgen\", \"check\", \"-c\", \"envgen.yaml\"
    system bin/\"envgen\", \"pull\", \"-c\", \"envgen.yaml\", \"-e\", \"local\", \"--force\"
    assert_match \"APP_NAME=envgen\", (testpath/\".env.local\").read
    assert_match version.to_s, shell_output(\"#{{bin}}/envgen --version\")
  end
end
"""


def run_command(
    args: list[str],
    *,
    cwd: Path | None = None,
    capture_stdout: bool = False,
    allow_nonzero: bool = False,
) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        args,
        cwd=str(cwd) if cwd else None,
        text=True,
        capture_output=capture_stdout,
        check=False,
    )
    if result.returncode != 0 and not allow_nonzero:
        stderr = result.stderr.strip() if result.stderr else ""
        stdout = result.stdout.strip() if result.stdout else ""
        details = stderr or stdout or f"exit code {result.returncode}"
        fail(f"Command failed: {' '.join(args)}\n{details}")
    return result


def render_next_step(
    stage: str,
    *,
    tag: str | None = None,
    tap_repo_dir: Path | None = None,
    tap_repo: str | None = None,
    formula_path: str | None = None,
    pr_url: str | None = None,
) -> tuple[str, list[str]]:
    resolved_tag = tag or "vX.Y.Z"
    resolved_repo_dir = str(tap_repo_dir or DEFAULT_TAP_REPO_DIR)
    resolved_formula = formula_path or DEFAULT_TAP_FORMULA
    resolved_tap_repo = tap_repo or DEFAULT_TAP_REPO

    if stage == "tap-after-source":
        return (
            f"Homebrew source metadata resolved for {resolved_tag}.",
            [
                (
                    "$ make homebrew-sync-formula "
                    f"TAG={resolved_tag} TAP_REPO_DIR={resolved_repo_dir}"
                ),
            ],
        )

    if stage == "tap-after-sync":
        return (
            "Tap formula synchronized from source metadata.",
            [
                (
                    "$ make homebrew-verify-formula "
                    f"TAP_REPO_DIR={resolved_repo_dir}"
                ),
            ],
        )

    if stage == "tap-after-verify":
        return (
            "Tap formula verification passed.",
            [
                (
                    "$ make homebrew-open-tap-pr "
                    f"TAG={resolved_tag} TAP_REPO_DIR={resolved_repo_dir} "
                    f"HOMEBREW_TAP_REPO={resolved_tap_repo}"
                ),
            ],
        )

    if stage == "tap-after-pr":
        next_lines = []
        if pr_url:
            next_lines.append(f"Tap PR: {pr_url}")
        next_lines.extend(
            [
                "Review and merge the tap PR after checks pass.",
                (
                    "Install path for users: "
                    "brew tap smorinlabs/tap && brew install envgen"
                ),
            ]
        )
        return (
            "Tap pull request is ready.",
            next_lines,
        )

    fail(f"Unsupported next-step stage: {stage}")


def emit_next_step(
    stage: str,
    *,
    tag: str | None = None,
    tap_repo_dir: Path | None = None,
    tap_repo: str | None = None,
    formula_path: str | None = None,
    pr_url: str | None = None,
) -> None:
    if not hints_enabled():
        return

    summary, lines = render_next_step(
        stage,
        tag=tag,
        tap_repo_dir=tap_repo_dir,
        tap_repo=tap_repo,
        formula_path=formula_path,
        pr_url=pr_url,
    )
    print("")
    print(f"Hint: {summary}")
    print("Next:")
    for line in lines:
        print(f"  {line}")


def do_status(args: argparse.Namespace) -> None:
    version = parse_tag(args.tag)
    source_json = args.source_json or default_source_json_path(args.tag)
    requested_url = requested_tarball_url(args.source_repo, args.tag)

    print(f"tag={args.tag}")
    print(f"version={version}")
    print(f"requested_url={requested_url}")
    print(f"source_json={source_json}")
    print(f"tap_repo={args.tap_repo}")
    print(f"tap_repo_dir={args.tap_repo_dir}")
    print(f"tap_formula={args.formula_path}")

    if source_json.exists():
        metadata = read_source_metadata(source_json)
        print(f"resolved_url={metadata['resolved_url']}")
        print(f"sha256={metadata['sha256']}")
        print(f"download_path={metadata['download_path']}")
    else:
        print("source_json_exists=false")


def do_resolve_source(args: argparse.Namespace) -> None:
    tag = args.tag.strip()
    version = parse_tag(tag)
    source_dir = args.source_dir
    output_json = args.out_json or default_source_json_path(tag)
    tarball_name = f"envgen-{version}.tar.gz"
    download_path = source_dir / tarball_name

    requested_url = requested_tarball_url(args.source_repo, tag)
    resolved_url, file_size = download_tarball_with_retries(
        url=requested_url,
        destination=download_path,
        attempts=args.attempts,
        sleep_seconds=args.sleep_seconds,
    )

    sha256 = sha256_file(download_path)
    payload: dict[str, object] = {
        "created_at_utc": dt.datetime.now(dt.UTC).replace(microsecond=0).isoformat(),
        "download_path": str(download_path.resolve()),
        "requested_url": requested_url,
        "resolved_url": resolved_url,
        "sha256": sha256,
        "size_bytes": file_size,
        "source_repo": args.source_repo,
        "tag": tag,
        "version": version,
    }
    write_json_atomic(output_json, payload)

    print(f"tag={tag}")
    print(f"version={version}")
    print(f"requested_url={requested_url}")
    print(f"resolved_url={resolved_url}")
    print(f"sha256={sha256}")
    print(f"download_path={download_path.resolve()}")
    print(f"source_json={output_json.resolve()}")
    emit_next_step("tap-after-source", tag=tag)


def do_sync_formula(args: argparse.Namespace) -> None:
    tag = args.tag.strip()
    parse_tag(tag)
    formula_path = args.formula_path

    if args.source_json:
        metadata = read_source_metadata(args.source_json)
        metadata_tag = str(metadata["tag"])
        if metadata_tag != tag:
            fail(
                "Source metadata tag mismatch: "
                f"expected {tag}, found {metadata_tag} in {args.source_json}"
            )
        source_url = str(metadata["requested_url"])
        sha256 = str(metadata["sha256"])
    else:
        if not args.sha256:
            fail("Provide either --source-json or --sha256")
        source_url = requested_tarball_url(args.source_repo, tag)
        sha256 = args.sha256

    new_content = formula_template(source_url=source_url, sha256=sha256)

    old_content = formula_path.read_text(encoding="utf-8") if formula_path.exists() else ""
    changed = old_content != new_content

    if args.dry_run:
        if changed:
            print(f"[dry-run] write formula: {formula_path}")
        else:
            print(f"[dry-run] no formula changes: {formula_path}")
    else:
        formula_path.parent.mkdir(parents=True, exist_ok=True)
        formula_path.write_text(new_content, encoding="utf-8")

    print(f"formula_path={formula_path}")
    print(f"changed={'true' if changed else 'false'}")
    print(f"source_url={source_url}")
    print(f"sha256={sha256}")

    emit_next_step(
        "tap-after-sync",
        tag=tag,
        formula_path=str(formula_path),
    )


def do_verify_formula(args: argparse.Namespace) -> None:
    tap_repo_dir = args.tap_repo_dir
    if not tap_repo_dir.exists():
        fail(f"Tap repo directory does not exist: {tap_repo_dir}")

    formula_path = args.formula_path
    full_formula_path = tap_repo_dir / formula_path
    if not full_formula_path.exists():
        fail(f"Formula file does not exist: {full_formula_path}")

    if shutil.which("brew") is None:
        fail("`brew` is required for formula verification")

    tap_name = tap_repo_to_tap_name(args.tap_repo)
    formula_name = formula_path.stem
    tapped_formula = f"{tap_name}/{formula_name}"

    run_command(
        ["brew", "tap", "--custom-remote", tap_name, str(tap_repo_dir)],
    )
    run_command(["brew", "style", str(formula_path)], cwd=tap_repo_dir)
    run_command(
        ["brew", "audit", "--strict", "--tap", tap_name, formula_name],
    )
    run_command(
        ["brew", "install", "--build-from-source", tapped_formula],
    )
    run_command(["brew", "test", tapped_formula])

    print(f"verified_formula={full_formula_path}")
    emit_next_step(
        "tap-after-verify",
        tag=args.tag,
        tap_repo_dir=tap_repo_dir,
        formula_path=str(formula_path),
    )


def get_existing_pr_url(*, tap_repo: str, branch: str) -> str | None:
    result = run_command(
        [
            "gh",
            "pr",
            "list",
            "--repo",
            tap_repo,
            "--head",
            branch,
            "--json",
            "number,url",
            "--limit",
            "1",
        ],
        capture_stdout=True,
    )
    prs = json.loads(result.stdout)
    if not prs:
        return None
    return str(prs[0]["url"])


def do_open_pr(args: argparse.Namespace) -> None:
    tag = args.tag.strip()
    version = parse_tag(tag)
    tap_repo_dir = args.tap_repo_dir
    formula_path = args.formula_path
    full_formula_path = tap_repo_dir / formula_path

    if not tap_repo_dir.exists():
        fail(f"Tap repo directory does not exist: {tap_repo_dir}")
    if not full_formula_path.exists():
        fail(f"Formula file does not exist: {full_formula_path}")
    if shutil.which("gh") is None:
        fail("`gh` CLI is required to open or update tap pull requests")

    branch = f"envgen-{version}"
    title = f"envgen {version}"
    body = "\n".join(
        [
            f"Update envgen formula to {tag}.",
            "",
            f"- Source tag: `{tag}`",
            (
                "- Source tarball: "
                f"`{requested_tarball_url(DEFAULT_SOURCE_REPO, tag)}`"
            ),
        ]
    )

    run_command(["git", "fetch", "origin", args.base_branch], cwd=tap_repo_dir)
    remote_branch = run_command(
        ["git", "ls-remote", "--heads", "origin", branch],
        cwd=tap_repo_dir,
        capture_stdout=True,
    )
    branch_exists = bool(remote_branch.stdout.strip())
    start_ref = f"origin/{branch}" if branch_exists else f"origin/{args.base_branch}"
    run_command(
        ["git", "checkout", "-B", branch, start_ref],
        cwd=tap_repo_dir,
    )
    run_command(["git", "add", str(formula_path)], cwd=tap_repo_dir)

    staged_diff = run_command(
        ["git", "diff", "--cached", "--quiet"],
        cwd=tap_repo_dir,
        allow_nonzero=True,
    )
    has_changes = staged_diff.returncode != 0

    if has_changes:
        run_command(
            ["git", "commit", "-m", f"envgen {version}"],
            cwd=tap_repo_dir,
        )
        if not args.dry_run:
            run_command(
                ["git", "push", "--force-with-lease", "origin", branch],
                cwd=tap_repo_dir,
            )
        else:
            print(f"[dry-run] git push --force-with-lease origin {branch}")

    pr_url: str | None = None
    if args.dry_run:
        print("[dry-run] skip gh pr create/edit")
    else:
        pr_url = get_existing_pr_url(tap_repo=args.tap_repo, branch=branch)
        if pr_url:
            run_command(
                [
                    "gh",
                    "pr",
                    "edit",
                    pr_url,
                    "--repo",
                    args.tap_repo,
                    "--title",
                    title,
                    "--body",
                    body,
                ]
            )
        else:
            create_result = run_command(
                [
                    "gh",
                    "pr",
                    "create",
                    "--repo",
                    args.tap_repo,
                    "--base",
                    args.base_branch,
                    "--head",
                    branch,
                    "--title",
                    title,
                    "--body",
                    body,
                ],
                capture_stdout=True,
            )
            created = create_result.stdout.strip()
            if created:
                pr_url = created.splitlines()[-1].strip()

            if not pr_url:
                pr_url = get_existing_pr_url(tap_repo=args.tap_repo, branch=branch)

    print(f"tap_repo={args.tap_repo}")
    print(f"tap_branch={branch}")
    print(f"has_changes={'true' if has_changes else 'false'}")
    if pr_url:
        print(f"pr_url={pr_url}")

    emit_next_step(
        "tap-after-pr",
        tag=tag,
        tap_repo_dir=tap_repo_dir,
        tap_repo=args.tap_repo,
        formula_path=str(formula_path),
        pr_url=pr_url,
    )


def do_next_step(args: argparse.Namespace) -> None:
    emit_next_step(
        args.stage,
        tag=args.tag,
        tap_repo_dir=args.tap_repo_dir,
        tap_repo=args.tap_repo,
        formula_path=args.formula_path,
    )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Manage envgen Homebrew tap release flow")
    subparsers = parser.add_subparsers(dest="command", required=True)

    status = subparsers.add_parser("status", help="Show Homebrew tap release status for a tag")
    status.add_argument("--tag", required=True)
    status.add_argument("--source-repo", default=DEFAULT_SOURCE_REPO)
    status.add_argument("--source-json", type=Path)
    status.add_argument("--tap-repo", default=DEFAULT_TAP_REPO)
    status.add_argument("--tap-repo-dir", type=Path, default=DEFAULT_TAP_REPO_DIR)
    status.add_argument("--formula-path", default=DEFAULT_TAP_FORMULA)
    status.set_defaults(func=do_status)

    resolve_source = subparsers.add_parser(
        "resolve-source",
        help="Download and hash GitHub source tarball for a release tag",
    )
    resolve_source.add_argument("--tag", required=True)
    resolve_source.add_argument("--source-repo", default=DEFAULT_SOURCE_REPO)
    resolve_source.add_argument("--source-dir", type=Path, default=DEFAULT_SOURCE_DIR)
    resolve_source.add_argument("--out-json", type=Path)
    resolve_source.add_argument("--attempts", type=int, default=5)
    resolve_source.add_argument("--sleep-seconds", type=float, default=3.0)
    resolve_source.set_defaults(func=do_resolve_source)

    sync_formula = subparsers.add_parser(
        "sync-formula",
        help="Create or update Formula/envgen.rb from source metadata",
    )
    sync_formula.add_argument("--tag", required=True)
    sync_formula.add_argument("--formula-path", type=Path, required=True)
    sync_formula.add_argument("--source-repo", default=DEFAULT_SOURCE_REPO)
    sync_formula.add_argument("--source-json", type=Path)
    sync_formula.add_argument("--sha256")
    sync_formula.add_argument("--dry-run", action="store_true")
    sync_formula.set_defaults(func=do_sync_formula)

    verify_formula = subparsers.add_parser(
        "verify-formula",
        help="Run brew style/audit/install/test for the tap formula",
    )
    verify_formula.add_argument("--tag", required=True)
    verify_formula.add_argument("--tap-repo-dir", type=Path, required=True)
    verify_formula.add_argument("--tap-repo", default=DEFAULT_TAP_REPO)
    verify_formula.add_argument("--formula-path", type=Path, default=Path(DEFAULT_TAP_FORMULA))
    verify_formula.set_defaults(func=do_verify_formula)

    open_pr = subparsers.add_parser(
        "open-pr",
        help="Open or update a pull request in the tap repository",
    )
    open_pr.add_argument("--tag", required=True)
    open_pr.add_argument("--tap-repo", default=DEFAULT_TAP_REPO)
    open_pr.add_argument("--tap-repo-dir", type=Path, required=True)
    open_pr.add_argument("--formula-path", type=Path, default=Path(DEFAULT_TAP_FORMULA))
    open_pr.add_argument("--base-branch", default="main")
    open_pr.add_argument("--dry-run", action="store_true")
    open_pr.set_defaults(func=do_open_pr)

    next_step = subparsers.add_parser(
        "next-step",
        help="Print guided next-step hints for Homebrew tap flow",
    )
    next_step.add_argument("--stage", required=True, choices=NEXT_STEP_STAGES)
    next_step.add_argument("--tag")
    next_step.add_argument("--tap-repo", default=DEFAULT_TAP_REPO)
    next_step.add_argument("--tap-repo-dir", type=Path, default=DEFAULT_TAP_REPO_DIR)
    next_step.add_argument("--formula-path", default=DEFAULT_TAP_FORMULA)
    next_step.set_defaults(func=do_next_step)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()

    try:
        args.func(args)
        return 0
    except TapReleaseError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1
    except (urllib.error.HTTPError, urllib.error.URLError) as exc:
        print(f"ERROR: network failure: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
