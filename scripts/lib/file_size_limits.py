#!/usr/bin/env python3
"""
Purpose: Enforce CueLoop file-size policy for human-authored repository files.
Responsibilities:
- Discover tracked and untracked non-ignored files when git metadata is present.
- Fall back to deterministic repository walking when git metadata is unavailable.
- Apply explicit include/exclude policy with configurable extra exclude globs.
- Report advisory, review, and fail-threshold offenders with actionable path/line details.
- Honor a reasoned fail-threshold allowlist for exceptional large files.
Scope:
- Read-only policy verification only; this script never rewrites repository files.
Usage:
- python3 scripts/lib/file_size_limits.py /path/to/repo
- python3 scripts/lib/file_size_limits.py /path/to/repo --exclude-glob 'docs/generated/**'
- python3 scripts/lib/file_size_limits.py /path/to/repo --soft-limit 1500 --review-limit 3000 --fail-limit 5000
Invariants/assumptions:
- Maintainer policy is soft advisory at 1500 LOC, review advisory at 3000 LOC, and blocking fail at 5000 LOC unless allowlisted.
- Advisory and review threshold violations are reported non-blocking.
"""

from __future__ import annotations

import argparse
import fnmatch
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence

SOFT_LIMIT_DEFAULT = 1500
REVIEW_LIMIT_DEFAULT = 3000
FAIL_LIMIT_DEFAULT = 5000
DEFAULT_ALLOWLIST_REL_PATH = "scripts/file-size-allowlist.txt"

INCLUDE_SUFFIXES = {
    ".md",
    ".py",
    ".rs",
    ".sh",
    ".swift",
    ".jsonc",
}
INCLUDE_BASENAMES = {"AGENTS.md", "Makefile"}
DEFAULT_EXCLUDE_GLOBS = (
    ".git/**",
    "target/**",
    ".cueloop/done.jsonc",
    ".cueloop/queue.jsonc",
    ".cueloop/config.jsonc",
    ".cueloop/cache/**",
    ".cueloop/workspaces/**",
    ".cueloop/lock/**",
    ".cueloop/logs/**",
    ".venv/**",
    ".pytest_cache/**",
    ".ty_cache/**",
    "docs/assets/images/**",
    "schemas/*.json",
    "apps/**/*.xcodeproj/project.pbxproj",
)
EXTRA_EXCLUDE_ENV_VAR = "CUELOOP_FILE_SIZE_EXCLUDE_GLOBS"


@dataclass(frozen=True)
class Offender:
    rel_path: str
    line_count: int
    reason: str | None = None


@dataclass(frozen=True)
class AllowlistEntry:
    pattern: str
    reason: str


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Enforce CueLoop file-size limits for human-authored repository files.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            "Exit codes:\n"
            "  0  no fail-threshold violations (advisories may still be reported)\n"
            "  1  one or more fail-threshold violations found\n"
            "  2  usage, argument, or allowlist error"
        ),
    )
    parser.add_argument(
        "repo_root",
        nargs="?",
        default=".",
        help="Repository root to scan (default: current working directory)",
    )
    parser.add_argument(
        "--soft-limit",
        type=int,
        default=SOFT_LIMIT_DEFAULT,
        help=f"Soft advisory line-count threshold (default: {SOFT_LIMIT_DEFAULT})",
    )
    parser.add_argument(
        "--review-limit",
        type=int,
        default=REVIEW_LIMIT_DEFAULT,
        help=f"Review advisory line-count threshold (default: {REVIEW_LIMIT_DEFAULT})",
    )
    parser.add_argument(
        "--fail-limit",
        type=int,
        default=FAIL_LIMIT_DEFAULT,
        help=f"Blocking fail line-count threshold (default: {FAIL_LIMIT_DEFAULT})",
    )
    parser.add_argument(
        "--hard-limit",
        type=int,
        dest="fail_limit",
        help="Deprecated alias for --fail-limit; kept for older scripts.",
    )
    parser.add_argument(
        "--allowlist",
        type=Path,
        default=None,
        help=(
            "Fail-threshold allowlist file. Defaults to "
            f"{DEFAULT_ALLOWLIST_REL_PATH} when that file exists. "
            "Each non-comment line must be: glob | reason."
        ),
    )
    parser.add_argument(
        "--exclude-glob",
        action="append",
        default=[],
        help=(
            "Additional fnmatch glob to exclude (repeatable). "
            f"You can also set {EXTRA_EXCLUDE_ENV_VAR} as newline-separated globs."
        ),
    )
    return parser.parse_args()


def parse_env_patterns(raw: str) -> list[str]:
    patterns: list[str] = []
    for line in raw.splitlines():
        pattern = line.strip()
        if pattern:
            patterns.append(pattern)
    return patterns


def unique_ordered(values: Iterable[str]) -> list[str]:
    seen: set[str] = set()
    ordered: list[str] = []
    for value in values:
        if value in seen:
            continue
        seen.add(value)
        ordered.append(value)
    return ordered


def normalize_rel_path(raw_path: str) -> str:
    normalized = raw_path.replace("\\", "/")
    while normalized.startswith("./"):
        normalized = normalized[2:]
    return normalized


def is_git_worktree(repo_root: Path) -> bool:
    result = subprocess.run(
        ["git", "-C", str(repo_root), "rev-parse", "--is-inside-work-tree"],
        check=False,
        capture_output=True,
        text=True,
    )
    return result.returncode == 0 and result.stdout.strip() == "true"


def git_list_files(repo_root: Path, args: Sequence[str]) -> list[str]:
    result = subprocess.run(
        ["git", "-C", str(repo_root), *args],
        check=False,
        capture_output=True,
    )
    if result.returncode != 0:
        stderr = result.stderr.decode("utf-8", errors="replace").strip()
        raise RuntimeError(f"git {' '.join(args)} failed: {stderr}")

    paths: list[str] = []
    for raw in result.stdout.split(b"\0"):
        if not raw:
            continue
        decoded = raw.decode("utf-8", errors="surrogateescape")
        paths.append(normalize_rel_path(decoded))
    return paths


def walk_repo_files(repo_root: Path) -> list[str]:
    rel_paths: list[str] = []
    for current_root, dirnames, filenames in os.walk(repo_root):
        current_root_path = Path(current_root)
        rel_dir = current_root_path.relative_to(repo_root).as_posix()

        if rel_dir == ".":
            rel_dir = ""

        dirnames[:] = [
            name
            for name in dirnames
            if name
            not in {
                ".git",
                "target",
                ".venv",
                ".pytest_cache",
                ".ty_cache",
            }
        ]

        for filename in filenames:
            rel_path = "/".join(part for part in (rel_dir, filename) if part)
            rel_paths.append(rel_path)
    return rel_paths


def discover_candidate_paths(repo_root: Path) -> list[str]:
    if is_git_worktree(repo_root):
        tracked = git_list_files(repo_root, ["ls-files", "-z"])
        untracked = git_list_files(
            repo_root,
            ["ls-files", "--others", "--exclude-standard", "-z"],
        )
        return sorted(set(tracked) | set(untracked))

    return sorted(set(walk_repo_files(repo_root)))


def is_excluded(rel_path: str, exclude_patterns: Sequence[str]) -> bool:
    return any(fnmatch.fnmatchcase(rel_path, pattern) for pattern in exclude_patterns)


def should_check(rel_path: str, exclude_patterns: Sequence[str]) -> bool:
    if is_excluded(rel_path, exclude_patterns):
        return False

    filename = Path(rel_path).name
    if filename in INCLUDE_BASENAMES:
        return True

    return Path(rel_path).suffix in INCLUDE_SUFFIXES


def count_lines(path: Path) -> int:
    data = path.read_bytes()
    if not data:
        return 0
    return data.count(b"\n") + (0 if data.endswith(b"\n") else 1)


def parse_allowlist_file(path: Path) -> list[AllowlistEntry]:
    entries: list[AllowlistEntry] = []
    for index, raw_line in enumerate(path.read_text().splitlines(), start=1):
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if "|" not in line:
            raise ValueError(
                f"{path}:{index}: allowlist entries must use 'glob | reason'"
            )
        pattern, reason = [part.strip() for part in line.split("|", 1)]
        if not pattern or not reason:
            raise ValueError(
                f"{path}:{index}: allowlist entries require both glob and reason"
            )
        entries.append(AllowlistEntry(pattern=normalize_rel_path(pattern), reason=reason))
    return entries


def load_allowlist(repo_root: Path, requested_path: Path | None) -> list[AllowlistEntry]:
    allowlist_path = requested_path
    if allowlist_path is None:
        default_path = repo_root / DEFAULT_ALLOWLIST_REL_PATH
        if not default_path.exists():
            return []
        allowlist_path = default_path
    elif not allowlist_path.is_absolute():
        allowlist_path = repo_root / allowlist_path

    if not allowlist_path.exists():
        raise ValueError(f"allowlist file does not exist: {allowlist_path}")
    if not allowlist_path.is_file():
        raise ValueError(f"allowlist path is not a file: {allowlist_path}")

    return parse_allowlist_file(allowlist_path)


def allowlist_reason(rel_path: str, allowlist: Sequence[AllowlistEntry]) -> str | None:
    for entry in allowlist:
        if fnmatch.fnmatchcase(rel_path, entry.pattern):
            return entry.reason
    return None


def classify_offenders(
    repo_root: Path,
    rel_paths: Sequence[str],
    soft_limit: int,
    review_limit: int,
    fail_limit: int,
    exclude_patterns: Sequence[str],
    allowlist: Sequence[AllowlistEntry],
) -> tuple[list[Offender], list[Offender], list[Offender], list[Offender], int]:
    soft_offenders: list[Offender] = []
    review_offenders: list[Offender] = []
    fail_offenders: list[Offender] = []
    allowlisted_fail_offenders: list[Offender] = []
    checked_files = 0

    for rel_path in rel_paths:
        rel_path = normalize_rel_path(rel_path)
        if not should_check(rel_path, exclude_patterns):
            continue

        abs_path = repo_root / rel_path
        if not abs_path.exists() or not abs_path.is_file():
            continue

        try:
            line_count = count_lines(abs_path)
        except OSError:
            continue

        checked_files += 1
        if line_count > fail_limit:
            reason = allowlist_reason(rel_path, allowlist)
            if reason:
                allowlisted_fail_offenders.append(
                    Offender(rel_path=rel_path, line_count=line_count, reason=reason)
                )
            else:
                fail_offenders.append(Offender(rel_path=rel_path, line_count=line_count))
        elif line_count > review_limit:
            review_offenders.append(Offender(rel_path=rel_path, line_count=line_count))
        elif line_count > soft_limit:
            soft_offenders.append(Offender(rel_path=rel_path, line_count=line_count))

    for offenders in (
        soft_offenders,
        review_offenders,
        fail_offenders,
        allowlisted_fail_offenders,
    ):
        offenders.sort(key=lambda item: (-item.line_count, item.rel_path))
    return (
        soft_offenders,
        review_offenders,
        fail_offenders,
        allowlisted_fail_offenders,
        checked_files,
    )


def print_offenders(label: str, offenders: Sequence[Offender], include_reason: bool = False) -> None:
    print(label)
    for offender in offenders:
        suffix = f"  # {offender.reason}" if include_reason and offender.reason else ""
        print(f"  {offender.line_count:>5}  {offender.rel_path}{suffix}")


def main() -> int:
    args = parse_args()

    if args.soft_limit <= 0 or args.review_limit <= 0 or args.fail_limit <= 0:
        print(
            "ERROR: --soft-limit, --review-limit, and --fail-limit must be positive integers",
            file=sys.stderr,
        )
        return 2
    if args.soft_limit >= args.review_limit:
        print("ERROR: --soft-limit must be less than --review-limit", file=sys.stderr)
        return 2
    if args.review_limit >= args.fail_limit:
        print("ERROR: --review-limit must be less than --fail-limit", file=sys.stderr)
        return 2

    repo_root = Path(args.repo_root).resolve()
    if not repo_root.exists() or not repo_root.is_dir():
        print(f"ERROR: repo root is not a directory: {repo_root}", file=sys.stderr)
        return 2

    env_patterns = parse_env_patterns(os.environ.get(EXTRA_EXCLUDE_ENV_VAR, ""))
    exclude_patterns = unique_ordered(
        [*DEFAULT_EXCLUDE_GLOBS, *env_patterns, *args.exclude_glob]
    )

    try:
        allowlist = load_allowlist(repo_root, args.allowlist)
        rel_paths = discover_candidate_paths(repo_root)
    except (RuntimeError, ValueError) as err:
        print(f"ERROR: {err}", file=sys.stderr)
        return 2

    (
        soft_offenders,
        review_offenders,
        fail_offenders,
        allowlisted_fail_offenders,
        checked_files,
    ) = classify_offenders(
        repo_root=repo_root,
        rel_paths=rel_paths,
        soft_limit=args.soft_limit,
        review_limit=args.review_limit,
        fail_limit=args.fail_limit,
        exclude_patterns=exclude_patterns,
        allowlist=allowlist,
    )

    print(
        "Checked "
        f"{checked_files} files "
        f"(soft>{args.soft_limit}, review>{args.review_limit}, fail>{args.fail_limit})"
    )

    if soft_offenders:
        print_offenders("ADVISORY: soft file-size threshold exceeded:", soft_offenders)
        print("ADVISORY: soft offenders are non-blocking cleanup candidates.")

    if review_offenders:
        print_offenders("WARN: review file-size threshold exceeded:", review_offenders)
        print("WARN: review offenders are non-blocking but should be split or justified.")

    if allowlisted_fail_offenders:
        print_offenders(
            "ALLOWLISTED: fail file-size threshold exceeded:",
            allowlisted_fail_offenders,
            include_reason=True,
        )
        print("ALLOWLISTED: entries are non-blocking while their reasons remain valid.")

    if fail_offenders:
        print_offenders("ERROR: fail file-size threshold exceeded:", fail_offenders)
        print("ERROR: split these files or add a reasoned allowlist entry.")
        return 1

    if not (soft_offenders or review_offenders or allowlisted_fail_offenders):
        print("OK: file-size limits within policy")

    return 0


if __name__ == "__main__":
    sys.exit(main())
