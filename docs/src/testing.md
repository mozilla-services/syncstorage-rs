
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

Each E2E test run:
1. Starts the required services (database, mock FxA server, syncserver) using docker-compose
2. Runs the Python integration tests with JWK caching enabled
3. Runs the tests again with JWK caching disabled
4. Outputs JUnit XML test results

The E2E test configurations are defined in:
- [docker-compose.e2e.mysql.yaml](../../docker-compose.e2e.mysql.yaml)
- [docker-compose.e2e.postgres.yaml](../../docker-compose.e2e.postgres.yaml)
- [docker-compose.e2e.spanner.yaml](../../docker-compose.e2e.spanner.yaml)

These compose files extend the base service definitions from their corresponding `docker-compose.<backend>.yaml` files.

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
