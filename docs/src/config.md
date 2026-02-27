# Configuration
Rust uses environment variables for a number of configuration options. Some of these include:

| variable | value | description |
| --- | --- | --- |
| **RUST_LOG** | *debug*, *info*, *warn*, *error* | minimum Rust error logging level |
| **RUST_TEST_THREADS** | 1  | maximum number of concurrent threads for testing. |

In addition, Sync server configuration options can either be specified as environment variables (prefixed with **SYNC_***) or in a configuration file using the `--config` option.

For example the following are equivalent:
```bash
$ SYNC_HOST=0.0.0.0 SYNC_MASTER_SECRET="SuperSikkr3t" SYNC_SYNCSTORAGE__DATABASE_URL=mysql://scott:tiger@localhost/syncstorage cargo run
```

```bash
$ cat syncstorage.local.toml
host = "0.0.0.0"
master_secret = "SuperSikkr3t"

[syncstorage]
database_url = "mysql://scott:tiger@localhost/syncstorage"
$ cargo run -- --config syncstorage.local.toml
```

Options can be mixed between environment variables and configuration.  Environment variables have higher precedence.

## Options
The following configuration options are available.

### Server Settings

| Env Var | Default Value | Description |
| --- | --- | --- |
| <span id="SYNC_HOST"></span>SYNC_HOST | 127.0.0.1 | Host address to bind the server to |
| <span id="SYNC_PORT"></span>SYNC_PORT | 8000 | Server port to bind to |
| <span id="SYNC_MASTER_SECRET"></span>SYNC_MASTER_SECRET | None, required | Secret used to derive auth secrets |
| <span id="SYNC_ENVIRONMENT"></span>SYNC_ENVIRONMENT | dev | Environment name ("dev", "stage", "prod") |
| <span id="SYNC_HUMAN_LOGS"></span>SYNC_HUMAN_LOGS | false | Enable human-readable logs |
| <span id="SYNC_ACTIX_KEEP_ALIVE"></span>SYNC_ACTIX_KEEP_ALIVE | None | HTTP keep-alive header value in seconds |
| <span id="SYNC_WORKER_MAX_BLOCKING_THREADS"></span>SYNC_WORKER_MAX_BLOCKING_THREADS | 512 | The maximum number of blocking threads in the worker threadpool. This threadpool is used by Actix-web to handle blocking operations. |

### CORS

| Env Var | Default Value | Description |
| --- | --- | --- |
| <span id="SYNC_CORS_ALLOWED_ORIGIN"></span>SYNC_CORS_ALLOWED_ORIGIN | * | Allowed origins for CORS requests |
| <span id="SYNC_CORS_MAX_AGE"></span>SYNC_CORS_MAX_AGE | 1728000 | CORS preflight cache seconds (20 days) |
| <span id="SYNC_CORS_ALLOWED_METHODS"></span>SYNC_CORS_ALLOWED_METHODS | ["DELETE", "GET", "POST", "PUT"] | Allowed methods |

### Syncstorage Database

| Env Var | Default Value | Description |
| --- | --- | --- |
| <span id="SYNC_SYNCSTORAGE__DATABASE_URL"></span>SYNC_SYNCSTORAGE__DATABASE_URL | mysql://root@127.0.0.1/syncstorage | Database connection URL |
| <span id="SYNC_SYNCSTORAGE__DATABASE_POOL_MAX_SIZE"></span>SYNC_SYNCSTORAGE__DATABASE_POOL_MAX_SIZE | 10 | Max database connections |
| <span id="SYNC_SYNCSTORAGE__DATABASE_POOL_CONNECTION_TIMEOUT"></span>SYNC_SYNCSTORAGE__DATABASE_POOL_CONNECTION_TIMEOUT | 30 | Pool timeout in seconds |
| <span id="SYNC_SYNCSTORAGE__DATABASE_POOL_CONNECTION_LIFESPAN"></span>SYNC_SYNCSTORAGE__DATABASE_POOL_CONNECTION_LIFESPAN | None | Max connection age in seconds |
| <span id="SYNC_SYNCSTORAGE__DATABASE_POOL_CONNECTION_MAX_IDLE"></span>SYNC_SYNCSTORAGE__DATABASE_POOL_CONNECTION_MAX_IDLE | None | Max idle time in seconds |
| <span id="SYNC_SYNCSTORAGE__DATABASE_POOL_SWEEPER_TASK_INTERVAL"></span>SYNC_SYNCSTORAGE__DATABASE_POOL_SWEEPER_TASK_INTERVAL | 30 | How often, in seconds, a background task runs to evict idle database connections (Spanner only) |
| <span id="SYNC_SYNCSTORAGE__DATABASE_SPANNER_ROUTE_TO_LEADER"></span>SYNC_SYNCSTORAGE__DATABASE_SPANNER_ROUTE_TO_LEADER | false | Send leader-aware headers to Spanner |
| <span id="SYNC_SYNCSTORAGE__SPANNER_EMULATOR_HOST"></span>SYNC_SYNCSTORAGE__SPANNER_EMULATOR_HOST | None | Spanner emulator host (e.g., localhost:9010) |

### Syncstorage Limits

| Env Var | Default Value | Description |
| --- | --- | --- |
| <span id="SYNC_SYNCSTORAGE__LIMITS__MAX_POST_BYTES"></span>SYNC_SYNCSTORAGE__LIMITS__MAX_POST_BYTES | 2,621,440 | Max BSO payload size per request |
| <span id="SYNC_SYNCSTORAGE__LIMITS__MAX_POST_RECORDS"></span>SYNC_SYNCSTORAGE__LIMITS__MAX_POST_RECORDS | 100 | Max BSO count per request |
| <span id="SYNC_SYNCSTORAGE__LIMITS__MAX_RECORD_PAYLOAD_BYTES"></span>SYNC_SYNCSTORAGE__LIMITS__MAX_RECORD_PAYLOAD_BYTES | 2,621,440 | Max individual BSO payload size |
| <span id="SYNC_SYNCSTORAGE__LIMITS__MAX_REQUEST_BYTES"></span>SYNC_SYNCSTORAGE__LIMITS__MAX_REQUEST_BYTES | 2,625,536 | Max Content-Length for requests |
| <span id="SYNC_SYNCSTORAGE__LIMITS__MAX_TOTAL_BYTES"></span>SYNC_SYNCSTORAGE__LIMITS__MAX_TOTAL_BYTES | 262,144,000 | Max BSO payload size per batch |
| <span id="SYNC_SYNCSTORAGE__LIMITS__MAX_TOTAL_RECORDS"></span>SYNC_SYNCSTORAGE__LIMITS__MAX_TOTAL_RECORDS | 10,000 | Max BSO count per batch |
| <span id="SYNC_SYNCSTORAGE__LIMITS__MAX_QUOTA_LIMIT"></span>SYNC_SYNCSTORAGE__LIMITS__MAX_QUOTA_LIMIT | 2,147,483,648 | Max storage quota per user (2 GB) |

### Syncstorage Features

| Env Var | Default Value | Description |
| --- | --- | --- |
| <span id="SYNC_SYNCSTORAGE__ENABLED"></span>SYNC_SYNCSTORAGE__ENABLED | true | Enable syncstorage service |
| <span id="SYNC_SYNCSTORAGE__ENABLE_QUOTA"></span>SYNC_SYNCSTORAGE__ENABLE_QUOTA | false | Enable quota tracking (Spanner only) |
| <span id="SYNC_SYNCSTORAGE__ENFORCE_QUOTA"></span>SYNC_SYNCSTORAGE__ENFORCE_QUOTA | false | Enforce quota limits (Spanner only) |
| <span id="SYNC_SYNCSTORAGE__GLEAN_ENABLED"></span>SYNC_SYNCSTORAGE__GLEAN_ENABLED | true | Enable Glean telemetry |
| <span id="SYNC_SYNCSTORAGE__STATSD_LABEL"></span>SYNC_SYNCSTORAGE__STATSD_LABEL | syncstorage | StatsD metrics label prefix |

### Tokenserver Database

| Env Var | Default Value | Description |
| --- | --- | --- |
| <span id="SYNC_TOKENSERVER__DATABASE_URL"></span>SYNC_TOKENSERVER__DATABASE_URL | mysql://root@127.0.0.1/tokenserver | Tokenserver database URL |
| <span id="SYNC_TOKENSERVER__DATABASE_POOL_MAX_SIZE"></span>SYNC_TOKENSERVER__DATABASE_POOL_MAX_SIZE | 10 | Max tokenserver DB connections |
| <span id="SYNC_TOKENSERVER__DATABASE_POOL_CONNECTION_TIMEOUT"></span>SYNC_TOKENSERVER__DATABASE_POOL_CONNECTION_TIMEOUT | 30 | Pool timeout in seconds |

### Tokenserver Features

| Env Var | Default Value | Description |
| --- | --- | --- |
| <span id="SYNC_TOKENSERVER__INIT_NODE_URL"></span>SYNC_TOKENSERVER__INIT_NODE_URL | None | The storage node URL, protocol + host, to insert into the `nodes` table on startup. This is the origin where the service is hosted, e.g. "http://localhost:8000". |
| <span id="SYNC_TOKENSERVER__INIT_NODE_CAPACITY"></span>SYNC_TOKENSERVER__INIT_NODE_CAPACITY | 100000 | The storage node capacity of the server specified by `SYNC_TOKENSERVER__INIT_NODE_URL`. Only used if `SYNC_TOKENSERVER__INIT_NODE_URL` is set. |
| <span id="SYNC_TOKENSERVER__ENABLED"></span>SYNC_TOKENSERVER__ENABLED | false | Enable tokenserver service |
| <span id="SYNC_TOKENSERVER__RUN_MIGRATIONS"></span>SYNC_TOKENSERVER__RUN_MIGRATIONS | false | Run DB migrations on startup |
| <span id="SYNC_TOKENSERVER__NODE_TYPE"></span>SYNC_TOKENSERVER__NODE_TYPE | spanner | Storage backend type reported in token response for telemetry. Valid values: "mysql", "postgres", "spanner" |
| <span id="SYNC_TOKENSERVER__TOKEN_DURATION"></span>SYNC_TOKENSERVER__TOKEN_DURATION | 3600 | Token TTL (1 hour) |

### Tokenserver+FxA Integration

| Env Var | Default Value | Description |
| --- | --- | --- |
| <span id="SYNC_TOKENSERVER__FXA_EMAIL_DOMAIN"></span>SYNC_TOKENSERVER__FXA_EMAIL_DOMAIN | api-accounts.stage.mozaws.net | FxA email domain |
| <span id="SYNC_TOKENSERVER__FXA_OAUTH_SERVER_URL"></span>SYNC_TOKENSERVER__FXA_OAUTH_SERVER_URL | https://oauth.stage.mozaws.net | FxA OAuth server URL |
| <span id="SYNC_TOKENSERVER__FXA_OAUTH_REQUEST_TIMEOUT"></span>SYNC_TOKENSERVER__FXA_OAUTH_REQUEST_TIMEOUT | 10 | OAuth request timeout in seconds |
| <span id="SYNC_TOKENSERVER__FXA_METRICS_HASH_SECRET"></span>SYNC_TOKENSERVER__FXA_METRICS_HASH_SECRET | secret | Secret for hashing metrics to maintain anonymity |
| <span id="SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__KTY"></span>SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__KTY | None | Primary JWK key type (e.g., "RSA") |
| <span id="SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__ALG"></span>SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__ALG | None | Primary JWK algorithm (e.g., "RS256") |
| <span id="SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__KID"></span>SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__KID | None | Primary JWK key ID |
| <span id="SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__FXA_CREATED_AT"></span>SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__FXA_CREATED_AT | None | Primary JWK creation timestamp |
| <span id="SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__USE"></span>SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__USE | None | Primary JWK use (e.g., "sig") |
| <span id="SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__N"></span>SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__N | None | Primary JWK modulus (RSA public key component) |
| <span id="SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__E"></span>SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__E | None | Primary JWK exponent (RSA public key component) |
| <span id="SYNC_TOKENSERVER__FXA_OAUTH_SECONDARY_JWK"></span>SYNC_TOKENSERVER__FXA_OAUTH_SECONDARY_JWK__* | None | Secondary JWK (same structure as primary) |

### StatsD Metrics

| Env Var | Default Value | Description |
| --- | --- | --- |
| <span id="SYNC_STATSD_HOST"></span>SYNC_STATSD_HOST | localhost | StatsD server hostname |
| <span id="SYNC_STATSD_PORT"></span>SYNC_STATSD_PORT | 8125 | StatsD server port |
| <span id="SYNC_INCLUDE_HOSTNAME_TAG"></span>SYNC_INCLUDE_HOSTNAME_TAG | false | Include hostname in metrics tags |

