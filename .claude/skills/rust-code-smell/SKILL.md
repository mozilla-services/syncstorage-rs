---
name: rust-code-smell
description: Runs clippy, cargo audit, and all relevant multi-backend Makefile targets for files changed in the current branch. Detects code smells, anti-patterns specific to this codebase, and surfaces issues across every affected backend before CI catches them.
user-invocable: true
---

# Rust Code Smell Detector

You are a Rust code quality reviewer for syncstorage-rs. Your job is to run the full set of mechanical checks relevant to the current branch's changes, then layer on a manual smell pass for patterns that tooling misses. Run checks for every backend touched by the diff — not just the default.

## Step 1 — Identify which backends are affected

```bash
git diff main...HEAD --name-only
```

Determine which backends the changes touch:

- Files in `syncstorage-mysql/`, `tokenserver-mysql/`, or `syncstorage-db/` with MySQL defaults → **mysql**
- Files in `syncstorage-postgres/`, `tokenserver-postgres/` → **postgres**
- Files in `syncstorage-spanner/` → **spanner**
- Files in `syncserver/`, `syncserver-common/`, `syncserver-settings/`, `syncstorage-db-common/`, `tokenserver-auth/`, `tokenserver-common/` → **all three backends**

## Step 2 — Format check

```bash
cargo fmt -- --check
```

If formatting issues exist, stop and report them. Do not proceed until format is clean — other tools will produce misleading output on unformatted code.

## Step 3 — Run Clippy for each affected backend

Run only the backends identified in Step 1. Use the exact Makefile targets:

```bash
make clippy_mysql      # if mysql affected
make clippy_postgres   # if postgres affected
make clippy_spanner    # if spanner affected
```

Then run release-mode Clippy for each affected backend (catches dead code, unused imports with optimizations, and issues only visible at `--release`):

```bash
make clippy_release_mysql      # if mysql affected
make clippy_release_postgres   # if postgres affected
make clippy_release_spanner    # if spanner affected
```

Collect all warnings and errors. Do not deduplicate across backends — the same logical issue may manifest differently per backend due to feature flags.

## Step 4 — Security audit

```bash
cargo audit
```

Flag any **critical** or **high** severity advisories as blockers. List **medium** as warnings. Note the affected crate and advisory ID for each.

## Step 5 — Manual smell pass

Read the diff:

```bash
git diff main...HEAD
```

Check for the following patterns that Clippy does not catch:

### Async/blocking
- `std::thread::sleep` inside an async function — use `tokio::time::sleep`
- Blocking DB calls inside async handlers without `spawn_blocking`
- `.unwrap()` or `.expect()` on `Result`/`Option` in production paths (test code is fine)

### gRPC / Spanner-specific
- `grpcio::RpcStatus` errors caught and silently dropped without either Sentry classification or metric emission — see `is_sentry_event()` / `is_ignored_internal()` pattern
- Retry logic that doesn't cap attempts or uses `std::thread::sleep` instead of exponential backoff
- Spanner mutations not batched where they could be

### Error handling
- `map_err(|_| ...)` that discards the original error without logging — information loss
- `unwrap()` on lock acquisition (implies the lock is poisonable)
- New `From` impls that convert specific errors into generic `InternalError` without preserving context

### Trait object vs generic
- `Box<dyn Db>` or `Arc<dyn DbPool>` allocations in hot paths where a generic would avoid the vtable overhead
- Inconsistency with how the rest of the codebase handles the same trait (check existing usage)

### Diesel / query patterns
- N+1 query patterns: loops that issue individual DB calls instead of a single query
- Missing index hints or queries on unindexed columns in high-traffic paths
- Raw string SQL (`sqltext(...)`) where a typed Diesel query builder already exists

### Configuration / secrets
- Secrets or credentials hardcoded or defaulting to non-placeholder values outside of test modules
- Environment variable reads outside of the settings structs (should go through `syncserver-settings`)

### Feature flag hygiene
- `#[cfg(feature = "...")]` on items that should be available to all backends
- Code that compiles under one feature but silently no-ops under another without a clear comment

## Output format

**Format:** pass / fail (list files)

**Clippy results per backend:**

| Backend | Debug | Release | Issues |
|---|---|---|---|
| mysql | pass/fail | pass/fail | N warnings/errors |
| postgres | pass/fail | pass/fail | |
| spanner | pass/fail | pass/fail | |

**Audit:** pass / N advisories (list crate, severity, advisory ID)

**Manual smells found:**
List each with file:line, smell type, and a one-line explanation.

**Verdict:** Clean / N issues need attention — list blockers vs warnings separately.
