<!-- NOTE: `mdbook build` will create documents if they're not present. It uses
     the path specified in the parenthesis. It has no idea about internal links
     so (foo.md#bar) will create a doc named "foo.md#bar".
-->

# Summary

[Introduction](introduction.md)
[Application Configuration](config.md)
[Application Architecture](architecture.md)
[Mozilla Accounts Server - FxA](mozilla-accounts.md)

- [How To Guides](how-to/index.md)
    - [Run Your Own FxA Server](how-to/how-to-run-fxa.md)
    - [Run Your Own Sync-1.5 Server (legacy)](how-to/how-to-run-sync-server.md)
    - [Configure Sync Server for TLS (legacy)](how-to/how-to-config-tls.md)
- [Styncstorage](syncstorage/api.md)
- [Tokenserver](tokenserver/tokenserver.md)
    - [Goals of Tokenserver](tokenserver/tokenserver-goals.md)
    - [Tokenserver API](tokenserver/tokenserver-api.md)
    - [User Flow](tokenserver/user-flow.md)
    - [Process Account Events](tools/process_account_events.md)
    - [Purge Old Records](tools/purge_old_records_tokenserver.md)
    - [Spanner Purge TTL](tools/spanner_purge_ttl.md)

[Documentation and MdBook Notes](doc-notes.md)
[Glossary of Terms](glossary.md)
[Response Codes](response-codes.md)
[Terms of Service](terms-of-service.md)