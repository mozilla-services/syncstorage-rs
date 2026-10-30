"""Tests for devtools.draft_lint."""

import pytest

from devtools.draft_lint import lint

PR_OK = (
    "## Description\n\nFix it.\n\n## Testing\n\nRan it.\n\n## Issue(s)\n\nCloses #1.\n"
)
SREIN_OK = (
    "* Service: syncstorage-rs\n* Engineering Team: STOR\n"
    "* Details and context: x\n* Request: y\n* Urgency: Within a week\n"
)
WSI = "https://github.com/mozilla/webservices-infra/pull/13343"


def test_clean_pr_passes() -> None:
    """A PR draft with all template sections and plain text passes."""
    result = lint(PR_OK, "pr")
    assert result.ok
    assert result.warnings == []


def test_missing_pr_section_fails() -> None:
    """Each PR template section is required."""
    result = lint("## Description\n\nx\n", "pr")
    assert "missing section '## Testing'" in result.errors
    assert "missing section '## Issue(s)'" in result.errors


def test_srein_sections_required() -> None:
    """SREIN drafts need all five form fields."""
    assert lint(SREIN_OK, "srein").ok
    assert not lint("Service: x", "srein").ok


def test_non_ascii_reports_position() -> None:
    """Em-dashes and arrows are reported with line and column."""
    result = lint("ok\na \u2014 b \u2192 c", "comment")
    assert result.errors == [
        "line 2 col 3: non-ASCII EM DASH",
        "line 2 col 7: non-ASCII RIGHTWARDS ARROW",
    ]


@pytest.mark.parametrize(
    "secret",
    [
        "https://" + "a" * 32 + "@o1.ingest.sentry.io/123",
        "mysql://user:hunter2@db.internal/sync",
        "ghp_" + "A" * 36,
        "-----BEGIN RSA PRIVATE KEY-----",
    ],
)
def test_secrets_fail(secret: str) -> None:
    """Anything that looks like a credential is an error."""
    assert not lint(f"see {secret}", "comment").ok


def test_placeholder_url_is_fine() -> None:
    """URLs without credentials do not trip the password check."""
    assert lint("https://mozilla.sentry.io/issues/1/", "comment").ok


def test_jira_internal_link_needs_no_sync() -> None:
    """A Jira draft linking webservices-infra fails without no-sync."""
    result = lint(f"Fix in {WSI}", "jira")
    assert not result.ok
    assert result.internal_links == ["webservices-infra"]
    assert lint(f"Fix in {WSI}", "jira", labels=["no-sync"]).ok


def test_pr_internal_link_is_a_warning() -> None:
    """Internal links in a public PR warn but do not block."""
    result = lint(PR_OK + f"\nSee {WSI} and SREIN-1886\n", "pr")
    assert result.ok
    assert result.internal_links == ["webservices-infra", "SREIN"]
    assert any("public repo" in w for w in result.warnings)


def test_srein_internal_links_are_expected() -> None:
    """SREIN is internal, so links there raise nothing."""
    result = lint(SREIN_OK + f"\n{WSI}\n", "srein")
    assert result.ok
    assert result.warnings == []


def test_filler_words_warn() -> None:
    """AI-tell words are warnings, reported once each."""
    result = lint("A robust and Robust fix that leverages things", "comment")
    assert result.warnings == ["filler word 'leverages'", "filler word 'robust'"]


def test_unknown_kind_raises() -> None:
    """Unknown kinds are a programming error."""
    with pytest.raises(ValueError):
        lint("x", "email")
