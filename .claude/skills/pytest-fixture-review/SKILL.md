---
name: pytest-fixture-review
description: Reviews conftest.py and helpers.py files in tools/integration_tests/ against the repo's fixture/helper separation standard. Flags helpers in conftest, missing teardown, wrong scopes, and raw DB operations outside helper functions.
user-invocable: true
---

# Pytest Fixture Review

You are a pytest code reviewer for syncstorage-rs integration tests. This repo has a deliberate structure: `conftest.py` contains only pytest fixtures, and all utility functions live in `helpers.py`. Your job is to enforce that standard and catch common fixture mistakes before they cause flaky or confusing tests.

## The standard

```
tools/integration_tests/
  conftest.py          ← fixtures ONLY (st_ctx)
  helpers.py           ← retry helpers, auth state, switch_user, constants

tools/integration_tests/tokenserver/
  conftest.py          ← fixtures ONLY (ts_db_conn, ts_app, ts_service_id, ts_ctx, fxa_auth)
  helpers.py           ← DB helpers, auth helpers, node/user/service helpers
```

Fixtures yield context dicts. Helpers are plain functions imported directly by test files.

## Step 1 — Find changed test files

```bash
git diff main...HEAD --name-only | grep -E "tools/integration_tests"
```

Read each changed file in full.

## Step 2 — conftest.py checks

For each `conftest.py` in the diff:

**Non-fixture code in conftest:**
- Are there any module-level functions that are not decorated with `@pytest.fixture`?
- Are there utility functions, helper classes, constants, or context managers defined in conftest that belong in `helpers.py`?
- Exception: module-level side effects that must run at import time (e.g. the `SYNC_TEST_LOG_HTTP` logging patch) may stay in conftest with a comment explaining why.

**Fixture scopes:**
- `scope="function"` — default, correct for most fixtures; state is reset per test
- `scope="session"` — only justified for expensive external calls (e.g. `fxa_auth` creates a real FxA account). Any new session-scoped fixture must have a comment explaining why.
- `scope="module"` or `scope="class"` — flag for review; not currently used in this repo

**Teardown:**
- Does every fixture that creates state also clean it up?
- `ts_ctx` pattern: `clear_db()` before yield AND after yield. Verify both sides are present.
- `fxa_auth` pattern: `acct.clear()` + `client.destroy_account()` in teardown. Verify error handling wraps the teardown.
- `st_ctx` pattern: `config.end()` + `del os.environ["MOZSVC_UUID"]`. Verify both.

**Database connections:**
- `ts_db_conn` uses `AUTOCOMMIT` isolation. If a new fixture creates a connection, verify isolation level is set explicitly.
- Connections must be closed in teardown (`conn.close()`).

## Step 3 — helpers.py checks

**No fixtures in helpers:**
- Are there any functions decorated with `@pytest.fixture` in `helpers.py`? These must move to `conftest.py`.

**Raw SQL outside helper functions:**
- Is `sqltext()` / `execute_sql()` called directly in a test file rather than through a helper?
- Direct SQL in tests bypasses the helper layer and makes tests brittle. Flag with file:line.

**Imports:**
- Are helpers importing from `conftest`? This creates a circular dependency risk. Helpers should have no pytest imports and no conftest imports.

**Constants:**
- Are test constants (NODE_ID, NODE_URL, FXA_EMAIL_DOMAIN, etc.) defined in helpers, not scattered in test files or conftest?

## Step 4 — Test file import checks

For each changed test file:

```bash
grep -n "from.*conftest import" <test_file>
```

Test files should import helpers from `helpers.py`, not from `conftest.py`. Any `from tools.integration_tests.conftest import` or `from integration_tests.tokenserver.conftest import` is wrong — flag it with the correct import path.

## Step 5 — New fixture pattern check

If a new fixture was added, verify:
- It follows the `yield`-based teardown pattern (not `return`)
- It is typed or documented enough that its contents are clear
- If it seeds database state, it cleans up after itself
- Its scope is the narrowest possible for its purpose

## Output format

**conftest.py issues:**
List each violation with file:line and what needs to move or change.

**helpers.py issues:**
List each violation with file:line.

**Import violations:**
List any test files importing from conftest instead of helpers.

**New fixtures:**
For each new fixture: name, scope, teardown present (yes/no), any concerns.

**Verdict:** Follows conventions / N issues found
