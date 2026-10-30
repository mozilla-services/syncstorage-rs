"""Tests for devtools.models."""

import json
from typing import Any

import pytest
from pydantic import ValidationError

from devtools.draft_lint import lint
from devtools.models import JiraDraft, PrDraft, SreinRequest, main

STOR = "https://mozilla-hub.atlassian.net/browse/STOR-789"
WSI = "https://github.com/mozilla/webservices-infra/pull/1"
EE = {"summary": "x", "category": "Engineering Excellence", "background": "y"}


def _srein(**overrides: Any) -> dict[str, Any]:
    data: dict[str, Any] = {
        "summary": "Archive old tokenserver Sentry projects",
        "component": "Sentry projects",
        "requester": "Some Engineer",
        "details": "Tokenserver now reports to syncstorage-rs.",
        "tracked_in": [STOR],
        "request": "Archive, not delete, the two old projects.",
    }
    return data | overrides


def test_srein_renders_all_form_fields() -> None:
    """The rendered SREIN text passes the srein lint."""
    text = SreinRequest(**_srein(related=[WSI])).render()
    assert lint(text, "srein").ok
    assert "Urgency: Within a week" in text
    assert WSI in text


def test_srein_summary_length_is_capped() -> None:
    """Summaries over 80 characters are rejected."""
    with pytest.raises(ValidationError):
        SreinRequest(**_srein(summary="x" * 81))


def test_srein_needs_a_tracking_ticket() -> None:
    """At least one STOR link is required."""
    with pytest.raises(ValidationError):
        SreinRequest(**_srein(tracked_in=[]))


def test_jira_internal_link_needs_no_sync() -> None:
    """A Jira draft with an internal link only validates with no-sync."""
    base: dict[str, Any] = {
        "summary": "Pin health check",
        "category": "Operational Excellence",
        "background": "See below.",
        "links": [WSI],
    }
    with pytest.raises(ValidationError, match="no-sync"):
        JiraDraft.model_validate(base)
    assert JiraDraft.model_validate(base | {"labels": ["no-sync"]}).labels == [
        "no-sync"
    ]


def test_jira_renders_only_filled_sections() -> None:
    """Empty optional sections are left out."""
    text = JiraDraft(
        summary="x",
        category="Engineering Excellence",
        category_reason="tooling",
        background="Why.",
        acceptance_criteria=["It works"],
    ).render()
    assert text == (
        "Category: Engineering Excellence -- tooling\n\n"
        "## Background\n\nWhy.\n\n## Acceptance criteria\n\n- It works\n"
    )


def test_jira_requires_a_valid_category() -> None:
    """Category is required and limited to the three work categories."""
    with pytest.raises(ValidationError):
        JiraDraft.model_validate({"summary": "x", "background": "y"})
    with pytest.raises(ValidationError):
        JiraDraft.model_validate({**EE, "category": "KTLO"})


def test_jira_rejects_bad_priority_and_parent() -> None:
    """Only P0 to P4 and STOR keys are allowed."""
    with pytest.raises(ValidationError):
        JiraDraft.model_validate(
            {"summary": "x", "background": "y", "priority": "Major"}
        )
    with pytest.raises(ValidationError):
        JiraDraft.model_validate({**EE, "parent": "SREIN-1"})


def test_non_ascii_is_rejected() -> None:
    """Em-dashes never make it into a rendered draft."""
    with pytest.raises(ValidationError, match="EM DASH"):
        JiraDraft.model_validate({**EE, "background": "a \u2014 b"})


@pytest.mark.parametrize(
    "title",
    [
        "Fix: thing",
        "fix: Capital subject",
        "fix: trailing period.",
        "update: wrong type",
    ],
)
def test_pr_title_must_be_conventional(title: str) -> None:
    """PR titles follow the repo's commit subject rules."""
    with pytest.raises(ValidationError):
        PrDraft(title=title, description="d", testing="t", issues=["Closes #1"])


def test_pr_renders_template() -> None:
    """The PR body has the three template sections."""
    pr = PrDraft(
        title="chore(devtools): add models",
        description="Adds models.",
        testing="pytest",
        issues=["Closes STOR-789"],
    )
    assert pr.render().startswith("## Description\n\nAdds models.")
    assert lint(pr.render(), "pr").ok


def test_extra_fields_are_rejected() -> None:
    """Typos in field names fail instead of being dropped."""
    with pytest.raises(ValidationError):
        JiraDraft.model_validate({**EE, "lables": []})


def test_cli_render_and_errors(
    tmp_path: Any, capsys: pytest.CaptureFixture[str]
) -> None:
    """render prints text on success and JSON errors on failure."""
    good = tmp_path / "good.json"
    good.write_text(json.dumps(_srein()))
    assert main(["render", "--kind", "srein", str(good)]) == 0
    assert capsys.readouterr().out.startswith("* Service: syncstorage-rs")

    bad = tmp_path / "bad.json"
    bad.write_text(json.dumps(_srein(summary="x" * 81)))
    assert main(["render", "--kind", "srein", str(bad)]) == 1
    out = json.loads(capsys.readouterr().out)
    assert out["errors"][0]["field"] == "summary"


def test_cli_schema(capsys: pytest.CaptureFixture[str]) -> None:
    """schema prints the model's JSON schema."""
    assert main(["schema", "--kind", "pr"]) == 0
    assert "title" in json.loads(capsys.readouterr().out)["properties"]
