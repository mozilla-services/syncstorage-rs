[![License: MPL 2.0][mpl-svg]][mpl] [![Build Status][circleci-badge]][circleci] [![Connect to Matrix via the Riot webapp][matrix-badge]][matrix][![Swagger OpenAPI Docs](https://img.shields.io/badge/API-Documentation-blue.svg)][swagger-ui]

# Syncstorage-rs

Mozilla's Sync provides a secure method for users to synchronize their data across Mozilla applications (like Firefox) using a Mozilla account. This project encapsulates the backend of the Sync service. It can be run using either a Postgres, Spanner, or MySQL database backend.

Sync operates by storing a combined version of your data on a remote server, which then synchronizes with the local Firefox copy across all your signed-in instances (referred to as connected devices, linked through a Mozilla account). 

## Get up and Running

To get up and running quickly, see [Run Your Own Sync with Docker](how-to/how-to-run-with-docker.md) for instructions on deploying with Docker.

For a complete list of available configuration options you'll need to consider, see the [Configuration](config.md) reference.

Below are detailed instructions for other setup configurations, including using the Google Spanner Emulator and MySQL.

Mozilla Sync Storage built with [Rust](https://rust-lang.org). Our documentation is generated using [mdBook](https://rust-lang.github.io/mdBook/index.html) and published to GitHub Pages.

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->


- [System Requirements](#system-requirements)
- [Local Setup](#local-setup)
  - [MySQL](#mysql)
  - [Spanner](#spanner)
  - [Running via Docker](#running-via-docker)
  - [Connecting to Firefox](#connecting-to-firefox)
- [Logging](#logging)
  - [Sentry:](#sentry)
  - [RUST_LOG](#rust_log)
- [Tests](#tests)
  - [Unit tests](#unit-tests)
  - [End-to-End tests](#end-to-end-tests)
- [Creating Releases](#creating-releases)
- [Troubleshooting](#troubleshooting)
- [Related Documentation](#related-documentation)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

## Initial Setup - Bootstrapping

### General PostgreSQL Setup

Syncstorage-rs supports PostgreSQL as a database backend. The database connection is specified with a DSN like:

`postgres://_user_:_password_@_host_/_database_`

This DSN is then used for the `SYNC_TOKENSERVER__DATABASE_URL` & `SYNC_SYNCSTORAGE__DATABASE_URL` URLs.

Use your preferred methods, however here are some general instructions on how to setup a fresh PostgreSQL database and user:

1. First make sure you have a PostgreSQL server running. On most systems, you can start it with:
   ```bash
   # On macOS with Homebrew
   brew services start postgresql

   # On Ubuntu/Debian
   sudo systemctl start postgresql
   ```

2. Create the databases using `createdb`:
   ```bash
   createdb -U postgres syncstorage
   createdb -U postgres tokenserver
   ```

3. Connect to PostgreSQL to create a user and grant privileges:
   ```bash
   psql -U postgres -d syncstorage
   ```

4. Run the following SQL statements:
   ```sql
   CREATE USER sample_user WITH PASSWORD 'sample_password';
   GRANT ALL PRIVILEGES ON DATABASE syncstorage TO sample_user;
   GRANT ALL PRIVILEGES ON DATABASE tokenserver TO sample_user;
   ```

**Connection pattern:** The general pattern for connecting to a PostgreSQL database is:
```bash
psql -d database_name -U username
```
The `-d` flag is a shorter alternative for `--dbname` while `-U` is an alternative for `--username`.

**Environment configuration:** You can optionally create a `.env` file with your database URL:
```bash
echo "DATABASE_URL=postgres://sample_user:sample_password@localhost/syncstorage" > .env
```

Or manually create the file:
```bash
touch .env
```
And add:
`DATABASE_URL=postgres://sample_user:sample_password@localhost/syncstorage`
### Bootstrapping Tokenserver (Postgres)

Tokenserver includes migrations to initialize its database, but they do not run by default. These can be enabled via the setting:

```bash
SYNC_TOKENSERVER__RUN_MIGRATIONS=true
```

Once you have created and defined your database, copy the URL.

```bash
SYNC_TOKENSERVER__DATABASE_URL=postgres://<DB URL>
```

After migrations run, insert a node entry:
```sql
INSERT INTO nodes (id, service, node, available, current_load, capacity, downed, backoff)
VALUES (1, 1, 'https://<SYNCSTORAGE URL HERE>', 100000, 0, 100000, 0, 0)
ON CONFLICT DO NOTHING;
```

### Bootstrapping Syncstorage (Postgres)

Syncstorage includes migrations to initialize its database. These run by default (unlike Tokenserver).

Configure the database URL:
```bash
SYNC_SYNCSTORAGE__DATABASE_URL=postgres://<DB URL>
```

### Bootstrapping Tokenserver (MySQL)

Tokenserver includes migrations to initialize its database, but they do not run by default. These can be enabled via the setting:

```bash
SYNC_TOKENSERVER__RUN_MIGRATIONS=true
```

**NOTE:** These migrations don't run with any locking (at least on MySQL), it's probably safest to limit the node count to 1 during the first run.

After migrations run, insert service and node entries:
```sql
INSERT INTO services (id, service, pattern)
VALUES (1, 'sync-1.5', '{node}/1.5/{uid}');

INSERT IGNORE INTO nodes (id, service, node, available, current_load, capacity, downed, backoff)
VALUES (1, 1, 'https://ent-dev.sync.nonprod.webservices.mozgcp.net', 100, 0, 100, 0, 0);
```

### Bootstrapping Syncstorage (Cloud Spanner)

Syncstorage does not support initializing Cloud Spanner instances; this must be done manually. It does support initializing its MySQL backend and will support initializing the PostgreSQL backend in the future.

The schema DDL is available here: [schema.ddl](https://github.com/mozilla-services/syncstorage-rs/blob/master/syncstorage-spanner/src/schema.ddl)

We include a basic script to create an instance and initialize the schema via Spanner's REST API: [prepare-spanner.sh](https://github.com/mozilla-services/syncstorage-rs/blob/master/scripts/prepare-spanner.sh). This script is currently oriented to run against Cloud Spanner emulators, but it may be adapted to run against a real Spanner database.

## System Requirements

- cmake (>= 3.5 and < 3.30)
- gcc
- [golang](https://golang.org/doc/install)
- libcurl4-openssl-dev
- libssl-dev
- make
- pkg-config
- [Rust stable](https://rustup.rs)
- python 3.9+
- MySQL 8.0 (or compatible)
  * libmysqlclient (`brew install mysql` on macOS, `apt install libmysqlclient-dev` on Ubuntu, `apt install libmariadb-dev-compat` on Debian)

Depending on your OS, you may also need to install `libgrpcdev`,
and `protobuf-compiler-grpc`. *Note*: if the code complies cleanly,
but generates a Segmentation Fault within Sentry init, you probably
are missing `libcurl4-openssl-dev`.

## Local Setup

1. Follow the instructions below to use either MySQL or Spanner as your DB.
2. Now `cp config/local.example.toml config/local.toml`. Open `config/local.toml` and make sure you have the desired settings configured. For a complete list of available configuration options, check out [docs/config.md](docs/config.md).
3. To start a local server in debug mode, run either:
    - `make run_mysql` if using MySQL or,
    - `make run_spanner` if using spanner.

    The above starts the server in debug mode, using your new `local.toml` file for config options. Or, simply `cargo run` with your own config options provided as env vars.
4. Visit `http://localhost:8000/__heartbeat__` to make sure the server is running.

### MySQL

Durable sync needs only a valid mysql DSN in order to set up connections to a MySQL database. The database can be local and is usually specified with a DSN like:

`mysql://_user_:_password_@_host_/_database_`

To setup a fresh MySQL DB and user:

- First make sure that you have a MySQL server running, to do that run: `mysqld`
- Then, run the following to launch a mysql shell `mysql -u root`
- Finally, run each of the following SQL statements

```sql
CREATE USER "sample_user"@"localhost" IDENTIFIED BY "sample_password";
CREATE DATABASE syncstorage_rs;
CREATE DATABASE tokenserver_rs;

GRANT ALL PRIVILEGES on syncstorage_rs.* to sample_user@localhost;
GRANT ALL PRIVILEGES on tokenserver_rs.* to sample_user@localhost;
```

Note that if you are running MySQL with Docker and encountered a socket connection error, change the MySQL DSN from `localhost` to `127.0.0.1` to use a TCP connection.

### Spanner

#### Authenticating via OAuth
The correct way to authenticate with Spanner is by generating an OAuth token and pointing your local application server to the token. In order for this to work, your Google Cloud account must have the correct permissions; contact the Ops team to ensure the correct permissions are added to your account.

First, install the Google Cloud command-line interface by following the instructions for your operating system [here](https://cloud.google.com/sdk/docs/install). Next, run the following to log in with your Google account (this should be the Google account associated with your Mozilla LDAP credentials):
```sh
gcloud auth application-default login
```
The above command will prompt you to visit a webpage in your browser to complete the login process. Once completed, ensure that a file called `application_default_credentials.json` has been created in the appropriate directory (on Linux, this directory is `$HOME/.config/gcloud/`). The Google Cloud SDK knows to check this location for your credentials, so no further configuration is needed.

##### Key Revocation
Accidents happen, and you may need to revoke the access of a set of credentials if they have been publicly leaked. To do this, run:
```sh
gcloud auth application-default revoke
```
This will revoke the access of the credentials currently stored in the `application_default_credentials.json` file. **If the file in that location does not contain the leaked credentials, you will need to copy the file containing the leaked credentials to that location and re-run the above command.** You can ensure that the leaked credentials are no longer active by attempting to connect to Spanner using the credentials. If access has been revoked, your application server should print an error saying that the token has expired or has been revoked.

#### Authenticating via Service Account
An alternative to authentication via application default credentials is authentication via a service account. **Note that this method of authentication is not recommended. Service accounts are intended to be used by other applications or virtual machines and not people. See [this article](https://cloud.google.com/iam/docs/service-accounts#what_are_service_accounts) for more information.**

Your system administrator will be able to tell you which service account keys have access to the Spanner instance to which you are trying to connect. Once you are given the email identifier of an active key, log into the [Google Cloud Console Service Accounts](https://console.cloud.google.com/iam-admin/serviceaccounts) page. Be sure to select the correct project.

- Locate the email identifier of the access key and pick the vertical dot menu at the far right of the row.
- Select "_Create Key_" from the pop-up menu.
- Select "JSON" from the Dialog Box.

A proper key file will be downloaded to your local directory. It's important to safeguard that key file. For this example, we're going to name the file
`service-account.json`.

The proper key file is in JSON format. An example file is provided below, with private information replaced by "`...`"

```json
{
  "type": "service_account",
  "project_id": "...",
  "private_key_id": "...",
  "private_key": "...",
  "client_email": "...",
  "client_id": "...",
  "auth_uri": "https://accounts.google.com/o/oauth2/auth",
  "token_uri": "https://oauth2.googleapis.com/token",
  "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs",
  "client_x509_cert_url": "..."
}
```

**Note that the name `service-account.json` must be exactly correct to be ignored by `.gitignore`.**

#### Connecting to Spanner
To point to a GCP-hosted Spanner instance from your local machine, follow these steps:

1. Authenticate via either of the two methods outlined above.
2. Open `local.toml` and replace `database_url` with a link to your spanner instance.
3. Open the Makefile and ensure you've correctly set you `PATH_TO_GRPC_CERT`.
4. `make run_spanner`.
5. Visit `http://localhost:8000/__heartbeat__` to make sure the server is running.

Note, that unlike MySQL, there is no automatic migrations facility. Currently, the Spanner schema must be hand edited and modified.

#### Emulator

Google supports an in-memory Spanner emulator, which can run on your local machine for development purposes. You can install the emulator via the gcloud CLI or Docker by following the instructions [here](https://cloud.google.com/spanner/docs/emulator#installing_and_running_the_emulator). Once the emulator is running, you'll need to create a new instance and a new database.

##### Quick Setup Using prepare-spanner.sh

The easiest way to set up a Spanner emulator database is to use the `prepare-spanner.sh` script:

```sh
SYNC_SYNCSTORAGE__SPANNER_EMULATOR_HOST=localhost:9020 ./scripts/prepare-spanner.sh
```

This script will automatically:
1. Create a test instance (`test-instance`) on a test project (`test-project`)
2. Create a test database (`test-database`) with the schema from `schema.ddl`
3. Apply all DDL statements to set up the database structure

The script looks for `schema.ddl` in either the current directory or in `syncstorage-spanner/src/`. Make sure the `SYNC_SYNCSTORAGE__SPANNER_EMULATOR_HOST` environment variable points to your emulator's REST API endpoint (typically `localhost:9020`).

After running the script, make sure that the `database_url` config variable in your `local.toml` file reflects the created database (i.e. `spanner://projects/test-project/instances/test-instance/databases/test-database`).

To run an application server that points to the local Spanner emulator:

```sh
SYNC_SYNCSTORAGE__SPANNER_EMULATOR_HOST=localhost:9010 make run_spanner
```

##### Manual Setup Using curl

If you prefer to manually create the instance and database, or need custom project/instance/database names, you can use the REST API directly. The Spanner emulator exposes a REST API on port 9020. To create an instance, use `curl`:

```sh
curl --request POST \
  "localhost:9020/v1/projects/$PROJECT_ID/instances" \
  --header 'Accept: application/json' \
  --header 'Content-Type: application/json' \
  --data "{\"instance\":{\"config\":\"emulator-test-config\",\"nodeCount\":1,\"displayName\":\"Test Instance\"},\"instanceId\":\"$INSTANCE_ID\"}"
```

Note that you may set `PROJECT_ID` and `INSTANCE_ID` to your liking. To create a new database on this instance, you'll need to include information about the database schema. Since we don't have migrations for Spanner, we keep an up-to-date schema in `src/db/spanner/schema.ddl`. The `jq` utility allows us to parse this file for use in the JSON body of an HTTP POST request:

```sh
DDL_STATEMENTS=$(
  grep -v ^-- schema.ddl \
  | sed -n 's/ \+/ /gp' \
  | tr -d '\n' \
  | sed 's/\(.*\);/\1/' \
  | jq -R -s -c 'split(";")'
)
```

This command:
- Filters out SQL comments (lines starting with `--`)
- Normalizes whitespace
- Removes newlines to create a single line
- Removes the trailing semicolon from the concatenated string
- Splits the DDL statements back into an array using `jq`

Finally, to create the database:

```sh
curl -sS --request POST \
  "localhost:9020/v1/projects/$PROJECT_ID/instances/$INSTANCE_ID/databases" \
  --header 'Accept: application/json' \
  --header 'Content-Type: application/json' \
  --data "{\"createStatement\":\"CREATE DATABASE \`$DATABASE_ID\`\",\"extraStatements\":$DDL_STATEMENTS}"
```

Note that, again, you may set `DATABASE_ID` to your liking. Make sure that the `database_url` config variable in your `local.toml` file reflects your choice of project name, instance name, and database name (i.e. it should be of the format `spanner://projects/<your project ID here>/instances/<your instance ID here>/databases/<your database ID here>`).

To run the application server that points to the local Spanner emulator:

```sh
SYNC_SYNCSTORAGE__SPANNER_EMULATOR_HOST=localhost:9010 make run_spanner
```

### Running via Docker

This requires access to [Google Cloud Rust (raw)](https://crates.io/crates/google-cloud-rust-raw/) crate. Please note that due to interdependencies, you will need to ensure that `grpcio` and `protobuf` match the version used by `google-cloud-rust-raw`.

1. Make sure you have [Docker installed](https://docs.docker.com/install/) locally.
2. Copy the contents of mozilla-rust-sdk into top level root dir here.
3. Comment out the `image` value under `syncserver` in either docker-compose.mysql.yml or docker-compose.spanner.yml (depending on which database backend you want to run), and add this instead:

    ```yml
      build:
        context: .
    ```

4. If you are using MySQL, adjust the MySQL db credentials in docker-compose.mysql.yml to match your local setup.
5. `make docker_start_mysql` or `make docker_start_spanner` - You can verify it's working by visiting [localhost:8000/\_\_heartbeat\_\_](http://localhost:8000/__heartbeat__)

### Connecting to Firefox

This will walk you through the steps to connect this project to your local copy of Firefox.

1. Follow the steps outlined above for running this project using MySQL or Spanner.
2. In Firefox, go to `about:config`. Change `identity.sync.tokenserver.uri` to `http://localhost:8000/1.0/sync/1.5`.
3. Restart Firefox. Now, try syncing. You should see new BSOs in your MySQL or Spanner instance.

## Logging

### Sentry:
1. If you want to connect to the existing [Sentry project](https://sentry.prod.mozaws.net/operations/syncstorage-local/) for local development, login to Sentry, and go to the page with [api keys](https://sentry.prod.mozaws.net/settings/operations/syncstorage-local/keys/). Copy the `DSN` value.
2. Comment out the `human_logs` line in your `config/local.toml` file.
3. You can force an error to appear in Sentry by adding a `panic!` into main.rs, just before the final `Ok(())`.
4. Now, `SENTRY_DSN={INSERT_DSN_FROM_STEP_1_HERE} make run`.
5. You may need to stop the local server after it hits the panic! before errors will appear in Sentry.

### RUST_LOG

We use [env_logger](https://crates.io/crates/env_logger): set the `RUST_LOG` env var.

The logging of non-Spanner SQL queries is supported in non-optimized builds via `RUST_LOG=syncserver=debug`.

## Troubleshooting

- `rm Cargo.lock; cargo clean;` - Try this if you're having problems compiling.

- Some versions of OpenSSL 1.1.1 can conflict with grpcio's built in BoringSSL. These errors can cause syncstorage to fail to run or compile.
If you see a problem related to `libssl` you may need to specify the `cargo` option `--features grpcio/openssl`  to force grpcio to use OpenSSL.

### Sentry

- If you're having trouble working with Sentry to create releases, try authenticating using their self hosted server option that's outlined [here](https://docs.sentry.io/product/cli/configuration/) Ie, `sentry-cli --url https://selfhosted.url.com/ login`. It's also recommended to create a `.sentryclirc` config file. See [this example](https://github.com/mozilla-services/syncstorage-rs/blob/master/.sentryclirc.example) for the config values you'll need.

## Tests

### Unit tests

Run unit tests for a specific database backend using one of the following make targets:

- MySQL: `make test` or `make test_with_coverage`
- Postgres: `make postgres_test_with_coverage`
- Spanner: `make spanner_test_with_coverage`

These commands will run the Rust test suite using cargo-nextest and generate coverage reports using cargo-llvm-cov.

### End-to-End tests

End-to-end (E2E) tests validate the complete integration of syncstorage-rs with a real database backend and mock Firefox Accounts server. These tests run the full Python integration test suite located in [tools/integration_tests/](../../tools/integration_tests/).

#### Running E2E Tests Locally

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

- [System Requirements](#system-requirements)
- [Local Setup](#local-setup)
  - [MySQL](#mysql)
  - [Spanner](#spanner)
  - [Running via Docker](#running-via-docker)
  - [Connecting to Firefox](#connecting-to-firefox)
- [Logging](#logging)
  - [Sentry:](#sentry)
  - [RUST_LOG](#rust_log)
- [Tests](#tests)
  - [Unit tests](#unit-tests)
  - [End-to-End tests](#end-to-end-tests)
- [Creating Releases](#creating-releases)
- [Troubleshooting](#troubleshooting)
- [Related Documentation](#related-documentation)


## System Requirements

- cmake (>= 3.5 and < 3.30)
- gcc
- [golang](https://golang.org/doc/install)
- libcurl4-openssl-dev
- libssl-dev
- make
- pkg-config
- [Rust stable](https://rustup.rs)
- python 3.9+
- MySQL 8.0 (or compatible)
  * libmysqlclient (`brew install mysql` on macOS, `apt install libmysqlclient-dev` on Ubuntu, `apt install libmariadb-dev-compat` on Debian)

Depending on your OS, you may also need to install `libgrpcdev`,
and `protobuf-compiler-grpc`. *Note*: if the code complies cleanly,
but generates a Segmentation Fault within Sentry init, you probably
are missing `libcurl4-openssl-dev`.


[mpl-svg]: https://img.shields.io/badge/License-MPL%202.0-blue.svg
[mpl]: https://opensource.org/licenses/MPL-2.0
[circleci-badge]: https://circleci.com/gh/mozilla-services/syncstorage-rs.svg?style=shield
[circleci]: https://circleci.com/gh/mozilla-services/syncstorage-rs
[matrix-badge]: https://img.shields.io/badge/chat%20on%20[m]-%23services%3Amozilla.org-blue
[matrix]: https://chat.mozilla.org/#/room/#services:mozilla.org
[swagger-ui]: https://mozilla-services.github.io/syncstorage-rs/swagger-ui/
