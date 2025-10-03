[![License: MPL 2.0][mpl-svg]][mpl] [![Build Status][circleci-badge]][circleci] [![Connect to Matrix via the Riot webapp][matrix-badge]][matrix]

# Syncstorage-rs

Mozilla Sync Storage built with [Rust](https://rust-lang.org).

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

Google supports an in-memory Spanner emulator, which can run on your local machine for development purposes. You can install the emulator via the gcloud CLI or Docker by following the instructions [here](https://cloud.google.com/spanner/docs/emulator#installing_and_running_the_emulator). Once the emulator is running, you'll need to create a new instance and a new database. To create an instance using the REST API (exposed via port 9020 on the emulator), we can use `curl`:

```sh
curl --request POST \
  "localhost:9020/v1/projects/$PROJECT_ID/instances" \
  --header 'Accept: application/json' \
  --header 'Content-Type: application/json' \
  --data "{\"instance\":{\"config\":\"emulator-test-config\",\"nodeCount\":1,\"displayName\":\"Test Instance\"},\"instanceId\":\"$INSTANCE_ID\"}"
```

Note that you may set `PROJECT_ID` and `INSTANCE_ID` to your liking. To create a new database on this instance, we'll use a similar HTTP request, but we'll need to include information about the database schema. Since we don't have migrations for Spanner, we keep an up-to-date schema in `src/db/spanner/schema.ddl`. The `jq` utility allows us to parse this file for use in the JSON body of an HTTP POST request:

```sh
DDL_STATEMENTS=$(
  grep -v ^-- schema.ddl \
  | sed -n 's/ \+/ /gp' \
  | tr -d '\n' \
  | sed 's/\(.*\);/\1/' \
  | jq -R -s -c 'split(";")'
)
```

Finally, to create the database:

```sh
curl -sS --request POST \
  "localhost:9020/v1/projects/$PROJECT_ID/instances/$INSTANCE_ID/databases" \
  --header 'Accept: application/json' \
  --header 'Content-Type: application/json' \
  --data "{\"createStatement\":\"CREATE DATABASE \`$DATABASE_ID\`\",\"extraStatements\":$DDL_STATEMENTS}"
```

Note that, again, you may set `DATABASE_ID` to your liking. Make sure that the `database_url` config variable reflects your choice of project name, instance name, and database name (i.e. it should be of the format `spanner://projects/<your project ID here>/instances/<your instance ID here>/databases/<your database ID here>`).

To run an application server that points to the local Spanner emulator:

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

## Tests

### Unit tests

You'll need [`nextest`](https://nexte.st/docs/installation/from-source/) and [`llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov?tab=readme-ov-file#installation) installed for full unittest and test coverage.

    $ cargo install cargo-nextest --locked
    $ cargo install cargo-llvm-cov --locked

- `make test` - Runs all tests
- `make test_with_coverage` - This will use `llvm-cov` to run tests and generate [source-based code coverage](https://clang.llvm.org/docs/SourceBasedCodeCoverage.html)

If you need to override `SYNC_SYNCSTORAGE__DATABASE_URL` or `SYNC_TOKENSERVER__DATABASE_URL` variables, you can modify them in the `Makefile` or by setting them in your shell

    $ echo 'export SYNC_SYNCSTORAGE__DATABASE_URL="mysql://sample_user:sample_password@localhost/syncstorage_rs"' >> ~/.zshrc
    $ echo 'export SYNC_TOKENSERVER__DATABASE_URL="mysql://sample_user:sample_password@localhost/tokenserver?rs"' >> ~/.zshrc

#### Debugging unit test state

In some cases, it is useful to inspect the mysql state of a failed test. By
default, we use the diesel test_transaction functionality to ensure test data
is not committed to the database. Therefore, there is an environment variable
which can be used to turn off test_transaction.

        SYNC_SYNCSTORAGE__DATABASE_USE_TEST_TRANSACTIONS=false make test ARGS="[testname]"

Note that you will almost certainly want to pass a single test name. When running
the entire test suite, data from previous tests will cause future tests to fail.

To reset the database state between test runs, drop and recreate the database
in the mysql client:

        drop database syncstorage_rs; create database syncstorage_rs; use syncstorage_rs;

### End-to-End tests

Functional tests live in [server-syncstorage](https://github.com/mozilla-services/server-syncstorage/) and can be run against a local server, e.g.:

1.  If you haven't already followed the instructions [here](https://mozilla-services.readthedocs.io/en/latest/howtos/run-sync-1.5.html) to get all the dependencies for the [server-syncstorage](https://github.com/mozilla-services/server-syncstorage/) repo, you should start there.

2.  Install (Python) server-syncstorage:

         $ git clone https://github.com/mozilla-services/server-syncstorage/
         $ cd server-syncstorage
         $ make build

3.  Run an instance of syncstorage-rs (`cargo run` in this repo).

4.  To run all tests:

         $ ./local/bin/python syncstorage/tests/functional/test_storage.py http://localhost:8000#<SOMESECRET>

5.  Individual tests can be specified via the `SYNC_TEST_PREFIX` env var:

        $ SYNC_TEST_PREFIX=test_get_collection \
            ./local/bin/python syncstorage/tests/functional/test_storage.py http://localhost:8000#<SOMESECRET>

## Creating Releases

1. Switch to master branch of syncstorage-rs
1. `git pull` to ensure that the local copy is up-to-date.
1. `git pull origin master` to make sure that you've incorporated any changes to the master branch.
1. `git diff origin/master` to ensure that there are no local staged or uncommited changes.
1. Bump the version number in [Cargo.toml](https://github.com/mozilla-services/syncstorage-rs/blob/master/Cargo.toml) (this new version number will be designated as `<version>` in this checklist)
1. create a git branch for the new version `git checkout -b release/<version>`
1. `cargo build --release` - Build with the release profile [release mode](https://doc.rust-lang.org/book/ch14-01-release-profiles.html).
1. `clog -C CHANGELOG.md` - Generate release notes. We're using [clog](https://github.com/clog-tool/clog-cli) for release notes. Add a `-p`, `-m` or `-M` flag to denote major/minor/patch version, ie `clog -C CHANGELOG.md -p`.
1. Review the `CHANGELOG.md` file and ensure all relevant changes since the last tag are included.
1. Create a new [release in Sentry](https://docs.sentry.io/product/releases/#create-release): `VERSION={release-version-here} bash scripts/sentry-release.sh`. If you're doing this for the first time, checkout the [tips below](https://github.com/mozilla-services/syncstorage-rs#troubleshooting) for troubleshooting sentry cli access.
1. `git commit -am "chore: tag <version>"` to commit the new version and changes
1. `git tag -s -m "chore: tag <version>" <version>` to create a signed tag of the current HEAD commit for release.
1. `git push origin release/<version>` to push the commits to a new origin release branch
1. `git push --tags origin release/<version>` to push the tags to the release branch.
1. Submit a Pull Request (PR) on github to merge the release branch to master.
1. Go to the [GitHub release](https://github.com/mozilla-services/syncstorage-rs/releases), you should see the new tag with no release information.
1. Click the `Draft a new release` button.
1. Enter the \<version> number for `Tag version`.
1. Copy and paste the most recent change set from `CHANGELOG.md` into the release description, omitting the top 2 lines (the name and version)
1. Once your PR merges, click [Publish Release] on the [GitHub release](https://github.com/mozilla-services/syncstorage-rs/releases) page.

Sync server is automatically deployed to STAGE, however QA may need to be notified if testing is required. Once QA signs off, then a bug should be filed to promote the server to PRODUCTION.

## Troubleshooting

- `rm Cargo.lock; cargo clean;` - Try this if you're having problems compiling.

- Some versions of OpenSSL 1.1.1 can conflict with grpcio's built in BoringSSL. These errors can cause syncstorage to fail to run or compile.
If you see a problem related to `libssl` you may need to specify the `cargo` option `--features grpcio/openssl`  to force grpcio to use OpenSSL.

### Sentry

- If you're having trouble working with Sentry to create releases, try authenticating using their self hosted server option that's outlined [here](https://docs.sentry.io/product/cli/configuration/) Ie, `sentry-cli --url https://selfhosted.url.com/ login`. It's also recommended to create a `.sentryclirc` config file. See [this example](https://github.com/mozilla-services/syncstorage-rs/blob/master/.sentryclirc.example) for the config values you'll need.

## Related Documentation

- [API docs](https://mozilla-services.readthedocs.io/en/latest/storage/apis-1.5.html)

- [Code docs](https://github.com/mozilla-services/syncstorage-rs/tree/master/docs)

[mpl-svg]: https://img.shields.io/badge/License-MPL%202.0-blue.svg
[mpl]: https://opensource.org/licenses/MPL-2.0
[circleci-badge]: https://circleci.com/gh/mozilla-services/syncstorage-rs.svg?style=shield
[circleci]: https://circleci.com/gh/mozilla-services/syncstorage-rs
[matrix-badge]: https://img.shields.io/badge/chat%20on%20[m]-%23services%3Amozilla.org-blue
[matrix]: https://chat.mozilla.org/#/room/#services:mozilla.org
