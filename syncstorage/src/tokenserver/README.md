# Tokenserver

Tokenserver is used to:
1. Authenticate Sync clients via an OAuth token that clients receive from FxA
1. Direct Sync clients to the appropriate Sync Storage node
1. Present Sync clients with the encryption key necessary to access their Sync Storage nodes

This functionality was previously provided by a [Python service](https://github.com/mozilla-services/tokenserver/). Originally, the intent was to use Tokenserver as a standalone authentication service for use with various, independent microservices. In practice, it is only used for Firefox Sync, so it was rewritten in Rust to be part of the same code repository as the Sync Storage node.


<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->


- [Configuration](#configuration)
  - [Disabling Syncstorage](#disabling-syncstorage)
  - [Test Mode](#test-mode)
  - [Connecting to Firefox](#connecting-to-firefox)
- [Database](#database)
- [Running](#running)
- [Testing](#testing)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

## Configuration

You can find example settings for Tokenserver in [config/local.example.toml](../../config/local.example.toml). The available settings are described in doc comments [here](../../src/tokenserver/settings.rs).

### Disabling Syncstorage

Tokenserver can be run as a standalone service by disabling the Sync Storage endpoints. This can be done simply by setting the `disable_syncstorage` setting to `true`. **Note that the Sync Storage settings must still be set even when those endpoints are disabled.**

### Connecting to Firefox

1. Visit `about:config` in Firefox
1. Set `identity.sync.tokenserver.uri` to `http://localhost:8000/1.0/sync/1.5`

This will point Firefox to the Tokenserver running alongside your local instance of Sync Storage.

## Database

Prior to using Tokenserver, the migrations must be run. First, install the [diesel](https://diesel.rs/guides/getting-started) CLI tool:
```
cargo install diesel_cli
```
Then, run the migrations:
```
diesel --database-url mysql://sample_user:sample_password@localhost/tokenserver_rs migration --migration-dir src/tokenserver/migrations run
```
You should replace the above database Data Source Name (DSN) with the DSN of the database you are using.

## Running

Tokenserver is run alongside Sync Storage using `make run`.

## Testing
Tokenserver includes unit tests and a comprehensive suite of integration tests. These tests are run alongside the Sync Storage tests and can be run by following the instructions [here](../../README.md#unit-tests) and [here](../../README.md#end-to-end-tests).
