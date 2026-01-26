# Use Docker to Deploy Your Own Sync Server

Mozilla publishes Docker images of its
[`syncstorage-rs`](https://github.com/mozilla-services/syncstorage-rs) builds
on ghcr.io. This guide provides a simple `docker compose` setup that can act as
a starting point to self-host Sync.

Images are available for both
[MySQL](https://github.com/mozilla-services/syncstorage-rs/pkgs/container/syncstorage-rs%2Fsyncstorage-rs-mysql)
and
[PostgreSQL](https://github.com/mozilla-services/syncstorage-rs/pkgs/container/syncstorage-rs%2Fsyncstorage-rs-postgres)
as the database.  The sample code will focus on MySQL.  Differences in
configuration or deployment steps will be noted. 

> **Note:** At the time of writing, there are no tagged release builds
> available on ghcr.io.  This guide will use a build from the main development
> branch.

## Prerequisites and Presumptions
- The reader has a MySQL or PostgreSQL database up and running.
- The reader is familiar with the command line interface and `docker`.
- The reader is going to use [Mozilla accounts](https://accounts.firefox.com/)
  for authentication and authorization.
- The service will be deployed at http://localhost:8000/.

## Docker Compose

Save the yaml below into a file, e.g. `docker-compose.yaml`.

```yaml
services:
  syncserver:
    image: ghcr.io/mozilla-services/syncstorage-rs/syncstorage-rs-mysql:b16ef5064b
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
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/__heartbeat__"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 60s
```

Note that multiple values will be read from the environment:
- `SYNC_MASTER_SECRET`: a secret used in cryptographic operationsk a passphrase or random character string, e.g. `use_your_own_secret_4d3d3d3d`
- `SYNC_SYNCSTORAGE__DATABASE_URL`: database URL for syncstorage, e.g. `mysql://sync:test@example.io/syncstorage` or `postgres://testo:@localhost/syncdb`
- `SYNC_TOKENSERVER__DATABASE_URL`: database URL for tokenserver, e.g. `mysql://sync:test@example.io/tokenserver` or `postgres://testo:@localhost/syncdb`

The values can be directly written into the yaml as well.

Next, start the service with `docker compose`:

```sh
SYNC_MASTER_SECRET=use_your_own_secret_4d3d3d3d \
SYNC_SYNCSTORAGE__DATABASE_URL="mysql://sync:test@example.io/syncstorage" \
SYNC_TOKENSERVER__DATABASE_URL="mysql://sync:test@example.io/tokenserver" \
docker compose -f docker-compose.yaml up -d
```

### Database Bootstrapping

After starting the service on a clean, uninitialized database, some bootstrapping records need to be inserted.

For MySQL, run
```sql
INSERT INTO tokenserver.services (service, pattern) VALUES ('sync-1.5', '{node}/1.5/{uid}');

INSERT INTO tokenserver.nodes (service, node, available, current_load, capacity, downed, backoff)
VALUES (
  (SELECT id FROM services WHERE service = 'sync-1.5'),
  'http://localhost:8000',
  1, 0, 1000, 0, 0
);
```

For PostgreSQL, run
```sql
INSERT INTO nodes (service, node, available, current_load, capacity, downed, backoff)
VALUES (
  (SELECT id FROM services WHERE service = 'sync-1.5'),
  'http://localhost:8000',
  1, 0, 1000, 0, 0
);
```

Note that `http://localhost:8000` above needs to be replaced with the actual
service URL.

Restart the service with 
```sh
docker compose -f docker-compose.yaml restart
```

## Configuring Firefox

Firefox itself needs to be configured to use the self-hosted Sync server.

1. Go to `about:config` in Firefox.
1. Find the `identity.sync.tokenserver.uri` configuration.
1. Change the value to `http://localhost:8000/1.0/sync/1.5`.
1. Restart Firefox.

Firefox should be using the self-hosted Sync server at this point.  That can be
verified by checking the logs in `about:sync-log`.
