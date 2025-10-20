-- user_collections table
CREATE TABLE user_collections (
    fxa_uid TEXT NOT NULL,
    fxa_kid TEXT NOT NULL,
    collection_id BIGINT NOT NULL,
    modified TIMESTAMP NOT NULL,
    count BIGINT,
    total_bytes BIGINT,
    PRIMARY KEY (
        fxa_uid,
        fxa_kid,
        collection_id
    )
);

-- bsos table
CREATE TABLE bsos (
    fxa_uid TEXT NOT NULL,
    fxa_kid TEXT NOT NULL,
    collection_id BIGINT NOT NULL,
    bso_id TEXT NOT NULL,
    sortindex BIGINT,
    payload TEXT NOT NULL,
    modified TIMESTAMP NOT NULL,
    expiry TIMESTAMP NOT NULL,
    PRIMARY KEY (
        fxa_uid,
        fxa_kid,
        collection_id,
        bso_id
    ),
    FOREIGN KEY (
        fxa_uid,
        fxa_kid,
        collection_id
    ) REFERENCES user_collections (
        fxa_uid,
        fxa_kid,
        collection_id
    ) ON DELETE CASCADE
);

CREATE INDEX bsos_modified_idx ON bsos (
    fxa_uid,
    fxa_kid,
    collection_id,
    modified DESC
);

CREATE INDEX bsos_expiry_idx ON bsos (
    fxa_uid,
    fxa_kid,
    collection_id,
    expiry
);

-- collections table
CREATE TABLE collections (
    collection_id BIGINT PRIMARY KEY,
    name VARCHAR(32) NOT NULL UNIQUE
);

-- batches table
CREATE TABLE batches (
    fxa_uid TEXT NOT NULL,
    fxa_kid TEXT NOT NULL,
    collection_id BIGINT NOT NULL,
    batch_id TEXT NOT NULL,
    expiry TIMESTAMP NOT NULL,
    PRIMARY KEY (
        fxa_uid,
        fxa_kid,
        collection_id,
        batch_id
    ),
    FOREIGN KEY (
        fxa_uid,
        fxa_kid,
        collection_id
    ) REFERENCES user_collections (
        fxa_uid,
        fxa_kid,
        collection_id
    ) ON DELETE CASCADE
);

CREATE INDEX batch_expiry_idx ON batches (
    fxa_uid,
    fxa_kid,
    collection_id,
    expiry
);

-- batch_bsos table
CREATE TABLE batch_bsos (
    fxa_uid TEXT NOT NULL,
    fxa_kid TEXT NOT NULL,
    collection_id BIGINT NOT NULL,
    batch_id TEXT NOT NULL,
    batch_bso_id TEXT NOT NULL,
    sortindex BIGINT,
    payload TEXT,
    ttl BIGINT,
    PRIMARY KEY (
        fxa_uid,
        fxa_kid,
        collection_id,
        batch_id,
        batch_bso_id
    ),
    FOREIGN KEY (
        fxa_uid,
        fxa_kid,
        collection_id,
        batch_id
    ) REFERENCES batches (
        fxa_uid,
        fxa_kid,
        collection_id,
        batch_id
    ) ON DELETE CASCADE
);