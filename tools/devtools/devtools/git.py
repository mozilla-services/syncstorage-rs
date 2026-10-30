"""Thin wrappers around the git commands the helpers need."""

import shutil
import subprocess

DEFAULT_BASE = "origin/master"


def _git() -> str:
    """Return the absolute path to git, failing clearly if it is missing."""
    path = shutil.which("git")
    if path is None:
        raise RuntimeError("git not found on PATH")
    return path


def run(*args: str, cwd: str | None = None) -> str:
    """Run a git command and return stdout, raising on failure."""
    result = subprocess.run(
        [_git(), *args], cwd=cwd, capture_output=True, text=True, check=True
    )
    return result.stdout


def merge_base(base: str = DEFAULT_BASE, cwd: str | None = None) -> str:
    """Return the merge-base commit between ``base`` and HEAD."""
    return run("merge-base", base, "HEAD", cwd=cwd).strip()


def changed_files(base: str = DEFAULT_BASE, cwd: str | None = None) -> list[str]:
    """Return files changed on this branch relative to its merge-base with ``base``."""
    out = run("diff", "--name-only", f"{merge_base(base, cwd)}...HEAD", cwd=cwd)
    return [line for line in out.splitlines() if line]


def exists_at(rev: str, path: str, cwd: str | None = None) -> bool:
    """Return whether ``path`` exists at revision ``rev``."""
    result = subprocess.run(
        [_git(), "cat-file", "-e", f"{rev}:{path}"],
        cwd=cwd,
        capture_output=True,
        check=False,
    )
    return result.returncode == 0
