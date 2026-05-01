# syncstorage-rs

Firefox Sync storage and token server, written in Rust. The workspace contains a
Rust multi-crate server plus Python utilities and integration/end-to-end tests.

## Agent rules

Security takes absolute precedence. This repository handles synchronization of
user data and token management to access storage nodes.

### 1) Scope & writes

- Operate strictly within this repo root; normalize paths; do not follow
  symlinks outside the repo.
- Writes are allowed to the working tree. Always present a diff for review
  before any staging or commit.
- Do not modify files adjacent to a requested change. Mention issues found
  but do not fix them unless asked.
- Ask before running any command (build, test, install, DB ops, git,
  service management). Do not run `git add`, `git commit`, `git push`, or
  `git rebase` unless explicitly instructed.

### 2) Non-negotiables

- **Secrets:** never read, print, summarize, or transmit secret files or
  values. Use placeholders (`YOUR_API_KEY_HERE`) in examples.
- **External network:** only with explicit approval to a trusted, documented
  endpoint.
- **Pipelines & contracts:** flag breaking API or contract changes. Do not
  alter GitHub Actions defined in `.github/`, any configuration for CI/CD workflows or git hooks without explicit, reviewed
  justification.
- **Published DB migrations:** never edit existing published migration files or schema definitions (ex. `syncstorage-spanner/src/schema.ddl`), whether for MySQL, Postgres, or Spanner.
  Always add a new forward migration and a separate rollback migration.
- **Workspace recommendations:** the repo deliberately ships no
  `.vscode/extensions.json`. Do not re-add one. Useful extensions are listed
  in `.vscode/README.md` for contributors to install at their own discretion.

### 3) Do-not-touch paths

Never read or write these paths. This list mirrors `.gitignore` — if there is
a discrepancy, `.gitignore` is the source of truth.

```
service-account.json      # GCP credentials
.sentryclirc              # Sentry auth token
.envrc                    # direnv secrets
config/local.toml         # local server config (may contain secrets)
tools/tokenserver/loadtests/*.pem
tools/tokenserver/loadtests/*.pub
target/                   # Rust build artifacts
*.xml                     # test result files
openapi.json              # generated artifact
book/                     # generated docs
docs/output/              # generated docs
venv/                     # Python virtual environments
.install.stamp            # Poetry install marker
workspace/                # CI artifact directory
```

### 4) Git commit messages

Follow the commit message format defined in [CONTRIBUTING.md](CONTRIBUTING.md) Git Commit Guidelines.

- Format: `type(scope): subject` — `type` is one of `feat`, `fix`, `docs`,
  `style`, `refactor`, `perf`, `test`, `chore`
- Subject: imperative present tense, no capital first letter, no trailing period
- Body: explain motivation and contrast with previous behavior
- Footer: `Closes STOR-1234` (Mozilla engineers — matches the Jira ticket) or
  `Closes #N` / `Issue #N` (community contributors — matches the GitHub issue number);
  `BREAKING CHANGE:` for breaking changes
- Branch naming (Mozilla engineers): `type/description-STOR-1234` where `1234` must match the numeric suffix of the associated Jira ticket
- Branch naming (community): `type/description-1112`
- All commits must be GPG/SSH signed (enforced by CI)

Example:

```
feat: add node reassignment on capacity overflow

Nodes at 100% capacity previously returned 503 rather than
assigning the user to an available node.

Closes #42.
```

## Project layout

```
syncserver/            Main Actix-web server (entry point)
syncstorage-db/        Syncstorage database abstraction layer
syncstorage-db-common/ Syncstorage shared database types and traits
syncstorage-mysql/     MySQL backend for syncstorage
syncstorage-postgres/  PostgreSQL backend for syncstorage
syncstorage-spanner/   Google Cloud Spanner backend for syncstorage
syncstorage-settings/  Syncstorage configuration types
tokenserver-db/        Tokenserver database abstraction layer
tokenserver-db-common/ Tokenserver shared database types
tokenserver-mysql/     MySQL backend for tokenserver
tokenserver-postgres/  PostgreSQL backend for tokenserver
tokenserver-auth/      HAWK token generation and verification
tokenserver-common/    Shared tokenserver utilities
tokenserver-settings/  Tokenserver configuration types
syncserver-common/     Shared server utilities
syncserver-db-common/  Shared database utilities
syncserver-settings/   Syncserver configuration types
glean/                 Glean telemetry crate
tools/
  integration_tests/   Pytest-based integration + e2e tests
  tokenserver/         Tokenserver utilities and load tests
  spanner/             Spanner utilities
  hawk/                HAWK token generation utility
  syncstorage-loadtest/ Molotov load tests
docker/                Docker Compose files for each backend
config/                Local TOML configuration files
docs/                  mdBook documentation source
.github/               GitHub CI Actions and configuration
```

## Rust builds

The default feature set targets MySQL, however production uses the Spanner database with MySQL Tokenserver. Backend features are mutually exclusive.

```bash
# MySQL (default)
cargo build

# PostgreSQL
cargo build --no-default-features --features=syncstorage-db/postgres --features=tokenserver-db/postgres --features=py_verifier

# Spanner
cargo build --no-default-features --features=syncstorage-db/spanner --features=py_verifier
```

## Rust tests

Tests require `RUST_TEST_THREADS=1` (many tests share a real database).

```bash
# MySQL unit tests (default DATABASE_URL envs set in Makefile)
make test

# Specific package
make test ARGS="--package syncstorage-db"

# With coverage
make test_with_coverage          # MySQL
make postgres_test_with_coverage
make spanner_test_with_coverage
```

## Clippy / formatting / audit

Formatting checks are necessary before applying any code changes, as they 
can capture problematic patterns or compiler issues in advance.

```bash
cargo fmt -- --check             # format check
cargo fmt                        # auto-format

make clippy_mysql
make clippy_postgres
make clippy_spanner

make clippy_release_mysql        # release-mode (catches dead code etc.)
make clippy_release_postgres
make clippy_release_spanner

cargo audit                      # check dependencies for known CVEs (run by CI on every push)
```

## Python tooling

All Python tooling uses [Poetry](https://python-poetry.org/). The root
`pyproject.toml` covers the `tools/` tree. Each subdirectory under `tools/` has
its own `pyproject.toml` for its specific dependencies.

All of these Python checks must be run with any related change to a Python
utility or test.

```bash
make install                     # install root dependencies
make integration-test            # install integration test dependencies
make tokenserver                 # install tokenserver utility dependencies
```

Lint / format / type-check:

```bash
make ruff-lint                   # ruff check tools/
make ruff-format                 # ruff format tools/
make ruff-fmt-chk                # format diff check
make mypy                        # mypy type checking
make pydocstyle                  # docstring validation
make bandit                      # security linting
```

## Integration / E2E tests

### Local (against a running server)

```bash
# Run all integration tests except real FxA e2e
PYTHONPATH=$(pwd)/tools \
SYNC_MASTER_SECRET=secret0 \
SYNC_TOKENSERVER__FXA_OAUTH_SERVER_URL=http://localhost:6000 \
TOKENSERVER_HOST=http://localhost:8000 \
poetry -C tools/integration_tests run pytest . --ignore=tokenserver/test_e2e.py
```

Or via the Makefile shortcut (sets the env vars above):

```bash
make run_local_e2e_tests
```

### Docker-based E2E (full stack)

```bash
make docker_run_mysql_e2e_tests
make docker_run_postgres_e2e_tests
make docker_run_spanner_e2e_tests
```

Each command spins up two compose stacks (with and without JWK cache), copies
JUnit XML results out of the container, then tears everything down.

## Running the server locally

```bash
# MySQL
make run_mysql

# Spanner (requires GCP service account key at ./service-account.json)
make run_spanner
```

Server listens on `http://localhost:8000`. Health check: `GET /__heartbeat__`.

## Key environment variables

| Variable | Purpose |
|---|---|
| `SYNC_MASTER_SECRET` | HAWK token signing secret |
| `SYNC_SYNCSTORAGE__DATABASE_URL` | Syncstorage DB connection string |
| `SYNC_TOKENSERVER__DATABASE_URL` | Tokenserver DB connection string |
| `SYNC_TOKENSERVER__NODE_TYPE` | `mysql` / `postgres` / `spanner` |
| `SYNC_TOKENSERVER__RUN_MIGRATIONS` | Auto-migrate on startup (`true`/`false`) |
| `SYNC_TOKENSERVER__FXA_OAUTH_SERVER_URL` | FxA OAuth endpoint |
| `RUST_TEST_THREADS` | Set to `1` for all test runs |
| `RUST_LOG` | Log level (`debug`, `info`, etc.) |
| `SYNC_TEST_LOG_HTTP` | Set any value to log all HTTP req/resp in integration tests |
| `SYNC_SERVER_URL` | Integration test base URL (default: `http://localhost:8000`) |
| `TOKENSERVER_HOST` | Tokenserver host for integration tests |
| `PYTHONPATH` | Must point to `$(pwd)/tools` when running integration tests |

## Docker Compose files

| File | Purpose |
|---|---|
| `docker/docker-compose.mysql.yaml` | MySQL syncstorage + MySQL tokenserver |
| `docker/docker-compose.postgres.yaml` | PostgreSQL syncstorage + PostgreSQL tokenserver |
| `docker/docker-compose.spanner.yaml` | Spanner emulator + MySQL tokenserver |
| `docker/docker-compose.e2e.{mysql,postgres,spanner}.yaml` | E2E test runner overlay |
| `docker/docker-compose.e2e.jwk-cache.yaml` | Enable JWK caching overlay |
| `docker/docker-compose.e2e.no-jwk-cache.yaml` | Disable JWK caching overlay |

## Documentation

```bash
make doc-install-deps            # install mdBook + mdBook-mermaid
make doc-watch                   # live preview at localhost
make doc-prev                    # build and serve
make doc-test                    # validate markdown

make api-prev                    # generate OpenAPI spec + serve Swagger UI on :8080
```


## Test structure

- `tools/integration_tests/conftest.py` — pytest fixtures only (`st_ctx`)
- `tools/integration_tests/helpers.py` — retry helpers, auth state, `switch_user`
- `tools/integration_tests/test_storage.py` — storage protocol tests
- `tools/integration_tests/tokenserver/conftest.py` — tokenserver fixtures only
- `tools/integration_tests/tokenserver/helpers.py` — tokenserver DB/auth helpers
- `tools/integration_tests/tokenserver/test_*.py` — tokenserver tests
