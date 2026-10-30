"""Tests for devtools.changed_scope."""

from devtools.changed_scope import classify


def test_backend_crate_maps_to_one_backend() -> None:
    """A change inside a backend crate only needs that backend's clippy."""
    out = classify(["syncstorage-spanner/src/db.rs"])
    assert out["backends"] == ["spanner"]
    assert out["clippy_targets"] == ["clippy_spanner"]


def test_tokenserver_backends_map_like_syncstorage() -> None:
    """Tokenserver backend crates map to the same backend names."""
    out = classify(["tokenserver-mysql/src/lib.rs", "tokenserver-postgres/src/a.rs"])
    assert out["backends"] == ["mysql", "postgres"]


def test_shared_crate_hits_all_backends() -> None:
    """Shared crates and Cargo files affect every backend."""
    for path in ["syncserver/src/web/handlers.rs", "Cargo.lock"]:
        assert classify([path])["backends"] == ["mysql", "postgres", "spanner"]


def test_non_rust_changes_need_no_clippy() -> None:
    """Docs, skills and Python changes do not trigger clippy."""
    out = classify(["docs/src/introduction.md", ".claude/skills/x/SKILL.md"])
    assert out["backends"] == []
    assert out["skills"] == []


def test_trait_and_error_files_suggest_review_skills() -> None:
    """Trait and error modules pull in their review skills."""
    out = classify(
        ["syncstorage-db-common/src/lib.rs", "syncstorage-db-common/src/error.rs"]
    )
    assert out["trait_files"] == [
        "syncstorage-db-common/src/lib.rs",
        "syncstorage-db-common/src/error.rs",
    ]
    assert out["error_files"] == ["syncstorage-db-common/src/error.rs"]
    assert "multi-backend-consistency" in out["skills"]
    assert "rust-error-review" in out["skills"]


def test_migrations_and_schema_are_flagged() -> None:
    """Migration dirs and schema.ddl are both picked up."""
    out = classify(
        [
            "syncstorage-mysql/migrations/2026-01-01-000000_x/up.sql",
            "syncstorage-spanner/src/schema.ddl",
        ]
    )
    assert len(out["migrations"]) == 2
    assert "migration-review" in out["skills"]


def test_python_under_tools_is_not_rust() -> None:
    """Python in tools/ suggests py-checks and fixture review, not clippy."""
    out = classify(["tools/integration_tests/conftest.py"])
    assert out["backends"] == []
    assert out["skills"] == ["py-checks", "pytest-fixture-review"]
