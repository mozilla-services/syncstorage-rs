"""Map the files changed on a branch to the checks and skills they need.

Prints JSON. Skills read it instead of re-deriving backend scope by hand.
"""

import argparse
import json
import re
import sys
from pathlib import PurePosixPath
from typing import Any

from devtools import git

BACKENDS = ("mysql", "postgres", "spanner")

BACKEND_DIRS = {
    "syncstorage-mysql": "mysql",
    "tokenserver-mysql": "mysql",
    "syncstorage-postgres": "postgres",
    "tokenserver-postgres": "postgres",
    "syncstorage-spanner": "spanner",
}

TRAIT_RE = re.compile(r"^(syncstorage|tokenserver)-db(-common)?/src/")
MIGRATION_RE = re.compile(r"^[a-z-]+/migrations/")
SCHEMA_FILES = {"syncstorage-spanner/src/schema.ddl"}
DEP_FILES = {"Cargo.toml", "Cargo.lock"}


def _is_rust(path: str) -> bool:
    """Return whether ``path`` is Rust source or a Cargo manifest outside tools/."""
    if path.startswith("tools/"):
        return False
    p = PurePosixPath(path)
    return p.suffix == ".rs" or p.name in DEP_FILES


def _is_error_file(path: str) -> bool:
    """Return whether ``path`` looks like an error module."""
    p = PurePosixPath(path)
    return p.suffix == ".rs" and (p.stem == "error" or "error" in p.parts)


def classify(paths: list[str]) -> dict[str, Any]:
    """Classify changed paths into backends, clippy targets and review areas."""
    backends: set[str] = set()
    rust: list[str] = []
    for path in paths:
        if not _is_rust(path):
            continue
        rust.append(path)
        top = path.split("/", 1)[0]
        if top in BACKEND_DIRS:
            backends.add(BACKEND_DIRS[top])
        else:
            # Shared crates and the workspace manifests affect every backend.
            backends.update(BACKENDS)

    trait_files = [p for p in paths if TRAIT_RE.match(p) and p.endswith(".rs")]
    error_files = [p for p in paths if _is_error_file(p)]
    migrations = [p for p in paths if MIGRATION_RE.match(p) or p in SCHEMA_FILES]
    python = [p for p in paths if p.startswith("tools/") and p.endswith(".py")]
    deps = [p for p in paths if PurePosixPath(p).name in DEP_FILES]
    ordered = [b for b in BACKENDS if b in backends]

    skills: list[str] = []
    if rust:
        skills.append("rust-code-smell")
    if trait_files:
        skills.append("multi-backend-consistency")
    if error_files:
        skills.append("rust-error-review")
    if migrations:
        skills.append("migration-review")
    if python:
        skills.append("py-checks")
    if any(p.startswith("tools/integration_tests/") for p in python):
        skills.append("pytest-fixture-review")

    return {
        "files": paths,
        "rust_files": rust,
        "backends": ordered,
        "clippy_targets": [f"clippy_{b}" for b in ordered],
        "trait_files": trait_files,
        "error_files": error_files,
        "migrations": migrations,
        "python_files": python,
        "dependency_files": deps,
        "skills": skills,
    }


def main(argv: list[str] | None = None) -> int:
    """Print the change scope for the current branch as JSON."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default=git.DEFAULT_BASE)
    parser.add_argument(
        "paths", nargs="*", help="classify these paths instead of the git diff"
    )
    args = parser.parse_args(argv)
    paths = args.paths or git.changed_files(args.base)
    json.dump(classify(paths), sys.stdout, indent=2)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
