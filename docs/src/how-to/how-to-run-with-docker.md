# Use Docker to Deploy Your Own Sync Server

Mozilla publishes Docker images of its
[`syncstorage-rs`](https://github.com/mozilla-services/syncstorage-rs) builds
on ghcr.io. This guide provides a simple `docker compose` setup that can act as
a starting point to self-host Sync.

Images are available for both
[MySQL](https://github.com/mozilla-services/syncstorage-rs/pkgs/container/syncstorage-rs%2Fsyncstorage-rs-mysql)
and
[PostgreSQL](https://github.com/mozilla-services/syncstorage-rs/pkgs/container/syncstorage-rs%2Fsyncstorage-rs-postgres)
as the database.  Differences in configuration or deployment steps will be
noted.

Tagged release builds are available on ghcr.io. To pin to a specific version,
set `SYNCSERVER_VERSION` to the desired release tag (e.g., `SYNCSERVER_VERSION=v1.45.0`)
before running `docker compose`. Available releases can be found on the
[syncstorage-rs releases page](https://github.com/mozilla-services/syncstorage-rs/releases).
If `SYNCSERVER_VERSION` is not set, the compose files below default to `latest`.

## Prerequisites and Presumptions
- The reader is familiar with the command line interface and `docker`.
- The reader is going to use [Mozilla accounts](https://accounts.firefox.com/)
  for authentication and authorization.
- The service will be deployed at http://localhost:8000/.

## Docker Compose, Sync Services Only

With a MySQL or PostgreSQL database is already up and running, save the yaml
below into a file, e.g. `docker-compose.yaml`, and ensure the `image` field is
using the correct MySQL or PostgreSQL build for the database.

```yaml
services:
  syncserver:
    image: ghcr.io/mozilla-services/syncstorage-rs/syncstorage-rs-mysql:${SYNCSERVER_VERSION:-latest}
    platform: linux/amd64
    container_name: syncserver
    ports:
      - "8000:8000"
    environment:
      SYNC_HOST: "0.0.0.0"
      SYNC_PORT: "8000"
      SYNC_MASTER_SECRET: "${SYNC_MASTER_SECRET}"
      SYNC_SYNCSTORAGE__DATABASE_URL: "${SYNC_SYNCSTORAGE__DATABASE_URL}"
      SYNC_TOKENSERVER__DATABASE_URL: "${SYNC_TOKENSERVER__DATABASE_URL}"
      SYNC_TOKENSERVER__ENABLED: "true"
      SYNC_TOKENSERVER__RUN_MIGRATIONS: "true"
      SYNC_TOKENSERVER__FXA_EMAIL_DOMAIN: "api.accounts.firefox.com"
      SYNC_TOKENSERVER__FXA_OAUTH_SERVER_URL: "https://oauth.accounts.firefox.com"
      SYNC_TOKENSERVER__INIT_NODE_URL: "${SYNC_TOKENSERVER__INIT_NODE_URL:-http://localhost:${SYNC_PORT}}"
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:${SYNC_PORT}/__heartbeat__"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 60s
```

Note that multiple values will be read from the environment:
- [`SYNC_MASTER_SECRET`](../config.md#SYNC_MASTER_SECRET): a secret used in cryptographic operations, a passphrase or random character string, e.g. `use_your_own_secret_4d3d3d3d`
- [`SYNC_SYNCSTORAGE__DATABASE_URL`](../config.md#SYNC_SYNCSTORAGE__DATABASE_URL): database URL for syncstorage, e.g. `mysql://sync:test@example.io/syncstorage` or `postgres://testo:@localhost/syncdb`
- [`SYNC_TOKENSERVER__DATABASE_URL`](../config.md#SYNC_TOKENSERVER__DATABASE_URL): database URL for tokenserver, e.g. `mysql://sync:test@example.io/tokenserver` or `postgres://testo:@localhost/syncdb`
- [`SYNC_TOKENSERVER__INIT_NODE_URL`](../config.md#SYNC_TOKENSERVER__INIT_NODE_URL): the storage node URL (defaults to `http://localhost:8000`).  Replace with the actual URL where clients will access the sync server.

The values can be directly written into the yaml as well.

Next, start the service with `docker compose`:

```sh
SYNC_MASTER_SECRET=use_your_own_secret_4d3d3d3d \
SYNC_SYNCSTORAGE__DATABASE_URL="mysql://sync:test@example.io/syncstorage" \
SYNC_TOKENSERVER__DATABASE_URL="mysql://sync:test@example.io/tokenserver" \
SYNC_TOKENSERVER__INIT_NODE_URL="http://localhost:8000" \
docker compose -f docker-compose.yaml up -d
```

## Docker Compose, One-Shot Stand-Alone Servers

The repository ships ready-to-run, stand-alone compose files under `docker/`
that bring up a complete server — database(s) included — in a single command,
with no manual database setup. Each builds the server from your local checkout,
so they work directly from a clone without a published image. From the repo
root:

| Backend | Make target | Compose file |
|---|---|---|
| MySQL | `make docker_oneshot_mysql` | `docker/docker-compose.one-shot.mysql.yaml` |
| PostgreSQL | `make docker_oneshot_postgres` | `docker/docker-compose.one-shot.postgres.yaml` |
| Spanner (emulator, local dev only) | `make docker_oneshot_spanner` | `docker/docker-compose.one-shot.spanner.yaml` |

For example, for MySQL:

```sh
make docker_oneshot_mysql
# equivalently:
docker compose -f docker/docker-compose.one-shot.mysql.yaml up -d --build
```

Once the `syncserver` container reports healthy, confirm it is serving:

```sh
curl http://localhost:8000/__heartbeat__
```

Stop and remove a stack with the matching `_stop` target, e.g.
`make docker_oneshot_mysql_stop`.

Syncstorage applies its schema migrations at startup,
`SYNC_TOKENSERVER__RUN_MIGRATIONS` applies the Tokenserver schema, and
`SYNC_TOKENSERVER__INIT_NODE_URL` bootstraps the `sync-1.5` service and storage
node records — so the stack is ready to serve immediately.

> Set `SYNC_MASTER_SECRET` to your own value for anything beyond local
> experimentation; the compose files default to a placeholder.

### Backend notes

- **MySQL / PostgreSQL** are reasonable starting points for a real self-hosted
  deployment. The MySQL recipe uses a separate database for Syncstorage and
  Tokenserver; the PostgreSQL recipe shares a single database between them for
  simplicity.
- **Spanner** runs against the Cloud Spanner *emulator* and is for local
  experimentation only — it is unauthenticated, single-node, and not durable.
  Production Spanner uses a real instance and a service-account key (see
  `make run_spanner`). The recipe mirrors the production split (Spanner for
  Syncstorage, MySQL for Tokenserver) and includes a one-time setup container
  that provisions the emulator's schema before the server starts.

### Using a published image instead of building

Mozilla also publishes prebuilt images on ghcr.io. The MySQL images are
currently tagged by commit SHA — there is **no `latest` or semver tag** — so you
must pin `SYNCSERVER_VERSION` to a tag listed on the
[`syncstorage-rs-mysql` packages page](https://github.com/mozilla-services/syncstorage-rs/pkgs/container/syncstorage-rs%2Fsyncstorage-rs-mysql).
To use one, replace the `syncserver` service's `build:` block in the compose
file with an `image:` reference (published images are `linux/amd64`):

```yaml
services:
  syncserver:
    image: ghcr.io/mozilla-services/syncstorage-rs/syncstorage-rs-mysql:${SYNCSERVER_VERSION:?set SYNCSERVER_VERSION to a published tag}
    platform: linux/amd64
    # ...the remaining syncserver settings are unchanged
```

```sh
SYNCSERVER_VERSION=<published-tag> docker compose -f docker/docker-compose.one-shot.mysql.yaml up -d
```

## Configuring Firefox (Desktop)

Firefox itself needs to be configured to use the self-hosted Sync server.

1. Go to `about:config` in Firefox.
1. Find the `identity.sync.tokenserver.uri` configuration.
1. Change the value to `http://localhost:8000/1.0/sync/1.5`.
1. Restart Firefox.

Firefox should be using the self-hosted Sync server at this point.  That can be
verified by checking the logs in `about:sync-log`.

## Configuring Firefox (Mobile)

Firefox itself needs to be configured to use the self-hosted Sync server.

1. Go to Settings -> About Firefox
1. Repeadetly press the Firefox logo (six times) to activate the debug menu
1. Go back to the main Setting menu.
1. Click on the "Sync Debug" menu
1. Click on "custom sync server" and change the value to `http://localhost:8000/1.0/sync/1.5`.
1. After changing the "custom sync server" click on "Stop Firefox" in the same menu so the changes can be applied.
