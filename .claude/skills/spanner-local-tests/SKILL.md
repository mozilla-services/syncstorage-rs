---
name: spanner-local-tests
description: Brings up the Spanner emulator, provisions the test database with the project schema, and runs the syncstorage-rs Spanner unit-test suite locally. Repeatable single-command flow plus full from-scratch documentation. Also covers the e2e (docker-compose) path.
user-invocable: true
---

# Run Spanner Tests Locally

Set up a local Spanner emulator + schema, then run the spanner-backed unit tests. The whole loop is < 90 seconds once the emulator image is cached. This is the **fast inner loop** for any change touching `syncstorage-spanner/`.

There are two test surfaces in this repo:

| Surface | What it covers | Time | Needs |
|---|---|---|---|
| **Unit tests** (this skill's default) | Direct calls into `SpannerDb` — exercises every DML path through `update_user_collection_quotas`, batch ops, BSO CRUD, tombstones, quota. | ~30s after warm-up | emulator only |
| **Docker e2e** | Full syncserver + emulator + tokenserver + mock FxA, hit over HTTP by the Python integration suite. | 5-15 min | freshly-built `app:build` image |

Default to unit tests. Reach for e2e only when changes touch HTTP routing, token handling, or cross-backend behavior CI would catch first.

## Quick re-run (after first-time setup)

If the emulator is already up and provisioned:

```bash
SYNC_SYNCSTORAGE__SPANNER_EMULATOR_HOST=localhost:9010 \
SYNC_SYNCSTORAGE__DATABASE_URL=spanner://projects/test-project/instances/test-instance/databases/test-database \
RUST_TEST_THREADS=1 \
cargo test --no-default-features --features=syncstorage-db/spanner --package syncstorage-db
```

If the emulator is **not** running, use the helper:

```bash
./scripts/spanner-local-test-setup.sh
```

That script brings up the emulator, provisions the database/schema, and runs the suite. It's idempotent — re-running just executes the tests if the stack is already up.

## From scratch — the full flow

### 1. Prerequisites

- **Docker Desktop running.** The emulator runs as a container.
- **Rust toolchain** matching `rust-toolchain.toml` (or close).
- For e2e only: enough Docker memory (≥ 8 GB recommended) — the `app:build` image compiles grpcio-sys from source and OOMs on small allocations.

### 2. Start the emulator

```bash
docker compose -f docker/docker-compose.spanner.yaml up -d sync-db
```

This starts the [`gcr.io/cloud-spanner-emulator/emulator`](https://cloud.google.com/spanner/docs/emulator) image. Two ports:

- **9010** — gRPC. The Rust client (and production) talk this.
- **9020** — REST. Used for admin (create instance / create database / DDL).

Wait ~2 seconds for the emulator to be ready. Confirm:

```bash
curl -fsS -o /dev/null http://localhost:9020/v1/projects/test-project/instances && echo ok
```

### 3. Create the instance and database

```bash
curl -fsS -X POST "http://localhost:9020/v1/projects/test-project/instances" \
  -H 'Content-Type: application/json' \
  -d '{"instance":{"config":"emulator-test-config","nodeCount":1,"displayName":"Test"},"instanceId":"test-instance"}'

curl -fsS -X POST "http://localhost:9020/v1/projects/test-project/instances/test-instance/databases" \
  -H 'Content-Type: application/json' \
  -d '{"createStatement":"CREATE DATABASE `test-database`"}'
```

### 4. Apply the schema

⚠️ **Do not run `scripts/prepare-spanner.sh` directly on macOS.** Its `sed` invocation uses GNU `\+` syntax that BSD `sed` doesn't accept. The script is designed to run inside the Linux container (where GNU sed lives). When you run it from a macOS host, the DDL parsing silently returns an empty array, the database gets created with no tables, and tests fail with cryptic *"Table not found: collections"* errors.

Use this macOS-safe inline equivalent:

```bash
DDL_STATEMENTS=$(grep -v '^--' syncstorage-spanner/src/schema.ddl \
  | tr '\n' ' ' \
  | tr -s ' ' \
  | sed 's/;[[:space:]]*$//' \
  | jq -R -s -c 'split(";") | map(select(length > 0)) | map(gsub("^\\s+|\\s+$";""))')

PAYLOAD=$(jq -n --argjson stmts "$DDL_STATEMENTS" '{statements:$stmts}')

curl -fsS -X PATCH "http://localhost:9020/v1/projects/test-project/instances/test-instance/databases/test-database/ddl" \
  -H 'Content-Type: application/json' \
  -d "$PAYLOAD"
```

Then verify the tables landed:

```bash
curl -fsS "http://localhost:9020/v1/projects/test-project/instances/test-instance/databases/test-database/ddl" \
  | jq '.statements | length'
# expected: 8 (5 tables + 3 indexes + 2 row-deletion policies, the last 2 are folded into CREATE TABLE statements by the emulator)
```

### 5. Run the unit tests

```bash
SYNC_SYNCSTORAGE__SPANNER_EMULATOR_HOST=localhost:9010 \
SYNC_SYNCSTORAGE__DATABASE_URL=spanner://projects/test-project/instances/test-instance/databases/test-database \
RUST_TEST_THREADS=1 \
cargo test --no-default-features --features=syncstorage-db/spanner --package syncstorage-db
```

Two non-obvious env vars:

- `SYNC_SYNCSTORAGE__SPANNER_EMULATOR_HOST` — the gRPC endpoint. **This is what tells the client to skip OAuth.** If you leave it unset, every test fails with *"Error occurred when fetching oauth2 token"* because the client tries to authenticate against the real Google API.
- `RUST_TEST_THREADS=1` — required. Many tests share the same database and assert on counts/timestamps; parallel runs race.

Expected output: `test result: ok. 39 passed; 0 failed; …`.

### 6. Run a single test

```bash
SYNC_SYNCSTORAGE__SPANNER_EMULATOR_HOST=localhost:9010 \
SYNC_SYNCSTORAGE__DATABASE_URL=spanner://projects/test-project/instances/test-instance/databases/test-database \
RUST_TEST_THREADS=1 \
cargo test --no-default-features --features=syncstorage-db/spanner --package syncstorage-db \
  tests::db::delete_collection_tombstone -- --nocapture
```

`--nocapture` is essential when tests fail — without it, errors get swallowed.

### 7. Teardown

```bash
docker compose -f docker/docker-compose.spanner.yaml down -v
```

The `-v` wipes named volumes for the whole compose project — including `tokenserver_db_data`. If you have the full e2e stack (or anything else from this compose file) running in parallel, prefer the helper script's narrower teardown instead:

```bash
./scripts/spanner-local-test-setup.sh down
```

That only stops + removes the `sync-db` container. The emulator keeps no data in named volumes, so next run starts from a fresh database either way.

## Common failures

| Symptom | Cause | Fix |
|---|---|---|
| `Error occurred when fetching oauth2 token` | Forgot `SYNC_SYNCSTORAGE__SPANNER_EMULATOR_HOST`. Client is trying real Google OAuth. | Export the var. |
| `Table not found: collections [at 2:25]` | DDL didn't apply. Most often because `prepare-spanner.sh` was run on macOS and produced an empty statement list. | Use the inline DDL block in step 4. |
| `INVALID_ARGUMENT: …` on `INSERT OR UPDATE` | Emulator version too old (< 1.5.x). | Update `docker/docker-compose.spanner.yaml`'s image tag. |
| 39 tests "pass" in 0.2s | Tests didn't actually connect — they're failing the OAuth check and treating it as a fresh start. Check the first test's error. | Same as the OAuth row above. |
| Tests intermittently fail with timestamp/count mismatches | `RUST_TEST_THREADS=1` not set. | Set it. |
| `cargo test` error: *the package 'syncstorage-db' does not contain this feature: py_verifier* | `py_verifier` is on the `syncserver` crate. | Drop `--features=py_verifier` when running tests on `syncstorage-db` alone. |

## Docker e2e (optional, slower path)

The Makefile target `docker_run_spanner_e2e_tests` spins up the full stack and runs the Python integration suite twice (with and without JWK cache). It requires the `app:build` image to be **current** — built against the working tree's code.

```bash
docker build -t app:build .
mkdir -p workflow/test-results
SYNCSTORAGE_RS_IMAGE=app:build make docker_run_spanner_e2e_tests
```

Two local-run-only gotchas the make target assumes CI handles:

- `SYNCSTORAGE_RS_IMAGE=app:build` must be exported. `docker-compose.spanner.yaml:52` references `${SYNCSTORAGE_RS_IMAGE:-syncstorage-rs:latest}`; without the override compose tries to pull a non-existent `syncstorage-rs:latest` from Docker Hub. CI sets it in `.github/workflows/main-workflow.yml`. Local doesn't get it for free.
- `workflow/test-results/` must exist before the make target runs. The target uses `docker cp ... ${INT_SPANNER_NO_JWK_JUNIT_XML}` which writes into that directory; if missing, the docker cp fails with `invalid output path` and aborts the run.

Two tests will always fail locally and can be safely ignored:

- `tools/integration_tests/tokenserver/test_e2e.py::test_unauthorized_oauth_error_status`
- `tools/integration_tests/tokenserver/test_e2e.py::test_valid_oauth_request`

Both hit **real FxA stage** (`api-accounts.stage.mozaws.net`, `oauth.stage.mozaws.net`) and need live network + production credentials the docker stack doesn't have. They surface as `fxa.errors.OutOfProtocolError: API responded with non-json content-type` — FxA returning an HTML error page because the request can't authenticate. CLAUDE.md excludes these from `make run_local_e2e_tests` via `--ignore=tokenserver/test_e2e.py`; the docker e2e target doesn't apply that exclusion, so expect `149 passed, 2 errors` not `151 passed`.

Notes:

- The image build compiles grpcio-sys from C++ source. On Docker Desktop for Mac, allocate **≥ 8 GB** memory.
- The build takes 10-20 minutes from cold. Subsequent rebuilds reuse cache layers and are much faster *unless* `Cargo.lock` or the Rust toolchain changed.
- Results land in `workspace/test-results/` as JUnit XML.
- The make target tears the stack down on completion.

### `ar: unable to copy file 'libgrpc.a'; reason: Success`

Misleading message — the linker wasn't killed, the BuildKit cache mount got into a corrupt state. Seen on macOS with stale buildkit caches (mine had ~19 GB accumulated).

Fix:

> ⚠️ `docker builder prune -f` clears BuildKit cache mounts for **every** Docker project on the host, not just this repo. Unrelated projects' next builds will go from cached to cold. Run it only when the link error reproduces and you've ruled out a real OOM.

```bash
docker builder prune -f
docker build -t app:build .
```

The prune drops the cache mounts grpcio-sys writes its 100+ MB `libgrpc.a` into. The next build re-creates them clean and the link step succeeds. If it fails a second time after prune, *then* suspect a real OOM and bump Docker Desktop memory.

If the local build won't cooperate, CI on push runs the same e2e suite — relying on it is reasonable for refactors that don't touch HTTP/auth/routing.

## When to invoke this skill

Reach for it when:

- You're editing anything under `syncstorage-spanner/` and want to validate before pushing.
- A spanner test is failing in CI and you need to reproduce locally.
- You're investigating an emulator-vs-prod behavior question.
- You forgot the env-var dance for the third time this month.

Don't invoke it for changes that don't touch Spanner — the MySQL or Postgres backends have their own test paths through the Makefile's `test_with_coverage` and `postgres_test_with_coverage` targets.

## Output format

After running, report:

1. **Emulator state:** running / fresh / pre-existing
2. **Schema applied:** N statements
3. **Test results:** `X passed; Y failed; Z ignored` — list the failed test names if any
4. **Total wall time**
5. **Teardown:** done / left running for follow-up

If anything failed, surface the actual error message (not just the test name) — Spanner emulator errors are verbose but informative, and the first line is usually enough to diagnose.
