CREATE TABLE collections (collection_id INT64 NOT NULL, name STRING(32) NOT NULL,) PRIMARY KEY(collection_id)
CREATE UNIQUE INDEX CollectionName ON collections(name)
CREATE TABLE user_collections (fxa_uid STRING(MAX) NOT NULL, fxa_kid STRING(MAX) NOT NULL, collection_id INT64 NOT NULL, modified TIMESTAMP NOT NULL, count INT64, total_bytes INT64,) PRIMARY KEY(fxa_uid, fxa_kid, collection_id)
CREATE TABLE batches (fxa_uid STRING(MAX) NOT NULL, fxa_kid STRING(MAX) NOT NULL, collection_id INT64 NOT NULL, batch_id STRING(MAX) NOT NULL, expiry TIMESTAMP NOT NULL,) PRIMARY KEY(fxa_uid, fxa_kid, collection_id, batch_id), INTERLEAVE IN PARENT user_collections ON DELETE CASCADE
CREATE INDEX BatchExpireId ON batches(fxa_uid, fxa_kid, collection_id, expiry), INTERLEAVE IN user_collections
CREATE TABLE batch_bsos (fxa_uid STRING(MAX) NOT NULL, fxa_kid STRING(MAX) NOT NULL, collection_id INT64 NOT NULL, batch_id STRING(MAX) NOT NULL, batch_bso_id STRING(64) NOT NULL, sortindex INT64, payload STRING(MAX), ttl INT64,) PRIMARY KEY(fxa_uid, fxa_kid, collection_id, batch_id, batch_bso_id), INTERLEAVE IN PARENT batches ON DELETE CASCADE
CREATE TABLE bsos (fxa_uid STRING(MAX) NOT NULL, fxa_kid STRING(MAX) NOT NULL, collection_id INT64 NOT NULL, bso_id STRING(64) NOT NULL, sortindex INT64, payload STRING(MAX) NOT NULL, modified TIMESTAMP NOT NULL, expiry TIMESTAMP NOT NULL,) PRIMARY KEY(fxa_uid, fxa_kid, collection_id, bso_id), INTERLEAVE IN PARENT user_collections ON DELETE CASCADE
CREATE INDEX BsoExpiry ON bsos(fxa_uid, fxa_kid, collection_id, expiry), INTERLEAVE IN user_collections
CREATE INDEX BsoModified ON bsos(fxa_uid, fxa_kid, collection_id, modified DESC), INTERLEAVE IN user_collections