CREATE TABLE batches (
  userid STRING(MAX) NOT NULL,
  collection INT64 NOT NULL,
  id TIMESTAMP NOT NULL,
  fxa_kid STRING(MAX) NOT NULL,
  bsos STRING(MAX) NOT NULL,
  expiry TIMESTAMP NOT NULL,
  timestamp TIMESTAMP,
) PRIMARY KEY(userid, fxa_kid, collection, id);

CREATE TABLE collections (
  collectionid INT64 NOT NULL,
  name STRING(MAX) NOT NULL,
) PRIMARY KEY(collectionid);

CREATE TABLE user_collections (
  userid STRING(MAX) NOT NULL,
  fxa_kid STRING(MAX) NOT NULL,
  collection INT64 NOT NULL,
  last_modified TIMESTAMP NOT NULL,
) PRIMARY KEY(userid, fxa_kid, collection);

CREATE TABLE bso (
  userid STRING(MAX) NOT NULL,
  fxa_kid STRING(MAX) NOT NULL,
  collection INT64 NOT NULL,
  id STRING(MAX) NOT NULL,
  sortindex INT64,
  modified TIMESTAMP NOT NULL,
  payload STRING(MAX) NOT NULL,
  ttl TIMESTAMP NOT NULL,
) PRIMARY KEY(userid, fxa_kid, collection, id),
  INTERLEAVE IN PARENT user_collections ON DELETE CASCADE;

CREATE INDEX BsoLastModified ON bso(userid, fxa_kid, collection, modified DESC, ttl), INTERLEAVE IN user_collections;

CREATE INDEX BsoTtl ON bso(ttl)