-- user_collections table
CREATE TABLE user_collections (
    user_id BIGINT NOT NULL,
    collection_id INTEGER NOT NULL,
    modified TIMESTAMP NOT NULL,
    count BIGINT,
    total_bytes BIGINT,
    PRIMARY KEY (
	user_id,
        collection_id
    )
);

-- bsos table
CREATE TABLE bsos (
    user_id BIGINT NOT NULL,
    collection_id INTEGER NOT NULL,
    bso_id TEXT NOT NULL,
    sortindex BIGINT,
    payload BYTEA NOT NULL,
    modified TIMESTAMP NOT NULL,
    expiry TIMESTAMP NOT NULL,
    PRIMARY KEY (
        user_id,
        collection_id,
        bso_id
    ),
    FOREIGN KEY (
        user_id,
        collection_id
    ) REFERENCES user_collections (
        user_id,
        collection_id
    ) ON DELETE CASCADE
);

CREATE INDEX bsos_modified_idx ON bsos (
    user_id,
    collection_id,
    modified DESC
);

CREATE INDEX bsos_expiry_idx ON bsos (
    user_id,
    collection_id,
    expiry
);

-- collections table
CREATE TABLE collections (
    collection_id INTEGER PRIMARY KEY,
    name VARCHAR(32) NOT NULL UNIQUE
);

-- batches table
CREATE TABLE batches (
    user_id BIGINT NOT NULL,
    collection_id INTEGER NOT NULL,
    batch_id TEXT NOT NULL,
    expiry TIMESTAMP NOT NULL,
    PRIMARY KEY (
        user_id,
        collection_id,
        batch_id
    ),
    FOREIGN KEY (
        user_id,
        collection_id
    ) REFERENCES user_collections (
        user_id,
        collection_id
    ) ON DELETE CASCADE
);

CREATE INDEX batch_expiry_idx ON batches (
    user_id,
    collection_id,
    expiry
);

-- batch_bsos table
CREATE TABLE batch_bsos (
    user_id BIGINT NOT NULL,
    collection_id INTEGER NOT NULL,
    batch_id TEXT NOT NULL,
    batch_bso_id TEXT NOT NULL,
    sortindex BIGINT,
    payload BYTEA,
    ttl BIGINT,
    PRIMARY KEY (
        user_id,
        collection_id,
        batch_id,
        batch_bso_id
    ),
    FOREIGN KEY (
        user_id,
        collection_id,
        batch_id
    ) REFERENCES batches (
        user_id,
        collection_id,
        batch_id
    ) ON DELETE CASCADE
);
