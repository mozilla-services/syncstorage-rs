---
name: multi-backend-consistency
description: Checks that trait method changes in syncstorage-db or tokenserver-db are consistently implemented across all backend crates (mysql, postgres, spanner). Flags missing impls, behavioral drift, and Spanner-specific assumptions leaking into shared code.
user-invocable: true
---

# Multi-Backend Consistency Check

You are a Rust trait/implementation consistency reviewer for syncstorage-rs. Changes to shared traits must be reflected in every backend. Your job is to find gaps before they reach CI.

## Backend map

```
syncstorage-db/         ← trait definitions (DbPool, Db, etc.)
  → syncstorage-mysql/
  → syncstorage-postgres/
  → syncstorage-spanner/

tokenserver-db/         ← trait definitions
  → tokenserver-mysql/
  → tokenserver-postgres/
```

## Step 1 — Find changed trait files

```bash
git diff main...HEAD --name-only | grep -E "syncstorage-db/|tokenserver-db/"
```

If no trait files changed, check whether any backend implementation changed without a corresponding trait change — that may indicate the trait was bypassed or duplicated.

```bash
git diff main...HEAD --name-only | grep -E "syncstorage-(mysql|postgres|spanner)/|tokenserver-(mysql|postgres)/"
```

## Step 2 — Diff the traits

For each changed trait file:

```bash
git diff main...HEAD -- <file>
```

List every added, removed, or modified method signature.

## Step 3 — Check each backend for the same change

For each modified method, grep for its name in every backend crate:

```bash
grep -rn "fn <method_name>" syncstorage-mysql/src/ syncstorage-postgres/src/ syncstorage-spanner/src/
grep -rn "fn <method_name>" tokenserver-mysql/src/ tokenserver-postgres/src/
```

For each backend, determine:
- **Present and updated** — matches the new signature ✓
- **Present but stale** — exists but hasn't been updated to match ✗
- **Missing** — not implemented at all ✗

## Step 4 — Check for cross-backend behavioral assumptions

Review the diff for patterns that are safe on one backend but wrong on another:

- **Auto-increment IDs** — MySQL/Postgres use serial/auto-increment; Spanner uses explicit INT64. Code that relies on `last_insert_id()` or assumes monotonic IDs will break on Spanner.
- **Transaction semantics** — Spanner transactions are bounded; MySQL/Postgres allow longer-lived transactions. Check for assumptions about transaction scope.
- **NULL handling** — Spanner is stricter about NULLs in some column types than MySQL.
- **RETURNING clause** — Postgres supports `RETURNING id`; MySQL uses `last_insert_id()`; Spanner uses a separate read. Check that insert helpers follow the per-backend pattern established in `helpers.py` / existing db impls.
- **Timestamp precision** — Spanner uses microseconds; MySQL DATETIME has second precision unless explicitly specified.
- **Feature flags** — confirm the change compiles under `--features=syncstorage-db/mysql`, `--features=syncstorage-db/postgres`, and `--features=syncstorage-db/spanner` independently.

## Step 5 — Check shared db-common crates

Changes to `syncstorage-db-common/` or `tokenserver-db-common/` affect all backends. Verify nothing in those changes makes backend-specific assumptions.

```bash
git diff main...HEAD --name-only | grep "db-common"
```

## Output format

**Trait changes detected:**
List each modified method with its new signature.

**Backend coverage:**

| Method | mysql | postgres | spanner |
|---|---|---|---|
| `method_name` | ✓ updated / ✗ stale / ✗ missing | ... | ... |

**Behavioral drift warnings:**
Any cross-backend assumptions found, with file:line.

**Verdict:** All backends consistent / N gaps found
