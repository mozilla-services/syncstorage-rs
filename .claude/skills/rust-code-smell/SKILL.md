---
name: rust-code-smell
description: Runs clippy, cargo audit, and all relevant multi-backend Makefile targets for files changed in the current branch. Detects code smells, anti-patterns specific to this codebase, and surfaces issues across every affected backend before CI catches them. Also handles dependency-update verification (Cargo.toml / Cargo.lock changes from dependabot or manual bumps) via the `deps` mode.
user-invocable: true
---

# Rust Code Smell Detector

You are a Rust code quality reviewer for syncstorage-rs. Your job is to run the full set of mechanical checks relevant to the current branch's changes, then layer on a manual smell pass for patterns that tooling misses. Run checks for every backend touched by the diff — not just the default.

## Standing authorization

When this skill is invoked, you have standing authorization to run the following without per-command confirmation:

- `cargo fmt -- --check`
- `cargo clippy ...` (any feature combination)
- `make clippy_mysql`, `make clippy_postgres`, `make clippy_spanner`
- `make clippy_release_mysql`, `make clippy_release_postgres`, `make clippy_release_spanner`
- `cargo audit`
- `cargo check --workspace ...`
- `cargo test --no-run` (compile-check only — do NOT run actual tests without explicit permission, since they require DB setup)
- `cargo update --precise <ver> -p <crate>` (lockfile-only adjustments during bisection)
- `git stash` / `git stash pop` (for baseline comparison)

Do not run anything destructive (commit, push, rebase, branch deletion, `cargo clean` of the whole workspace, `make run_*`, docker-compose up/down) without asking.

## Mode selection

- **Default mode** — branch under review contains Rust source changes (not just `Cargo.toml`/`Cargo.lock`). Run Steps 1–5 below.
- **`deps` mode** — branch only changes `Cargo.toml` / `Cargo.lock`, or the user invokes the skill in the context of a dependabot PR / dependency bump. Jump to the "Dependency update verification" section near the end and follow that flow instead.

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

---

# Dependency update verification (`deps` mode)

Use this flow when the branch only changes `Cargo.toml` / `Cargo.lock`, the user is evaluating a dependabot PR, or the user is splitting a grouped dependency PR into smaller pieces.

## When to enter this mode

- User invokes the skill on a dependabot PR (e.g. mozilla-services/syncstorage-rs#NNNN).
- `git diff main...HEAD --name-only` returns only `Cargo.toml` and/or `Cargo.lock`.
- User explicitly says "check this dep bump" / "verify the cargo updates" / "is this PR safe".

## Step D1 — Establish the baseline

The single most valuable signal is whether the gates already pass on `main` before the bump. Stash, test, restore:

```bash
git stash
make clippy_mysql 2>&1 | tail -10
git stash pop
```

If `main` already fails a gate, stop and surface that to the user — the bump did not introduce the failure and pretending otherwise wastes time.

## Step D2 — Classify the bumps

Categorize every changed crate before running anything else. Read the manifest diff plus dependabot's release notes (in the PR body) and assign each crate to one of:

- **Routine** — patch/minor bumps to leaf-ish deps (`env_logger`, `reqwest`, `utoipa`, `uuid`, `cadence`, `jsonwebtoken`, `config`, `pyo3` patches). Group these.
- **Ecosystem-coupled** — crates whose versions must move together. In this repo:
  - `diesel-async` and `deadpool` are paired — Cargo.toml comments already say so.
  - `diesel_migrations` patch bumps can interact with `diesel-async`'s major version (a 2.3.2 bump broke `IntoUpdateTarget` trait selection against diesel-async 0.7; the same bump composes fine with diesel-async 0.9). Treat `diesel_migrations` as ecosystem-coupled even though it looks like a patch.
  - `protobuf` for the Spanner backend must match what `google-cloud-rust-raw` resolves. Do not bump `syncstorage-spanner`'s pinned `protobuf` ahead of `google-cloud-rust-raw`.
- **Major / breaking** — any `x.0 → y.0` or `0.x → 0.y` bump on a direct dep. Flag for human review even if compilation passes.

Recommend splitting a grouped dependabot PR along these lines: one PR per category, with the riskier categories held until their constraints clear.

## Step D3 — Apply bumps surgically

When applying a chosen group, use `--precise` per crate so transitive deps don't drift:

```bash
cargo update --precise <version> -p <crate>
```

Do not run a bare `cargo update -p <crate>` — that will pull in unrelated transitive bumps (e.g. `hashlink 0.10 → 0.11`, `anstream 0.6 → 1.0`) and you'll then have to debug whether those broke something. `--precise` keeps the diff to exactly what was requested.

Cargo only accepts one `--precise` per invocation. To apply N crates, run N sequential `cargo update --precise` commands.

If two crates share a version qualifier (`reqwest@0.13.2`), cargo will demand the disambiguator — use the form it suggests in the error.

After every cargo-update batch, run:

```bash
git diff origin/master -- Cargo.lock | grep -E '^[-+]version = ' | sort | uniq -c | sort -rn
```

Verify the version-change set matches the intended group. Anything you did not request showing up is a red flag.

## Step D4 — Run the gates

Run, in order, stopping at the first failure:

```bash
cargo fmt -- --check
cargo audit
make clippy_mysql
make clippy_postgres
make clippy_spanner
cargo test --workspace --no-run    # compile-check only
```

`make test` (running the suite) requires a real MySQL on `localhost:3306` with `sample_user:sample_password` and the schemas pre-created. If the user's environment doesn't have it, report that explicitly and recommend `make docker_run_mysql_e2e_tests` / `make docker_run_postgres_e2e_tests` for full verification. Do not attempt to set up the DB.

`cargo test --no-run` also requires `libpython3.9` linkable on disk because `pyo3` is a default-feature dep. If that fails at the link stage with `library 'python3.9' not found`, report it as a local env issue, not a bump regression. Confirm by running the same `--no-run` against the baseline (Step D1) — if it fails there too, it's not your problem.

## Step D5 — Bisect on failure

If a gate fails, do not start guessing. Bisect:

1. Revert the *most likely* single crate to its pre-bump version with `cargo update --precise <old-ver> -p <crate>`.
2. Re-run only the failing gate. If it now passes, that crate is the culprit. If not, restore it and try the next candidate.
3. Likely-culprit ordering for diesel/diesel-async-related failures: `diesel_migrations` first, then `uuid` (it's a diesel feature type), then anything else that touches DB.
4. For trait-selection / type-inference errors in `syncstorage-postgres` or `syncstorage-mysql` under a *different* backend's clippy target (e.g. clippy_mysql failing in syncstorage-postgres source), the failure is almost always an ecosystem-coupled crate moving alone. Re-classify and move it to the appropriate group.

When a crate is identified as incompatible with the current group, remove it from the group, return it to the manifest's previous version, and update the Jira ticket (or PR description) to note it's been moved to whichever group it now belongs with.

## Step D6 — Report

Output one table:

| Crate | Old | New | Status |
|---|---|---|---|
| ... | ... | ... | applied / held / failed / moved to Group X |

Plus the verification matrix:

| Gate | Result | Notes |
|---|---|---|
| `cargo fmt --check` | pass/fail | |
| `cargo audit` | pass/fail | N advisories listed if any |
| `make clippy_mysql` | pass/fail | |
| `make clippy_postgres` | pass/fail | |
| `make clippy_spanner` | pass/fail | |
| `cargo test --no-run` | pass/fail/env-skip | note libpython if env-skipped |

End with one paragraph: what landed, what was excluded and why, and what should ride with the next group.

## Known coupling notes for this repo

Keep these in mind when classifying bumps — they cost real time to rediscover:

- **`diesel-async` + `deadpool`** are paired. Cargo.toml comments say so. Do not move one without the other.
- **`diesel_migrations` 2.3.2** breaks `IntoUpdateTarget` trait selection in `syncstorage-postgres` source when paired with `diesel-async 0.7`. It needs to ride with the `diesel-async 0.9` upgrade.
- **`protobuf`** is pinned with `=<version>` in `syncstorage-spanner/Cargo.toml` deliberately. The pin must match what `google-cloud-rust-raw` resolves to. Currently both should be `2.28.0`. Bumping the spanner pin ahead of `google-cloud-rust-raw` will cause two protobuf majors in the lockfile and break compilation (RepeatedField doesn't exist in protobuf 3.x).
- **`make clippy_mysql` / `clippy_postgres` / `clippy_spanner`** all use `--workspace --all-targets` — every backend crate compiles under every feature set. A failure in `syncstorage-postgres` source under `clippy_mysql` is not a misconfiguration; it means a workspace-shared dep changed something diesel-postgres can't tolerate.
- **`pyo3` + `libpython3.9` link** is required for default-feature builds on this repo. Local `cargo test --no-run` will fail at link without it. Docker e2e is the fallback for full test verification.
