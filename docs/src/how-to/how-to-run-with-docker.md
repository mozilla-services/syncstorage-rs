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

## Docker Compose, One-Shot with PostgreSQL

Alternatively, the database can be started through `docker compose` as well. The real service URL can be set with the `INIT_NODE_URL` environment variable.

Save the yaml below into a file, e.g. `docker-compose.one-shot.yaml`.

```yaml
services:
  syncserver:
    image: ghcr.io/mozilla-services/syncstorage-rs/syncserver-postgres:${SYNCSERVER_VERSION:-latest}
    platform: linux/amd64
    container_name: syncserver
    ports:
      - "8000:8000"
    environment:
      SYNC_HOST: "0.0.0.0"
      SYNC_PORT: "8000"
      SYNC_MASTER_SECRET: "${SYNC_MASTER_SECRET:-changeme_secret_key}"
      SYNC_SYNCSTORAGE__DATABASE_URL: "postgres://sync:sync@postgres:5432/syncserver"
      SYNC_TOKENSERVER__DATABASE_URL: "postgres://sync:sync@postgres:5432/syncserver"
      SYNC_TOKENSERVER__ENABLED: "true"
      SYNC_TOKENSERVER__RUN_MIGRATIONS: "true"
      SYNC_TOKENSERVER__FXA_EMAIL_DOMAIN: "api.accounts.firefox.com"
      SYNC_TOKENSERVER__FXA_OAUTH_SERVER_URL: "https://oauth.accounts.firefox.com"
      SYNC_HUMAN_LOGS: "${SYNC_HUMAN_LOGS:-false}"
      RUST_LOG: "${RUST_LOG:-info}"
      SYNC_TOKENSERVER__INIT_NODE_URL: "${SYNC_TOKENSERVER__INIT_NODE_URL:-http://localhost:${SYNC_PORT}}"
    depends_on:
      postgres:
        condition: service_healthy
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:${SYNC_PORT}/__heartbeat__"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 60s

  postgres:
    image: postgres:18
    container_name: syncserver-postgres
    environment:
      POSTGRES_USER: sync
      POSTGRES_PASSWORD: sync
      POSTGRES_DB: syncserver
    volumes:
      - postgres_data:/var/lib/postgresql
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U sync -d syncserver"]
      interval: 10s
      timeout: 5s
      retries: 5
      start_period: 30s
    restart: unless-stopped

volumes:
  postgres_data:
    driver: local
```

Next, start the service with `docker compose`:

```sh
docker compose -f docker-compose.one-shot.yaml up -d
```

## Docker Compose, One-Shot with MySQL

This recipe brings up everything needed for a working MySQL-backed server in a
single command: a MySQL database for Syncstorage and one for Tokenserver, plus
the server itself. Syncstorage applies its schema migrations automatically at
startup, `SYNC_TOKENSERVER__RUN_MIGRATIONS` applies the Tokenserver schema, and
`SYNC_TOKENSERVER__INIT_NODE_URL` bootstraps the `sync-1.5` service and storage
node records — so the stack is ready to serve with no manual database setup.

### Option A: build from source (works from a checkout)

Run this from a clone of the repository; the build `context` is the repo root,
so the MySQL build of the server is compiled locally and the recipe does not
depend on any published image. Save the yaml below into a file, e.g.
`docker-compose.one-shot.yaml`.

```yaml
services:
  syncserver:
    build:
      context: .
      args:
        SYNCSTORAGE_DATABASE_BACKEND: mysql
        TOKENSERVER_DATABASE_BACKEND: mysql
    container_name: syncserver
    ports:
      - "8000:8000"
    environment:
      SYNC_HOST: "0.0.0.0"
      SYNC_PORT: "8000"
      SYNC_MASTER_SECRET: "${SYNC_MASTER_SECRET:-changeme_secret_key}"
      SYNC_SYNCSTORAGE__DATABASE_URL: "mysql://sync:sync@sync-db:3306/syncstorage"
      SYNC_TOKENSERVER__DATABASE_URL: "mysql://sync:sync@tokenserver-db:3306/tokenserver"
      SYNC_TOKENSERVER__ENABLED: "true"
      SYNC_TOKENSERVER__NODE_TYPE: "mysql"
      SYNC_TOKENSERVER__RUN_MIGRATIONS: "true"
      SYNC_TOKENSERVER__FXA_EMAIL_DOMAIN: "api.accounts.firefox.com"
      SYNC_TOKENSERVER__FXA_OAUTH_SERVER_URL: "https://oauth.accounts.firefox.com"
      SYNC_HUMAN_LOGS: "${SYNC_HUMAN_LOGS:-false}"
      RUST_LOG: "${RUST_LOG:-info}"
      SYNC_TOKENSERVER__INIT_NODE_URL: "${SYNC_TOKENSERVER__INIT_NODE_URL:-http://localhost:${SYNC_PORT:-8000}}"
    depends_on:
      sync-db:
        condition: service_healthy
      tokenserver-db:
        condition: service_healthy
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/__heartbeat__"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 60s

  sync-db:
    image: docker.io/library/mysql:8.0
    container_name: syncserver-sync-db
    command: --explicit_defaults_for_timestamp
    environment:
      MYSQL_RANDOM_ROOT_PASSWORD: "yes"
      MYSQL_DATABASE: syncstorage
      MYSQL_USER: sync
      MYSQL_PASSWORD: sync
    volumes:
      - sync_db_data:/var/lib/mysql
    healthcheck:
      test: ["CMD-SHELL", "mysqladmin -h 127.0.0.1 -usync -psync ping"]
      interval: 10s
      timeout: 5s
      retries: 5
      start_period: 30s
    restart: unless-stopped

  tokenserver-db:
    image: docker.io/library/mysql:8.0
    container_name: syncserver-tokenserver-db
    command: --explicit_defaults_for_timestamp
    environment:
      MYSQL_RANDOM_ROOT_PASSWORD: "yes"
      MYSQL_DATABASE: tokenserver
      MYSQL_USER: sync
      MYSQL_PASSWORD: sync
    volumes:
      - tokenserver_db_data:/var/lib/mysql
    healthcheck:
      test: ["CMD-SHELL", "mysqladmin -h 127.0.0.1 -usync -psync ping"]
      interval: 10s
      timeout: 5s
      retries: 5
      start_period: 30s
    restart: unless-stopped

volumes:
  sync_db_data:
    driver: local
  tokenserver_db_data:
    driver: local
```

Next, build and start the service with `docker compose`:

```sh
docker compose -f docker-compose.one-shot.yaml up -d --build
```

Once the `syncserver` container reports healthy, confirm it is serving:

```sh
curl http://localhost:8000/__heartbeat__
```

### Option B: use a published image

Mozilla also publishes prebuilt images on ghcr.io. Note that these are
currently tagged by commit SHA — there is **no `latest` or semver tag** — so
you must pin `SYNCSERVER_VERSION` to a tag listed on the
[`syncstorage-rs-mysql` packages page](https://github.com/mozilla-services/syncstorage-rs/pkgs/container/syncstorage-rs%2Fsyncstorage-rs-mysql).
To use a published image, replace the `syncserver` service's `build:` block with
an `image:` reference; the `sync-db`, `tokenserver-db`, and `volumes` sections
are unchanged:

```yaml
services:
  syncserver:
    image: ghcr.io/mozilla-services/syncstorage-rs/syncstorage-rs-mysql:${SYNCSERVER_VERSION:?set SYNCSERVER_VERSION to a published tag}
    platform: linux/amd64
    container_name: syncserver
    # ...the remaining syncserver settings are identical to Option A
```

Then start it with the tag pinned (published images are `linux/amd64`):

```sh
SYNCSERVER_VERSION=<published-tag> docker compose -f docker-compose.one-shot.yaml up -d
```

> Set `SYNC_MASTER_SECRET` to your own value for anything beyond local
> experimentation; the default above is a placeholder.

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
