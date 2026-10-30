"""Lint a Jira, PR, SREIN or comment draft before it is posted.

Checks for non-ASCII characters, anything that looks like a secret, the
sections each kind of draft needs, filler words, and internal links. STOR
tickets sync to public GitHub unless labeled ``no-sync`` at creation, so a
Jira draft with internal links fails unless ``--label no-sync`` is passed.
Prints JSON and exits 1 when there are errors.
"""

import argparse
import json
import re
import sys
import unicodedata
from dataclasses import asdict, dataclass, field
from pathlib import Path

KINDS = ("jira", "pr", "srein", "comment")

REQUIRED_SECTIONS = {
    "pr": ("## Description", "## Testing", "## Issue(s)"),
    "srein": (
        "Service:",
        "Engineering Team:",
        "Details and context:",
        "Request:",
        "Urgency:",
    ),
}

SECRET_PATTERNS = {
    "Sentry DSN": re.compile(r"https://[0-9a-f]{32}@[\w.-]*sentry\.io"),
    "Sentry auth token": re.compile(r"\bsntry[su]_[A-Za-z0-9_=+/-]{20,}"),
    "GitHub token": re.compile(r"\bgh[pousr]_[A-Za-z0-9]{30,}"),
    "AWS access key": re.compile(r"\bAKIA[0-9A-Z]{16}\b"),
    "Google API key": re.compile(r"\bAIza[0-9A-Za-z_-]{35}\b"),
    "Slack token": re.compile(r"\bxox[abpr]-[A-Za-z0-9-]{10,}"),
    "private key": re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----"),
    "URL with password": re.compile(r"\b[a-z][a-z0-9+.-]*://[^/\s:@]+:[^/\s@]+@"),
}

INTERNAL_LINKS = {
    "webservices-infra": re.compile(r"github\.com/mozilla/webservices-infra\b"),
    "global-platform-admin": re.compile(r"\bglobal-platform-admin\b"),
    "cloudops-infra": re.compile(r"\bcloudops-infra\b"),
    "Confluence": re.compile(r"mozilla-hub\.atlassian\.net/wiki\b"),
    "SREIN": re.compile(r"\bSREIN-\d+\b"),
    "Argo CD / internal GCP": re.compile(r"\b[\w.-]+\.mozgcp\.net\b"),
    "mozilla.cloud": re.compile(r"\b[\w.-]+\.mozilla\.cloud\b"),
}

FILLER_WORDS = re.compile(
    r"\b(genuinely|comprehensive|robust|seamless(ly)?|leverag(e|es|ed|ing)|delv(e|es|ing))\b",
    re.IGNORECASE,
)


@dataclass
class Result:
    """Lint findings for one draft."""

    errors: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)
    internal_links: list[str] = field(default_factory=list)

    @property
    def ok(self) -> bool:
        """Return whether the draft has no errors."""
        return not self.errors


def _non_ascii(text: str) -> list[str]:
    """Return a finding for each non-ASCII character, with its position."""
    found = []
    for lineno, line in enumerate(text.splitlines(), 1):
        for col, ch in enumerate(line, 1):
            if ord(ch) > 127:
                name = unicodedata.name(ch, f"U+{ord(ch):04X}")
                found.append(f"line {lineno} col {col}: non-ASCII {name}")
    return found


def lint(text: str, kind: str, labels: list[str] | None = None) -> Result:
    """Lint ``text`` as a draft of ``kind`` with the given Jira labels."""
    if kind not in KINDS:
        raise ValueError(f"unknown kind {kind!r}")
    result = Result()
    result.errors.extend(_non_ascii(text))

    for name, pattern in SECRET_PATTERNS.items():
        if pattern.search(text):
            result.errors.append(f"looks like a {name}; use a placeholder")

    for section in REQUIRED_SECTIONS.get(kind, ()):
        if section not in text:
            result.errors.append(f"missing section {section!r}")

    for word in sorted({m.group(0).lower() for m in FILLER_WORDS.finditer(text)}):
        result.warnings.append(f"filler word {word!r}")

    result.internal_links = [
        name for name, pattern in INTERNAL_LINKS.items() if pattern.search(text)
    ]
    if result.internal_links:
        where = ", ".join(result.internal_links)
        if kind == "jira" and "no-sync" not in (labels or []):
            result.errors.append(
                f"internal links ({where}) need the no-sync label at create time"
            )
        elif kind in ("pr", "comment"):
            result.warnings.append(
                f"internal links ({where}) in a public repo; keep detail out or "
                "move it to a no-sync ticket"
            )
    return result


def main(argv: list[str] | None = None) -> int:
    """Lint a draft file (or stdin) and print JSON."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--kind", choices=KINDS, required=True)
    parser.add_argument(
        "--label", action="append", default=[], help="Jira label (repeatable)"
    )
    parser.add_argument("file", nargs="?", help="draft file; defaults to stdin")
    args = parser.parse_args(argv)
    text = Path(args.file).read_text() if args.file else sys.stdin.read()
    result = lint(text, args.kind, args.label)
    json.dump({"ok": result.ok, **asdict(result)}, sys.stdout, indent=2)
    sys.stdout.write("\n")
    return 0 if result.ok else 1


if __name__ == "__main__":
    sys.exit(main())
