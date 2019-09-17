[![License: MPL 2.0][mpl-svg]][mpl] [![Test Status][travis-badge]][travis] [![Build Status][circleci-badge]][circleci]

# Syncstorage-rs

Mozilla Sync Storage node built with [Rust](https://rust-lang.org).

API docs: https://mozilla-services.readthedocs.io/en/latest/storage/apis-1.5.html

Code docs: https://mozilla-services.github.io/syncstorage-rs/syncstorage/

Functional tests live in https://github.com/mozilla-services/server-syncstorage/
and can be run against a local server, e.g.:

## Requirements

 * Rust stable
 * MySQL 5.7 (or compatible)
   * libmysqlclient (`brew install mysql` on macOS, `apt-get install libmysqlclient-dev` on Ubuntu)

## Setting Up

1) [Install Rust](https://rustup.rs)

2) Create a `syncstorage` user/database. In the mysql shell:
    ```
    CREATE USER "syncstorage"@"localhost" IDENTIFIED BY "password";
    CREATE DATABASE syncstorage;`
    ```
3) Run:

        $ export SYNC_MASTER_SECRET=<SOMESECRET>
        $ export SYNC_DATABASE_URL=mysql://<YOUR_MYSQL_USERNAME>:<YOUR_MYSQL_PASSWORD>@localhost/syncstorage
        $ cargo run

## Running the Unit tests

1) Run:

        $ export RUST_TEST_THREADS=1
        $ cargo test

## Running the End-to-End tests

1) If you haven't already followed the instructions [here](https://mozilla-services.readthedocs.io/en/latest/howtos/run-sync-1.5.html) to get all the dependencies for the [server-syncstorage](https://github.com/mozilla-services/server-syncstorage/) repo, you should start there.

2) Install (Python) server-syncstorage:

        $ git clone https://github.com/mozilla-services/server-syncstorage/
        $ cd server-syncstorage
        $ make build

3) Run an instance of syncstorage-rs (see step 3 above).

4) Run:

        $ ./local/bin/python syncstorage/tests/functional/test_storage.py http://localhost:8000#<SOMESECRET>

Individual tests can be specified via the `SYNC_TEST_PREFIX` env var:

        $ SYNC_TEST_PREFIX=test_get_collection \
            ./local/bin/python syncstorage/tests/functional/test_storage.py http://localhost:8000#<SOMESECRET>


[mpl-svg]: https://img.shields.io/badge/License-MPL%202.0-blue.svg
[mpl]: https://opensource.org/licenses/MPL-2.0
[travis-badge]: https://travis-ci.org/mozilla-services/syncstorage-rs.svg?branch=master
[travis]: https://travis-ci.org/mozilla-services/syncstorage-rs
[circleci-badge]: https://circleci.com/gh/mozilla-services/syncstorage-rs.svg?style=shield
[circleci]: https://circleci.com/gh/mozilla-services/syncstorage-rs