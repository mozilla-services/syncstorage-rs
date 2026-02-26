<!-- NOTE: `mdbook build` will create documents if they're not present. It uses
     the path specified in the parenthesis. It has no idea about internal links
     so (foo.md#bar) will create a doc named "foo.md#bar".
-->

# Summary

- [Introduction](introduction.md)
- [Application Architecture](architecture.md)
- [Application Configuration](config.md)
- [Frequently Asked Questions](faq.md)
- [Data Types](data-types.md)
- [OpenAPI Documentation](open-api-docs.md)
- [Syncstorage API](syncstorage/api.md)
    - [API v1.5](syncstorage/api-1.5.md)
    - [API v1.1 (Obsolete)](syncstorage/api-1.1.md)
    - [API v1.0 (Obsolete)](syncstorage/api-1.0.md)
- [Syncstorage DB - Postgres](syncstorage/syncstorage-postgres-db.md)
- [Tokenserver](tokenserver/tokenserver.md)
    - [Goals of Tokenserver](tokenserver/tokenserver-goals.md)
    - [Tokenserver API](tokenserver/tokenserver-api.md)
    - [Tokenserver DB - Postgres](tokenserver/tokenserver-db-postgres.md)
    - [User Flow](tokenserver/user-flow.md)
    - [Process Account Events](tools/process_account_events.md)
    - [Purge Old Records](tools/purge_old_records_tokenserver.md)
    - [Spanner Purge TTL](tools/spanner_purge_ttl.md)
- [Sync Client](sync-client/overview.md)
    - [Life of a Sync](sync-client/life-of-a-sync.md)
    - [Sync Storage Formats](sync-client/sync-storage-formats.md)
    - [Global Storage Version 5](sync-client/global-storage-v5.md)
    - [Firefox object formats](sync-client/fx-object-formats.md)
[Mozilla Accounts Server - FxA](mozilla-accounts.md)

- [How To Guides](how-to/index.md)
    - [Run Your Own Sync-1.5 Server with Docker](how-to/how-to-run-with-docker.md)
    - [Run Your Own Sync-1.5 Server (legacy)](how-to/how-to-run-sync-server.md)
    - [Configure Sync Server for TLS (legacy)](how-to/how-to-config-tls.md)

[Documentation and MdBook Notes](mdbook-doc-notes.md)
[Glossary of Terms](glossary.md)
[Response Codes](response-codes.md)
[Terms of Service](terms-of-service.md)
