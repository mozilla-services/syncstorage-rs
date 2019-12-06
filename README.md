<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->


- [Syncstorage-rs](#syncstorage-rs)
  - [System Requirements](#system-requirements)
  - [Local Setup](#local-setup)
    - [MySQL](#mysql)
    - [Spanner](#spanner)
  - [Logging](#logging)
  - [Tests](#tests)
    - [Unit tests](#unit-tests)
    - [End-to-End tests](#end-to-end-tests)
  - [Creating Releases](#creating-releases)
  - [Troubleshooting](#troubleshooting)
  - [Related Documentation](#related-documentation)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

[![License: MPL 2.0][mpl-svg]][mpl] [![Test Status][travis-badge]][travis] [![Build Status][circleci-badge]][circleci]

# Syncstorage-rs

Mozilla Sync Storage built with [Rust](https://rust-lang.org).

## System Requirements

- [Rust stable](https://rustup.rs)
- MySQL 5.7 (or compatible)
  -\* libmysqlclient (`brew install mysql` on macOS, `apt-get install libmysqlclient-dev` on Ubuntu)
- [Go](https://golang.org/doc/install)
- Cmake
- Pkg-config
- Openssl

Depending on your OS, you may also need to install `libgrpcdev`, and `protobuf-compiler-grpc`.

## Local Setup

1. Follow the instructions below to use either MySQL or Spanner as your DB.
2. Now `cp config/local.example.toml config/local.toml`. Open `config/local.toml` and make sure you have the desired settings configured. For a complete list of available configuration options, check out [docs/config.md](docs/config.md).
3. `make run_local` starts the server in debug mode, using your new `local.toml` file for config options. Or, simply `cargo run` with your own config options provided as env vars.
4. Visit `http://localhost:8000/__heartbeat__` to make sure the server is running.

### MySQL

Durable sync needs only a valid mysql DSN in order to set up connections to a MySQL database. The database can be local and is usually specified with a DSN like:

`mysql://_user_:_password_@_host_/_database_`

To setup a fresh MySQL DB and user: (`mysql -u root`):

    ```
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

A proper Key file will be downloaded to your local directory. It's important to safeguard that key file. For this example, we're going to name the file
`sync-spanner.json` and store it in a subdirectory called `./keys`

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

You can then specify the path to the key file using the environment variable `GOOGLE_APPLICATION_CREDENTIALS` when running the application.

e.g.

```bash
RUST_LOG=warn GOOGLE_APPLICATION_CREDENTIALS=`pwd`/keys/sync-spanner.json` cargo run -- --config sync.ini
```

Note, that unlike MySQL, there is no automatic migrations facility. Currently Spanner schema must be hand edited and modified.

## Logging

- If you want to connect to the existing [Sentry project](https://sentry.prod.mozaws.net/operations/syncstorage-dev/) for local development, login to Sentry, and go to the page with [api keys](https://sentry.prod.mozaws.net/settings/operations/syncstorage-dev/keys/). Copy the `DSN` value, and `export SENTRY_DSN=DSN_VALUE_GOES_HERE` to the environment when running this project.
- Using [env_logger](https://crates.io/crates/env_logger): set the `RUST_LOG` env var.

## Tests

### Unit tests

1. `cd db-tests`.
2. Pass along your `SYNC_DATABASE_URL` to the test runner. Ie:

```
SYNC_DATABASE_URL="mysql://sample_user:sample_password@localhost/syncstorage_rs" && /
RUST_TEST_THREADS=1 && /
cargo test
```

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

Open a PR after doing the following:

1. Bump the version number in [Cargo.toml](https://github.com/mozilla-services/syncstorage-rs/blob/master/Cargo.toml).
2. `cargo build --release` - Build with the release profile [release mode](https://doc.rust-lang.org/book/ch14-01-release-profiles.html).
3. `clog -C CHANGELOG.md` - Generate release notes. We're using [clog](https://github.com/clog-tool/clog-cli) for release notes. Add a `-p`, `-m` or `-M` flag to denote major/minor/patch version, ie `clog -C CHANGELOG.md -p`.

Once your PR merges, then go ahead and create an official [GitHub release](https://github.com/mozilla-services/syncstorage-rs/releases).


## Troubleshooting

- `rm Cargo.lock; cargo clean;` - Try this if you're having problems compiling.

## Related Documentation

- [API docs](https://mozilla-services.readthedocs.io/en/latest/storage/apis-1.5.html)

- [Code docs](https://mozilla-services.github.io/syncstorage-rs/syncstorage/)

[mpl-svg]: https://img.shields.io/badge/License-MPL%202.0-blue.svg
[mpl]: https://opensource.org/licenses/MPL-2.0
[travis-badge]: https://travis-ci.org/mozilla-services/syncstorage-rs.svg?branch=master
[travis]: https://travis-ci.org/mozilla-services/syncstorage-rs
[circleci-badge]: https://circleci.com/gh/mozilla-services/syncstorage-rs.svg?style=shield
[circleci]: https://circleci.com/gh/mozilla-services/syncstorage-rs
