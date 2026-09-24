-- XXX: bsov1, etc
CREATE TABLE IF NOT EXISTS bso
(
    userid       BIGINT  NOT NULL,
    collection   INTEGER NOT NULL,
    id           TEXT    NOT NULL,

    sortindex    INTEGER,

    payload      TEXT    NOT NULL,
    payload_size BIGINT  DEFAULT 0,

    -- last modified time in milliseconds since epoch
    modified     BIGINT  NOT NULL,
    -- expiration in milliseconds since epoch
    ttl          BIGINT  NOT NULL DEFAULT 3153600000000,

    PRIMARY KEY (userid, collection, id)
);
CREATE INDEX IF NOT EXISTS bso_expiry_idx ON bso (ttl);
CREATE INDEX IF NOT EXISTS bso_usr_col_mod_idx ON bso (userid, collection, modified);

CREATE TABLE IF NOT EXISTS collections
(
    id   INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT UNIQUE NOT NULL
);
INSERT INTO collections (id, name) VALUES
    (1, 'clients'),
    (2, 'crypto'),
    (3, 'forms'),
    (4, 'history'),
    (5, 'keys'),
    (6, 'meta'),
    (7, 'bookmarks'),
    (8, 'prefs'),
    (9, 'tabs'),
    (10, 'passwords'),
    (11, 'addons'),
    (12, 'addresses'),
    (13, 'creditcards');
-- Reserve space for additions to the standard collections, so that
-- custom collections created via AUTOINCREMENT start at 101 (see
-- FIRST_CUSTOM_COLLECTION_ID).
INSERT INTO collections (id, name) VALUES (100, '');

CREATE TABLE IF NOT EXISTS user_collections
(
    userid        BIGINT  NOT NULL,
    collection    INTEGER NOT NULL,
    -- last modified time in milliseconds since epoch
    last_modified BIGINT  NOT NULL,
    total_bytes   BIGINT,
    count         INTEGER,
    PRIMARY KEY (userid, collection)
);

CREATE TABLE IF NOT EXISTS batch_uploads
(
    batch      BIGINT  NOT NULL,
    userid     BIGINT  NOT NULL,
    collection INTEGER NOT NULL,
    PRIMARY KEY (batch, userid)
);

CREATE TABLE IF NOT EXISTS batch_upload_items
(
    batch        BIGINT NOT NULL,
    userid       BIGINT NOT NULL,
    id           TEXT   NOT NULL,
    sortindex    INTEGER DEFAULT NULL,
    payload      TEXT,
    payload_size BIGINT  DEFAULT NULL,
    ttl_offset   INTEGER DEFAULT NULL,
    PRIMARY KEY (batch, userid, id)
);
