-- PostgreSQL enforces referential integrity, so to drop foreign-key-constrained tables, we must reverse the creation order:
-- Indexes must be dropped before the tables they belong to.
-- Child tables (e.g., bsos, batches, batch_bsos) must be dropped before their parent tables (user_collections) due to ON DELETE CASCADE.
-- The collections table can be dropped after all dependencies are removed.

-- Drop indexes first to avoid dangling references
DROP INDEX bsos_modified_idx;

DROP INDEX bsos_expiry_idx;

DROP INDEX batch_expiry_idx;

ALTER TABLE collections
DROP CONSTRAINT IF EXISTS collections_name_key;

-- Drop child tables first (reverse dependency order)
DROP TABLE batch_bsos;

DROP TABLE batches;

DROP TABLE bsos;

DROP TABLE collections;

DROP TABLE user_collections;