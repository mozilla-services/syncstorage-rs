# Durable Sync Setup Guide

## General setup
 1) [Install Rust](https://rustup.rs) - Durable sync uses the *stable* branch, so no additional elements should be required.
 2) Install `libmysqlclient`, the mysql client development library 
    * macOS: `brew install mysql` 
    * ubuntu: `apt install libmysqlclient-dev`

## Configuration
Rust uses environment variables for a number of configuration options. Some of these include:

| variable | value | description |
| --- | --- | --- |
| **RUST_LOG** | *debug*, *info*, *warn*, *error* | minimum Rust error logging level |
| **RUST_TEST_THREADS** | 1  | maximum number of concurrent threads for testing. |

In addition, durable sync configuration options can either be specified as environment variables (prefixed with **SYNC_***) or in a configuration file using the `--config` option.

For example the following are equivalent:
```bash
$ SYNC_HOST=0.0.0.0 SYNC_MASTER_SECRET="SuperSikkr3t" SYNC_DATABASE_URL=mysql://scott:tiger@localhost/syncstorage cargo run
```

```bash
$ cat sync.ini
HOST=0.0.0.0
MASTER_SECRET=SuperSikkr3t
DATABASE_URL=mysql://scott:tiger@localhost/syncstorage
$ cargo run -- --config sync.ini
```

Options can be mixed between environment and configuration.

### Options
The following configuration options are avaialble.

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

## Mysql Integration

Durable sync needs only a valid mysql DSN in order to set up connections to a MySQL database. The database can be local and is usually specified with a DSN like:

mysql://_user_:_password_@_host_/_database_

## Setting up Spanner integration

Spanner requires a key in order to access the database. It's important that you know which keys have access to the spanner database. Contact your administrator 
to find out. One you know the key, log into the [Google Cloud Console Service Accounts](https://console.cloud.google.com/iam-admin/serviceaccounts) page. Be sure to 
select the correct project.

* Locate the email identifier of the access key and pick the vertical dot menu at the far right of the row. 
* Select "*Create Key*" from the pop-up menu.
* Select "JSON" from the Dialog Box.

A proper Key file will be downloaded to your local directroy. It's important to safeguard that key file. For this example, we're going to name the file 
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