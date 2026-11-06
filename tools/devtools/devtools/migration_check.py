"""Check changed migrations against the published-migration policy.

Published migrations and schema.ddl must never be edited. New migrations need
an up.sql and a real down.sql, a timestamp newer than every existing migration
in that crate, and destructive SQL gets flagged for review. Prints JSON and
exits 1 when there are violations.
"""

import argparse
import json
import re
import sys
from collections.abc import Callable
from dataclasses import asdict, dataclass, field
from pathlib import Path, PurePosixPath

from devtools import git

MIGRATION_RE = re.compile(r"^(?P<crate>[a-z-]+)/migrations/(?P<dir>[^/]+)/")
SCHEMA_FILES = {"syncstorage-spanner/src/schema.ddl"}
DESTRUCTIVE_RE = re.compile(
    r"\b(DROP\s+TABLE|DROP\s+COLUMN|DROP\s+INDEX|TRUNCATE|DELETE\s+FROM)\b",
    re.IGNORECASE,
)
COMMENT_RE = re.compile(r"--[^\n]*|/\*.*?\*/", re.DOTALL)


@dataclass
class Report:
    """Result of a migration check."""

    violations: list[str] = field(default_factory=list)
    new_migrations: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)

    @property
    def ok(self) -> bool:
        """Return whether no blocking violations were found."""
        return not self.violations


def _sql_body(text: str) -> str:
    """Return ``text`` with SQL comments and whitespace removed."""
    return COMMENT_RE.sub("", text).strip()


def check(
    changed: list[str],
    is_published: Callable[[str], bool],
    existing_dirs: Callable[[str], list[str]],
    read: Callable[[str], str | None],
) -> Report:
    """Check ``changed`` paths.

    ``is_published(path)`` says whether the path exists on the base branch,
    ``existing_dirs(crate)`` lists the crate's migration dirs on the base
    branch, and ``read(path)`` returns the file's current text or None.
    """
    report = Report()
    new_dirs: dict[str, set[str]] = {}

    for path in changed:
        if path in SCHEMA_FILES:
            report.violations.append(f"{path}: published schema edited")
            continue
        m = MIGRATION_RE.match(path)
        if m is None:
            if path.endswith(".sql"):
                report.warnings.append(f"{path}: non-migration SQL changed")
            continue
        if is_published(path):
            report.violations.append(f"{path}: published migration edited")
            continue
        new_dirs.setdefault(m["crate"], set()).add(m["dir"])

    for crate, dirs in sorted(new_dirs.items()):
        existing = existing_dirs(crate)
        latest = max(existing, default="")
        for d in sorted(dirs):
            base = f"{crate}/migrations/{d}"
            report.new_migrations.append(base)
            up = read(f"{base}/up.sql")
            down = read(f"{base}/down.sql")
            if up is None or not _sql_body(up):
                report.violations.append(f"{base}: missing or empty up.sql")
            if down is None or not _sql_body(down):
                report.violations.append(f"{base}: missing or empty down.sql")
            if d <= latest:
                report.violations.append(
                    f"{base}: timestamp not newer than {crate}/migrations/{latest}"
                )
            if up is not None:
                for op in sorted({o.upper() for o in DESTRUCTIVE_RE.findall(up)}):
                    report.warnings.append(f"{base}/up.sql: destructive op {op}")

    return report


def main(argv: list[str] | None = None) -> int:
    """Run the check against the current branch and print JSON."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default=git.DEFAULT_BASE)
    args = parser.parse_args(argv)

    root = Path(git.run("rev-parse", "--show-toplevel").strip())
    base = git.merge_base(args.base)

    def existing_dirs(crate: str) -> list[str]:
        out = git.run("ls-tree", "--name-only", base, f"{crate}/migrations/")
        return [PurePosixPath(line).name for line in out.splitlines() if line]

    def read(path: str) -> str | None:
        f = root / path
        return f.read_text() if f.is_file() else None

    report = check(
        git.changed_files(args.base),
        lambda p: git.exists_at(base, p),
        existing_dirs,
        read,
    )
    json.dump({"ok": report.ok, **asdict(report)}, sys.stdout, indent=2)
    sys.stdout.write("\n")
    return 0 if report.ok else 1


if __name__ == "__main__":
    sys.exit(main())
