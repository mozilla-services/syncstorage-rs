"""Tests for devtools.migration_check."""

from devtools.migration_check import Report, check

CRATE = "syncstorage-mysql"
OLD = "2020-08-24-091401_add_quota"
NEW = "2026-11-01-120000_add_thing"


def _run(
    changed: list[str],
    files: dict[str, str],
    published: set[str] | None = None,
) -> Report:
    return check(
        changed,
        lambda p: p in (published or set()),
        lambda crate: [OLD],
        files.get,
    )


def test_editing_published_migration_is_a_violation() -> None:
    """Changing a migration that exists on the base branch is blocked."""
    path = f"{CRATE}/migrations/{OLD}/up.sql"
    report = _run([path], {}, published={path})
    assert not report.ok
    assert "published migration edited" in report.violations[0]


def test_editing_schema_ddl_is_a_violation() -> None:
    """schema.ddl counts as published."""
    report = _run(["syncstorage-spanner/src/schema.ddl"], {})
    assert report.violations == [
        "syncstorage-spanner/src/schema.ddl: published schema edited"
    ]


def test_clean_new_migration_passes() -> None:
    """A new dir with real up and down SQL and a newer timestamp passes."""
    base = f"{CRATE}/migrations/{NEW}"
    files = {
        f"{base}/up.sql": "ALTER TABLE bso ADD COLUMN x INT;",
        f"{base}/down.sql": "ALTER TABLE bso DROP COLUMN x;",
    }
    report = _run(list(files), files)
    assert report.ok
    assert report.new_migrations == [base]
    assert report.warnings == []


def test_comment_only_down_sql_is_a_violation() -> None:
    """A down.sql with only comments is not a rollback."""
    base = f"{CRATE}/migrations/{NEW}"
    files = {
        f"{base}/up.sql": "CREATE TABLE t (id INT);",
        f"{base}/down.sql": "-- nothing to do\n",
    }
    report = _run(list(files), files)
    assert report.violations == [f"{base}: missing or empty down.sql"]


def test_missing_down_sql_is_a_violation() -> None:
    """A new migration without down.sql is blocked."""
    base = f"{CRATE}/migrations/{NEW}"
    files = {f"{base}/up.sql": "CREATE TABLE t (id INT);"}
    report = _run(list(files), files)
    assert report.violations == [f"{base}: missing or empty down.sql"]


def test_old_timestamp_is_a_violation() -> None:
    """New migrations must sort after every existing one."""
    base = f"{CRATE}/migrations/2019-01-01-000000_late"
    files = {f"{base}/up.sql": "SELECT 1;", f"{base}/down.sql": "SELECT 1;"}
    report = _run(list(files), files)
    assert any("timestamp not newer" in v for v in report.violations)


def test_destructive_sql_is_a_warning() -> None:
    """DROP TABLE in up.sql is flagged but not blocked."""
    base = f"{CRATE}/migrations/{NEW}"
    files = {
        f"{base}/up.sql": "DROP TABLE old_stuff;",
        f"{base}/down.sql": "CREATE TABLE old_stuff (id INT);",
    }
    report = _run(list(files), files)
    assert report.ok
    assert report.warnings == [f"{base}/up.sql: destructive op DROP TABLE"]


def test_inline_sql_is_a_warning() -> None:
    """Non-migration SQL files get a review warning."""
    path = "syncstorage-spanner/src/insert_standard_collections.sql"
    report = _run([path], {})
    assert report.ok
    assert report.warnings == [f"{path}: non-migration SQL changed"]
