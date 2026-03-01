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

> **Note:** At the time of writing, there are no tagged release builds
> available on ghcr.io.  This guide will use a build from the main development
> branch.

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
    image: ghcr.io/mozilla-services/syncstorage-rs/syncstorage-rs-mysql:${SYNCSERVER_VERSION:-b16ef5064b}
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
      SYNC_TOKENSERVER__INIT_NODE_URL: "${SYNC_TOKENSERVER__INIT_NODE_URL:-http://localhost:8000}"
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/__heartbeat__"]
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
    image: ghcr.io/mozilla-services/syncstorage-rs/syncstorage-rs-postgres:${SYNCSERVER_VERSION:-11659d98f9}
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
      SYNC_TOKENSERVER__INIT_NODE_URL: "${SYNC_TOKENSERVER__INIT_NODE_URL:-http://localhost:8000}"
    depends_on:
      postgres:
        condition: service_healthy
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/__heartbeat__"]
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
SYNC_MASTER_SECRET=use_your_own_secret_4d3d3d3d \
SYNC_TOKENSERVER__INIT_NODE_URL=http://localhost:8000 \
docker compose -f docker-compose.one-shot.yaml up -d
```

## Configuring Firefox (Desktop)

Firefox itself needs to be configured to use the self-hosted Sync server.

1. Go to `about:config` in Firefox.
2. Find the `identity.sync.tokenserver.uri` configuration.
3. Change the value to `http://localhost:8000/1.0/sync/1.5`.
4. Restart Firefox.

Firefox should be using the self-hosted Sync server at this point.  That can be
verified by checking the logs in `about:sync-log`.

## Configuring Firefox (Mobile)

Firefox itself needs to be configured to use the self-hosted Sync server.

1. Go to Settings -> About Firefox
2. Repeadetly press the Firefox logo (six times) to activate the debug menu
3. Go back to the main Setting menu.
4. Click on the "Sync Debug" menu
5. Click on "custom sync server" and change the value to `http://localhost:8000/1.0/sync/1.5`.
6. After changing the "custom sync server" click on "Stop Firefox" in the same menu so the changes can be applied.
