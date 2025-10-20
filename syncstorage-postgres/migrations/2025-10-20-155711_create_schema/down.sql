-- This file should undo anything in `up.sql`
-- PostgreSQL enforces referential integrity, so to drop foreign-key-constrained tables, we must reverse the creation order:
-- Indexes must be dropped before the tables they belong to.
-- Child tables (e.g., bsos, batches, batch_bsos) must be dropped before their parent tables (user_collections) due to ON DELETE CASCADE.
-- The collections table can be dropped after all dependencies are removed.

-- Drop indexes first to avoid dangling references
DROP INDEX IF EXISTS bsos_modified_idx;

DROP INDEX IF EXISTS bsos_expiry_idx;

DROP INDEX IF EXISTS batch_expiry_idx;

DROP INDEX IF EXISTS collections_name_key;

-- Drop child tables first (reverse dependency order)
DROP TABLE IF EXISTS batch_bsos;

DROP TABLE IF EXISTS batches;

DROP TABLE IF EXISTS bsos;

DROP TABLE IF EXISTS collections;

DROP TABLE IF EXISTS user_collections;