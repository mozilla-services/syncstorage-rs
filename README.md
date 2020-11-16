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

- cmake
- gcc
- [golang](https://golang.org/doc/install)
- libcurl4-openssl-dev
- libssl-dev
- make
- pkg-config
- [Rust stable](https://rustup.rs)
- MySQL 5.7 (or compatible)
  * libmysqlclient (`brew install mysql` on macOS, `apt install libmysqlclient-dev` on Ubuntu, `apt install libmariadb-dev-compat` on Debian)

Depending on your OS, you may also need to install `libgrpcdev`,
and `protobuf-compiler-grpc`. *Note*: if the code complies cleanly,
but generates a Segmentation Fault within Sentry init, you probably
are missing `libcurl4-openssl-dev`.

## Local Setup

1. Follow the instructions below to use either MySQL or Spanner as your DB.
2. Now `cp config/local.example.toml config/local.toml`. Open `config/local.toml` and make sure you have the desired settings configured. For a complete list of available configuration options, check out [docs/config.md](docs/config.md).
3. `make run` starts the server in debug mode, using your new `local.toml` file for config options. Or, simply `cargo run` with your own config options provided as env vars.
4. Visit `http://localhost:8000/__heartbeat__` to make sure the server is running.

### MySQL

Durable sync needs only a valid mysql DSN in order to set up connections to a MySQL database. The database can be local and is usually specified with a DSN like:

`mysql://_user_:_password_@_host_/_database_`

To setup a fresh MySQL DB and user: (`mysql -u root`):

```sql
CREATE USER "sample_user"@"localhost" IDENTIFIED BY "sample_password";
CREATE DATABASE syncstorage_rs;

GRANT ALL PRIVILEGES on syncstorage_rs.* to sample_user@localhost;
```

### Spanner

Spanner requires a key in order to access the database. It's important that you know which keys have access to the spanner database. Contact your administrator
to find out. One you know the key, log into the [Google Cloud Console Service Accounts](https://console.cloud.google.com/iam-admin/serviceaccounts) page. Be sure to
select the correct project.

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

Note, that unlike MySQL, there is no automatic migrations facility. Currently Spanner schema must be hand edited and modified.

To point to a GCP hosted Spanner instance from your local machine, follow these steps:

1. Download the key file as shown above.
2. Open `local.toml` and replace `database_url` with a link to your spanner instance.
3. Open the Makefile and ensure you've correctly set you `PATH_TO_GRPC_CERT`.
4. `make run_spanner`.
5. Visit `http://localhost:8000/__heartbeat__` to make sure the server is running.

### Running via Docker
This requires access to the mozilla-rust-sdk which is now available at `/vendor/mozilla-rust-adk`.

1. Make sure you have [Docker installed](https://docs.docker.com/install/) locally.
2. Copy the contents of mozilla-rust-sdk into top level root dir here.
3. Change cargo.toml mozilla-rust-sdk entry to point to `"path = "mozilla-rust-sdk/googleapis-raw"` instead of the parent dir.
4. Comment out the `image` value under `syncstorage-rs` in docker-compose.yml, and add this instead:
    ```yml
      build:
        context: .
    ```
5. Adjust the MySQL db credentials in docker-compose.yml to match your local setup.
6. `make docker_start` - You can verify it's working by visiting [localhost:8000/\_\_heartbeat\_\_](http://localhost:8000/__heartbeat__)

### Connecting to Firefox

This will walk you through the steps to connect this project to your local copy of Firefox.

1. Follow the steps outlined above for running this project using [MySQL](https://github.com/mozilla-services/syncstorage-rs#mysql).

2. Setup a local copy of [syncserver](https://github.com/mozilla-services/syncserver), with a few special changes to [syncserver.ini](https://github.com/mozilla-services/syncserver/blob/master/syncserver.ini); make sure that you're using the following values (in addition to all of the other defaults):

    ```ini
    [server:main]
    port = 5000

    [syncserver]
    public_url = http://localhost:5000/

    # This value needs to match your "master_secret" for syncstorage-rs!
    secret = INSERT_SECRET_KEY_HERE

    [tokenserver]
    node_url = http://localhost:8000
    sqluri = pymysql://sample_user:sample_password@127.0.0.1/syncstorage_rs

    [endpoints]
    sync-1.5 = "http://localhost:8000/1.5/1"```


3. In Firefox, go to `about:config`. Change `identity.sync.tokenserver.uri` to `http://localhost:5000/token/1.0/sync/1.5`.
4. Restart Firefox. Now, try syncing. You should see new BSOs in your local MySQL instance.

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

`make test` - open the Makefile to adjust your `SYNC_DATABASE_URL` as needed.

#### Debugging unit test state

In some cases, it is useful to inspect the mysql state of a failed test. By
default, we use the diesel test_transaction functionality to ensure test data
is not committed to the database. Therefore, there is an environment variable
which can be used to turn off test_transaction.

        SYNC_DATABASE_USE_TEST_TRANSACTIONS=false cargo test [testname]

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
1. Create a new [release in Sentry](https://docs.sentry.io/product/releases/#create-release): `bash scripts/sentry-release.sh`. If you're doing this for the first time, checkout the [tips below](https://github.com/mozilla-services/syncstorage-rs#troubleshooting) for troubleshooting sentry cli access.
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

- [Code docs](https://mozilla-services.github.io/syncstorage-rs/syncstorage/)

[mpl-svg]: https://img.shields.io/badge/License-MPL%202.0-blue.svg
[mpl]: https://opensource.org/licenses/MPL-2.0
[circleci-badge]: https://circleci.com/gh/mozilla-services/syncstorage-rs.svg?style=shield
[circleci]: https://circleci.com/gh/mozilla-services/syncstorage-rs
[matrix-badge]: https://img.shields.io/badge/chat%20on%20[m]-%23services%3Amozilla.org-blue
[matrix]: https://chat.mozilla.org/#/room/#services:mozilla.org
