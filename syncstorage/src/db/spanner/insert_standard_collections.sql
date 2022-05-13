-- These are the 13 standard collections that are expected to exist by clients.
-- The IDs are fixed. The below statement can be used to add these collections
-- to a Spanner instance.
INSERT INTO collections (collection_id, name) VALUES
    ( 1, "clients"),
    ( 2, "crypto"),
    ( 3, "forms"),
    ( 4, "history"),
    ( 5, "keys"),
    ( 6, "meta"),
    ( 7, "bookmarks"),
    ( 8, "prefs"),
    ( 9, "tabs"),
    (10, "passwords"),
    (11, "addons"),
    (12, "addresses"),
    (13, "creditcards");
