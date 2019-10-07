CREATE TABLE user_collections (
  fxa_uid STRING(MAX)  NOT NULL,
  fxa_kid STRING(MAX)  NOT NULL,
  collection_id INT64  NOT NULL,
  modified TIMESTAMP   NOT NULL,
) PRIMARY KEY(fxa_uid, fxa_kid, collection_id);


CREATE TABLE bso (
  fxa_uid STRING(MAX)  NOT NULL,
  fxa_kid STRING(MAX)  NOT NULL,
  collection_id INT64  NOT NULL,
  id STRING(MAX)       NOT NULL,

  sortindex INT64,

  payload STRING(MAX)  NOT NULL,

  modified TIMESTAMP   NOT NULL,
  expiry TIMESTAMP     NOT NULL,
)    PRIMARY KEY(fxa_uid, fxa_kid, collection_id, id),
  INTERLEAVE IN PARENT user_collections ON DELETE CASCADE;

    CREATE INDEX BsoModified
        ON bso(fxa_uid, fxa_kid, collection_id, modified DESC, expiry),
INTERLEAVE IN user_collections;

    CREATE INDEX BsoExpiry ON bso(expiry);


CREATE TABLE collections (
  id INT64          NOT NULL,
  name STRING(MAX)  NOT NULL,
) PRIMARY KEY(id);


CREATE TABLE batches (
  fxa_uid STRING(MAX)  NOT NULL,
  fxa_kid STRING(MAX)  NOT NULL,
  id TIMESTAMP         NOT NULL,
  collection_id INT64  NOT NULL,
  bsos STRING(MAX)     NOT NULL,
  expiry TIMESTAMP     NOT NULL,
  timestamp TIMESTAMP,
) PRIMARY KEY(fxa_uid, fxa_kid, collection_id, id);
