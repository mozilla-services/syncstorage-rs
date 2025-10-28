# Syncstorage Postgres Backend

## Tables Overview
| Table              | Description                                                                                      |
| ------------------ | ------------------------------------------------------------------------------------------------ |
| `user_collections` | Per-user metadata about each collection, including `last_modified`, record count, and total size |
| `bsos`             | Stores Basic Storage Objects (BSOs) that represent synced records                                |
| `collections`      | Maps collection names to their stable IDs                                                        |
| `batches`          | Temporary staging of BSOs in batch uploads                                                       |
| `batch_bsos`       | Stores BSOs that are part of a batch, pending commit                                             |

## User Collection Table
Stores per-user, per-collection metadata.

| Column          | Type        | Description                                                           |
| --------------- | ----------- | --------------------------------------------------------------------- |
| `fxa_uid`       | `UUID`      | Firefox Account UID PK (part 1)                              |
| `fxa_kid`       | `TEXT`      | Key identifier; part of the sync crypto context. PK (part 2) |
| `collection_id` | `BIGINT`    | Maps to a named collection. PK (part 3)                       |
| `modified`      | `TIMESTAMP` | Last modification time (server-assigned, updated on writes)           |
| `count`         | `BIGINT`    | Count of BSOs in this collection (used for quota enforcement)         |
| `total_bytes`   | `BIGINT`    | Total payload size of all BSOs (used for quota enforcement)     

Supports last-modified time tracking at the collection level.

Enables `/info/collections`, `/info/collection_counts`, and `/info/collection_usage endpoints`.

## BSOS Table
Stores actual records being synced — Basic Storage Objects.

| Column          | Type        | Description                                        |
| --------------- | ----------- | -------------------------------------------------- |
| `fxa_uid`       | `UUID`      | Firefox Account UID. PK (part 1) & FK (part 1) to `user_collections` |
| `fxa_kid`       | `TEXT`      | Key identifier. PK (part 2) & FK (part 2) to `user_collections`      |
| `collection_id` | `BIGINT`    | Maps to a named collection. PK (part 3) & FK (part 3) to `user_collections`                           |
| `bso_id`        | `TEXT`      | Unique ID within a collection. PK (part 4) |
| `sortindex`     | `BIGINT`    | Indicates record importance for syncing (optional) |
| `payload`       | `BYTEA`     | Bytes payload (e.g. JSON blob)                     |
| `modified`      | `TIMESTAMP` | Auto-assigned modification timestamp               |
| `expiry`        | `TIMESTAMP` | TTL as absolute expiration time (optional)         |

Indexes
`bsos_modified_idx`: for sorting by modified descending (used in sort=newest)

`bsos_expiry_idx`: for pruning expired records and TTL logic

Implements all BSO semantics from the [API spec](https://mozilla-services.readthedocs.io/en/latest/storage/apis-1.5.html#basic-storage-object)

## Collections Table
Maps internal numeric IDs to collection names.

| Column          | Type          | Description                     |
| --------------- | ------------- | ------------------------------- |
| `collection_id` | `BIGINT`      | Primary key                     |
| `name`          | `VARCHAR(32)` | Collection name, must be unique |

Used to reference collections efficiently via ID.

Collections can include bookmarks, tabs, passwords, etc.

## Batches Table
Temporary table for staging batch uploads before final commit.

| Column          | Type        | Description                                       |
| --------------- | ----------- | ------------------------------------------------- |
| `fxa_uid`       | `UUID`      | Firefox Account UID. PK (part 1) & FK (part 1) to `user_collections`        |
| `fxa_kid`       | `TEXT`      | Key identifier. PK (part 2) & FK (part 2) to `user_collections`             |
| `collection_id` | `BIGINT`    | Maps to a named collection. PK (part 3) & FK (part 3) to `user_collections` |
| `batch_id`      | `TEXT`      | Client-generated or server-assigned batch ID. PK (part 4)  |
| `expiry`        | `TIMESTAMP` | Time at which batch is discarded if not committed |

Indexes:
`batch_expiry_idx`: For cleaning up stale batches

## Batch BSOS Table
Stores BSOs during a batch upload, not yet committed to bsos.

| Column          | Type     | Description                 |
| --------------- | -------- | --------------------------- |
| `fxa_uid`       | `UUID`   | FK to `batches`             |
| `fxa_kid`       | `TEXT`   | FK to `batches`             |
| `collection_id` | `BIGINT` | FK to `batches`             |
| `batch_id`      | `TEXT`   | FK to `batches`             |
| `batch_bso_id`  | `TEXT`   | Unique ID within batch      |
| `sortindex`     | `BIGINT` | Optional, for sort priority |
| `payload`       | `BYTEA`  | Bytes payload               |
| `ttl`           | `BIGINT` | Time-to-live in seconds     |

## Database Diagram and Relationship
```mermaid
erDiagram
    USER_COLLECTIONS {
        UUID fxa_uid PK
        TEXT fxa_kid PK
        BIGINT collection_id PK
        TIMESTAMP modified
        BIGINT count
        BIGINT total_bytes
    }

    COLLECTIONS {
        BIGINT collection_id PK
        VARCHAR name
    }

    BSOS {
        UUID fxa_uid PK
        TEXT fxa_kid PK
        BIGINT collection_id PK
        TEXT bso_id PK
        BIGINT sortindex
        BYTEA payload
        TIMESTAMP modified
        TIMESTAMP expiry
    }

    BATCHES {
        UUID fxa_uid PK
        TEXT fxa_kid PK
        BIGINT collection_id PK
        TEXT batch_id PK
        TIMESTAMP expiry
    }

    BATCH_BSOS {
        UUID fxa_uid PK
        TEXT fxa_kid PK
        BIGINT collection_id PK
        TEXT batch_id PK
        TEXT batch_bso_id PK
        BIGINT sortindex
        BYTEA payload
        BIGINT ttl
    }

    USER_COLLECTIONS ||--o{ BSOS : "has"
    USER_COLLECTIONS ||--o{ BATCHES : "has"
    BATCHES ||--o{ BATCH_BSOS : "has"
    COLLECTIONS ||--o{ USER_COLLECTIONS : "mapped by"
```
