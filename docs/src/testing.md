
# Tests

## Unit tests

Run unit tests for a specific database backend using one of the following make targets:

- MySQL: `make test` or `make test_with_coverage`
- Postgres: `make postgres_test_with_coverage`
- Spanner: `make spanner_test_with_coverage`

These commands will run the Rust test suite using cargo-nextest and generate coverage reports using cargo-llvm-cov.

You'll need [`nextest`](https://nexte.st/docs/installation/from-source/) and [`llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov?tab=readme-ov-file#installation) installed for full unittest and test coverage.

```bash
  $ cargo install cargo-nextest --locked
  $ cargo install cargo-llvm-cov --locked
```

- `make test` - Runs all tests
- `make test_with_coverage` - This will use `llvm-cov` to run tests and generate [source-based code coverage](https://clang.llvm.org/docs/SourceBasedCodeCoverage.html)

If you need to override `SYNC_SYNCSTORAGE__DATABASE_URL` or `SYNC_TOKENSERVER__DATABASE_URL` variables, you can modify them in the `Makefile` or by setting them in your shell:
```bash
  $ echo 'export SYNC_SYNCSTORAGE__DATABASE_URL="mysql://sample_user:sample_password@localhost/syncstorage_rs"' >> ~/.zshrc
  $ echo 'export SYNC_TOKENSERVER__DATABASE_URL="mysql://sample_user:sample_password@localhost/tokenserver?rs"' >> ~/.zshrc
```

### Debugging unit test state

In some cases, it is useful to inspect the mysql state of a failed test. By
default, we use the diesel test_transaction functionality to ensure test data
is not committed to the database. Therefore, there is an environment variable
which can be used to turn off test_transaction.
```bash
  SYNC_SYNCSTORAGE__DATABASE_USE_TEST_TRANSACTIONS=false make test ARGS="[testname]"
```

Note that you will almost certainly want to pass a single test name. When running
the entire test suite, data from previous tests will cause future tests to fail.

To reset the database state between test runs, drop and recreate the database
in the mysql client:

  `drop database syncstorage_rs; create database syncstorage_rs; use syncstorage_rs;`

## End-to-End tests

End-to-end (E2E) tests validate the complete integration of syncstorage-rs with a real database backend and mock Firefox Accounts server. These tests run the full Python integration test suite located in [tools/integration_tests/](../../tools/integration_tests/).

### Running E2E Tests Locally

To run E2E tests, you'll need to:

1. Build a Docker image for your target backend using the appropriate Makefile target
2. Run the E2E test suite using docker-compose

The E2E tests are available for three database backends:

**MySQL:**
```bash
make docker_run_mysql_e2e_tests
```

**Postgres:**
```bash
make docker_run_postgres_e2e_tests
```

**Spanner:**
```bash
make docker_run_spanner_e2e_tests
```

Each E2E test run performs two separate docker-compose invocations:
1. **No Local JWK run**: starts services with no local JWK configured, runs only `test_e2e.py` against FxA stage
2. **Local JWK & Mocked FxA run**: runs all integration tests using a mocked local FxA server and local JWK; the local JWK affects only the tests in `test_e2e.py`
3. Outputs JUnit XML test results for each run

The E2E test configurations are defined in:
- [docker/docker-compose.e2e.mysql.yaml](../../docker/docker-compose.e2e.mysql.yaml) - base
- [docker/docker-compose.e2e.mysql.jwk-cache.yaml](../../docker/docker-compose.e2e.mysql.jwk-cache.yaml) - JWK + mock FxA overlay
- [docker/docker-compose.e2e.mysql.no-jwk-cache.yaml](../../docker/docker-compose.e2e.mysql.no-jwk-cache.yaml) - FxA stage overlay
- [docker/docker-compose.e2e.postgres.yaml](../../docker/docker-compose.e2e.postgres.yaml)
- [docker/docker-compose.e2e.postgres.jwk-cache.yaml](../../docker/docker-compose.e2e.postgres.jwk-cache.yaml)
- [docker/docker-compose.e2e.postgres.no-jwk-cache.yaml](../../docker/docker-compose.e2e.postgres.no-jwk-cache.yaml)
- [docker/docker-compose.e2e.spanner.yaml](../../docker/docker-compose.e2e.spanner.yaml)
- [docker/docker-compose.e2e.spanner.jwk-cache.yaml](../../docker/docker-compose.e2e.spanner.jwk-cache.yaml)
- [docker/docker-compose.e2e.spanner.no-jwk-cache.yaml](../../docker/docker-compose.e2e.spanner.no-jwk-cache.yaml)

These compose files extend the base service definitions from their corresponding `docker/docker-compose.<backend>.yaml` files. Syncserver configuration (JWK, FxA OAuth URL, CORS) is defined in the `syncserver` block of the e2e overlays.

#### How E2E Tests Work

The E2E tests:
- Run in a containerized environment with all dependencies (database, syncserver, mock FxA)
- Execute integration tests from [tools/integration_tests/](../../tools/integration_tests/) using pytest
- Test OAuth token validation with both cached and non-cached JWKs
- Validate tokenserver functionality, including user allocation and token generation
- Test syncstorage operations like BSO creation, retrieval, and deletion

#### CI/CD

In GitHub Actions, E2E tests run as part of the CI/CD pipeline for each backend:
- [.github/workflows/mysql.yml](../../.github/workflows/mysql.yml) - `mysql-e2e-tests` job
- [.github/workflows/postgres.yml](../../.github/workflows/postgres.yml) - `postgres-e2e-tests` job
- [.github/workflows/spanner.yml](../../.github/workflows/spanner.yml) - `spanner-e2e-tests` job

Each workflow builds a Docker image, runs unit tests, then executes E2E tests using the same make targets described above.

### Running E2E Tests Against a Local Server

You can run the integration tests against a locally running Sync server.  Start your server, then use the `run_local_e2e_tests` make target:

```bash
SYNC_SYNCSTORAGE__DATABASE_URL="postgres://user:pass@localhost/syncstorage" \
SYNC_TOKENSERVER__DATABASE_URL="postgres://user:pass@localhost/tokenserver" \
make run_local_e2e_tests
```

The target uses the following env vars:
- `SYNC_SERVER_URL` (default: `http://localhost:8000`)
- `TOKENSERVER_HOST` (default: `http://localhost:8000`)
- `SYNC_MASTER_SECRET` (default: `secret0`)
- `SYNC_TOKENSERVER__FXA_OAUTH_SERVER_URL` (default: `http://localhost:6000`)

`SYNC_SYNCSTORAGE__DATABASE_URL` and `SYNC_TOKENSERVER__DATABASE_URL` must be set when `make` is invoked.

To run a specific test by name:

```bash
SYNC_SYNCSTORAGE__DATABASE_URL="..." \
SYNC_TOKENSERVER__DATABASE_URL="..." \
PYTHONPATH=/path/to/syncstorage-rs/tools \
TOKENSERVER_HOST=http://localhost:8000 \
poetry -C /path/to/syncstorage-rs/tools/integration_tests \
  run pytest . -k test_meta_global_sanity
```

Or by full module path:

```bash
SYNC_SYNCSTORAGE__DATABASE_URL="..." \
SYNC_TOKENSERVER__DATABASE_URL="..." \
PYTHONPATH=/path/to/syncstorage-rs/tools \
TOKENSERVER_HOST=http://localhost:8000 \
poetry -C /path/to/syncstorage-rs/tools/integration_tests \
  run pytest tokenserver/test_authorization.py::TestAuthorization::test_authorized_request
```

#### HTTP Request and Response Logging

Set `SYNC_TEST_LOG_HTTP=1` and pass `--log-cli-level=INFO` to pytest log HTTP requests and responses to stdout:

```bash
SYNC_SYNCSTORAGE__DATABASE_URL="..." \
SYNC_TOKENSERVER__DATABASE_URL="..." \
PYTHONPATH=/path/to/syncstorage-rs/tools \
TOKENSERVER_HOST=http://localhost:8000 \
SYNC_TEST_LOG_HTTP=1 \
poetry -C /path/to/syncstorage-rs/tools/integration_tests \
  run pytest . -k test_meta_global_sanity --log-cli-level=INFO
```

