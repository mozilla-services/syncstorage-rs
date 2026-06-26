# Default Configuration for Spanner & MySQL Builds

This page describes the out-of-the-box configuration needed to run the two
standard `syncstorage-rs` builds:

- **MySQL build** — the default `cargo build` / `make run_mysql` target and the
  `syncstorage-rs-mysql` Docker image. Both Syncstorage and Tokenserver run
  against MySQL.
- **Spanner build** — `make run_spanner` and the `syncstorage-rs-spanner`
  Docker image. This mirrors production: Syncstorage runs against Google Cloud
  Spanner while Tokenserver runs against MySQL.

Annotated, copy-paste-ready templates live in the repo:

| Build | Template |
| --- | --- |
| MySQL | [`config/local.example.toml`](https://github.com/mozilla-services/syncstorage-rs/blob/master/config/local.example.toml) |
| Spanner | [`config/local.example.spanner.toml`](https://github.com/mozilla-services/syncstorage-rs/blob/master/config/local.example.spanner.toml) |

Copy one to `config/local.toml` and edit the values marked `REQUIRED`:

```sh
cp config/local.example.toml config/local.toml          # MySQL
# or
cp config/local.example.spanner.toml config/local.toml  # Spanner
```

Every setting can also be supplied as an environment variable prefixed with
`SYNC_` (nested keys use `__`). For example `syncstorage.database_url` becomes
`SYNC_SYNCSTORAGE__DATABASE_URL`. Environment variables take precedence over the
config file. The complete list of options and their defaults is in the
[Application Configuration](../config.md) reference, and the source of truth is
the doc-commented `Settings` structs in the `*-settings` crates.

## Minimum required settings

Most settings have sensible defaults. Regardless of backend you must supply:

| Setting | Why |
| --- | --- |
| [`master_secret`](../config.md#SYNC_MASTER_SECRET) | Derives the Hawk signing/token secrets. No default. |
| [`syncstorage.database_url`](../config.md#SYNC_SYNCSTORAGE__DATABASE_URL) | No usable default; the server fails fast at startup if unset. |
| [`tokenserver.database_url`](../config.md#SYNC_TOKENSERVER__DATABASE_URL) | Required when `tokenserver.enabled = true` (fails fast if unset). |

Backend-specific notes:

- **MySQL:** set `tokenserver.node_type = "mysql"`. Syncstorage MySQL schema
  migrations run automatically at startup; set
  `tokenserver.run_migrations = true` to apply the Tokenserver schema too.
- **Spanner:** `syncstorage.database_url` must use the
  `spanner://projects/.../instances/.../databases/...` form — the `spanner://`
  scheme is what selects the Spanner backend. Leave `tokenserver.node_type` at
  its default (`"spanner"`). To run against the local emulator instead of GCP,
  set `syncstorage.spanner_emulator_host` (e.g. `localhost:9010`); for real
  Spanner, point `GOOGLE_APPLICATION_CREDENTIALS` at a service-account key.

## Run it out of the box with Docker

For a zero-to-running MySQL stack (database + server, schema applied
automatically, ready to serve), see the
[one-shot MySQL `docker compose`](how-to-run-with-docker.md#docker-compose-one-shot-with-mysql)
recipe. It brings up MySQL and the server, runs migrations, and bootstraps the
storage node so that `curl http://localhost:8000/__heartbeat__` succeeds with no
further setup.

A Spanner stack cannot be fully zero-config: real Spanner requires GCP
credentials, and the local emulator requires extra wiring (see
`docker/docker-compose.spanner.yaml` and `make run_spanner`).
