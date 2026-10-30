"""pydantic models for the drafts the skills produce.

A skill fills one of these as JSON, then ``render`` validates it and prints
the final text. Validation runs ``draft_lint`` on the rendered text, so a
draft with non-ASCII characters, secrets, or internal links on a Jira ticket
without ``no-sync`` never renders. ``schema`` prints the JSON schema, which is
the shape to ask an LLM for.
"""

import argparse
import json
import sys
from pathlib import Path
from typing import ClassVar, Literal, Self

from pydantic import BaseModel, ConfigDict, Field, ValidationError, model_validator

from devtools.draft_lint import lint

Priority = Literal["P0", "P1", "P2", "P3", "P4"]
STOR_KEY = r"^STOR-\d+$"
COMMIT_TYPES = "feat|fix|docs|style|refactor|perf|test|chore"
REPO_URL = "https://github.com/mozilla-services/syncstorage-rs"


def _bullets(items: list[str]) -> str:
    """Render ``items`` as a markdown bullet list."""
    return "\n".join(f"- {item}" for item in items)


class _Draft(BaseModel):
    """Shared config and lint hook for all drafts."""

    model_config = ConfigDict(extra="forbid", str_strip_whitespace=True)
    kind: ClassVar[str] = ""

    def labels_for_lint(self) -> list[str]:
        """Return the Jira labels the lint should consider."""
        return []

    def render(self) -> str:
        """Return the final text."""
        raise NotImplementedError

    @model_validator(mode="after")
    def _lint(self) -> Self:
        result = lint(self.render(), self.kind, self.labels_for_lint())
        if result.errors:
            raise ValueError("; ".join(result.errors))
        return self


class SreinRequest(_Draft):
    """An SRE intake request in the SREIN form's shape."""

    kind = "srein"

    summary: str = Field(max_length=80)
    component: str = Field(description="what part of the service, e.g. Sentry")
    requester: str
    details: str = Field(description="2 to 5 sentences of context")
    tracked_in: list[str] = Field(min_length=1, description="STOR ticket/epic URLs")
    related: list[str] = Field(default_factory=list, description="PR, doc URLs")
    request: str = Field(description="the exact action, including what not to do")
    urgency: str = "Within a week"
    priority: Priority = "P1"

    def render(self) -> str:
        """Render the SREIN description."""
        context = f"{self.details}\n  Tracked in {', '.join(self.tracked_in)}."
        if self.related:
            context += f" Related: {', '.join(self.related)}"
        return (
            f"* Service: syncstorage-rs ({REPO_URL}) : {self.component}\n"
            f"* Engineering Team: STOR - syncstorage-rs ({self.requester})\n"
            f"* Details and context: {context}\n"
            f"* Request: {self.request}\n"
            f"* Urgency: {self.urgency}\n"
        )


class JiraDraft(_Draft):
    """A STOR ticket. Only background is required; keep tickets light."""

    kind = "jira"

    summary: str = Field(max_length=120)
    issue_type: Literal["Task", "Bug", "Story", "Spike"] = "Task"
    priority: Priority = "P3"
    labels: list[str] = Field(default_factory=list)
    parent: str | None = Field(default=None, pattern=STOR_KEY)
    background: str
    acceptance_criteria: list[str] = Field(default_factory=list)
    implementation_steps: list[str] = Field(default_factory=list)
    out_of_scope: list[str] = Field(default_factory=list)
    links: list[str] = Field(default_factory=list)

    def labels_for_lint(self) -> list[str]:
        """Return the ticket's labels so the no-sync rule can see them."""
        return self.labels

    def render(self) -> str:
        """Render the Jira description (markdown)."""
        parts = [f"## Background\n\n{self.background}"]
        sections = (
            ("Acceptance criteria", self.acceptance_criteria),
            ("Implementation steps", self.implementation_steps),
            ("Out of scope", self.out_of_scope),
            ("Links", self.links),
        )
        parts += [
            f"## {title}\n\n{_bullets(items)}" for title, items in sections if items
        ]
        return "\n\n".join(parts) + "\n"


class PrDraft(_Draft):
    """A syncstorage-rs PR in the repo template's shape."""

    kind = "pr"

    title: str = Field(
        max_length=72,
        pattern=rf"^({COMMIT_TYPES})(\([a-z0-9-]+\))?: [a-z0-9`].*[^.]$",
    )
    description: str
    testing: str
    issues: list[str] = Field(min_length=1, description="e.g. 'Closes STOR-789'")

    def render(self) -> str:
        """Render the PR body."""
        return (
            f"## Description\n\n{self.description}\n\n"
            f"## Testing\n\n{self.testing}\n\n"
            f"## Issue(s)\n\n{chr(10).join(self.issues)}\n"
        )


MODELS: dict[str, type[_Draft]] = {
    "srein": SreinRequest,
    "jira": JiraDraft,
    "pr": PrDraft,
}


def main(argv: list[str] | None = None) -> int:
    """Validate and render a draft, or print a model's JSON schema."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("render", "schema"))
    parser.add_argument("--kind", choices=sorted(MODELS), required=True)
    parser.add_argument("file", nargs="?", help="draft JSON; defaults to stdin")
    args = parser.parse_args(argv)
    model = MODELS[args.kind]

    if args.command == "schema":
        json.dump(model.model_json_schema(), sys.stdout, indent=2)
        sys.stdout.write("\n")
        return 0

    raw = Path(args.file).read_text() if args.file else sys.stdin.read()
    try:
        draft = model.model_validate_json(raw)
    except ValidationError as exc:
        errors = [
            {"field": ".".join(map(str, e["loc"])), "error": e["msg"]}
            for e in exc.errors()
        ]
        json.dump({"ok": False, "errors": errors}, sys.stdout, indent=2)
        sys.stdout.write("\n")
        return 1
    sys.stdout.write(draft.render())
    return 0


if __name__ == "__main__":
    sys.exit(main())
