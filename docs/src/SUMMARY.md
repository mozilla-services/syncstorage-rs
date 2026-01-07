<!-- NOTE: `mdbook build` will create documents if they're not present. It uses
     the path specified in the parenthesis. It has no idea about internal links
     so (foo.md#bar) will create a doc named "foo.md#bar".
-->

# Summary

[Introduction](introduction.md)
[Application Configuration](config.md)

[Application Architecture](architecture.md)

- [Tokenserver](tokenserver/tokenserver.md)
    - [Tokenserver API](tokenserver/tokenserver_api.md)
    - [Process Account Events](tools/process_account_events.md)
    - [Purge Old Records](tools/purge_old_records_tokenserver.md)
    - [Spanner Purge TTL](tools/spanner_purge_ttl.md)
    
[Documentation and MdBook Notes](doc-notes.md)
[Glossary of Terms](glossary.md)
[Response Codes](response-codes.md)
[Terms of Service](terms-of-service.md)