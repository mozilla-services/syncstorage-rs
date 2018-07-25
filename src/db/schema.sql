-- why is collection_id before id in BSO?
CREATE TABLE bso (
  collection_id   INTEGER NOT NULL,
  id              VARCHAR(64) NOT NULL,

  sortindex       INTEGER DEFAULT 0,

  payload         TEXT NOT NULL DEFAULT '',
  payload_size    INTEGER NOT NULL DEFAULT 0,

  -- milliseconds since unix epoch. Sync 1.5 spec says it shoud
  -- be a float of seconds since epoch accurate to two decimal places
  -- convert it in the API response, but work with it as an int
  last_modified   INTEGER NOT NULL,

  expiry          INTEGER NOT NULL,

  PRIMARY KEY (collection_id, id)
);
-- speeds up search immensely. See issue #116
CREATE INDEX search_newer ON BSO (collection_id, last_modified);


CREATE TABLE collections (
  -- store as an integer to save some space
  id             INTEGER PRIMARY KEY ASC AUTOINCREMENT NOT NULL,
  name           VARCHAR(32) UNIQUE NOT NULL,

  last_modified  INTEGER NOT NULL DEFAULT 0
);
INSERT INTO collections (id, name) VALUES
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
        -- forces new collections to start at 100
        (99, "-push-");


-- stores batch uploads. BSOS should be text/newline of BSO json blobs
CREATE TABLE batches (
        id               INTEGER PRIMARY KEY ASC AUTOINCREMENT NOT NULL,
        collection_id    INTEGER NOT NULL,
        last_modified    INTEGER NOT NULL,
        bsos             TEXT NOT NULL DEFAULT ''
);


CREATE TABLE keyvalues (
        key      VARCHAR(32) PRIMARY KEY NOT NULL,
        value    VARCHAR(32) NOT NULL
);
INSERT INTO keyvalues (key, value) VALUES ("SCHEMA_VERSION", 0);


INSERT INTO collections (id, name) VALUES
        (12, "addresses"),
        (13, "creditcards");

-- begin using user_version to track schema changes
-- skip user_version=1 as that *should have been* set by 'SCHEMA_0'
PRAGMA user_version=2;
