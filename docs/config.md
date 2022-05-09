# Configuration
Rust uses environment variables for a number of configuration options. Some of these include:

| variable | value | description |
| --- | --- | --- |
| **RUST_LOG** | *debug*, *info*, *warn*, *error* | minimum Rust error logging level |
| **RUST_TEST_THREADS** | 1  | maximum number of concurrent threads for testing. |

In addition, durable sync configuration options can either be specified as environment variables (prefixed with **SYNC_***) or in a configuration file using the `--config` option.

For example the following are equivalent:
```bash
$ SYNC_HOST=0.0.0.0 SYNC_MASTER_SECRET="SuperSikkr3t" SYNC_SYNCSTORAGE__DATABASE_URL=mysql://scott:tiger@localhost/syncstorage cargo run
```

```bash
$ cat sync.ini
HOST=0.0.0.0
MASTER_SECRET=SuperSikkr3t

[syncstorage]
DATABASE_URL=mysql://scott:tiger@localhost/syncstorage
$ cargo run -- --config sync.ini
```

Options can be mixed between environment and configuration.

## Options
The following configuration options are available.

| Option | Default value |Description |
| --- | --- | --- |
| debug | false | _unused_ |
| port | 8000 | connection port |
| host | 127.0.0.1 | host to listen for connections |
| database_url | mysql://root@127.0.0.1/syncstorage | database DSN |
| database_pool_max_size | _None_ | Max pool of database connections |
| master_secret| _None_ |  Sync master encryption secret |
| limits.max_post_bytes | 2,097,152‬ | Largest record post size | 
| limits.max_post_records | 100 | Largest number of records per post | 
| limits.max_records_payload_bytes | 2,097,152‬ | Largest ... | 
| limits.max_request_bytes | 2,101,248 | Largest ... |
| limits.max_total_bytes | 209,715,200 | Largest ... |
| limits.max_total_records | 100,000 | Largest ... |

