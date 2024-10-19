pub const LAST_INSERT_ID_QUERY: &'static str = "SELECT LAST_INSERT_ROWID() AS id";
pub const GET_NODE_ID_SYNC_QUERY: &str = r#"
SELECT rowid as id
FROM nodes
WHERE service = ?
AND node = ?"#;
// FIXME: MySQL specific
pub const REPLACE_USERS_SYNC_QUERY: &str = r#"
UPDATE users
SET replaced_at = ?
WHERE service = ?
AND email = ?
AND replaced_at IS NULL
AND created_at < ?"#;
pub const REPLACE_USER_SYNC_QUERY: &str = r#"
UPDATE users
SET replaced_at = ?
WHERE service = ?
AND uid = ?"#;
// The `where` clause on this statement is designed as an extra layer of
// protection, to ensure that concurrent updates don't accidentally move
// timestamp fields backwards in time. The handling of `keys_changed_at`
// is additionally weird because we want to treat the default `NULL` value
// as zero.
pub const PUT_USER_SYNC_QUERY: &str = r#"
UPDATE users
SET generation = ?,
keys_changed_at = ?
WHERE service = ?
AND email = ?
AND generation <= ?
AND COALESCE(keys_changed_at, 0) <= COALESCE(?, keys_changed_at, 0)
AND replaced_at IS NULL"#;
// FIXME: MySQL specific
pub const POST_USER_SYNC_QUERY: &str = r#"
INSERT INTO users (service, email, generation, client_state, created_at, nodeid, keys_changed_at, replaced_at)
VALUES (?, ?, ?, ?, ?, ?, ?, ?);"#;
pub const CHECK_SYNC_QUERY: &str = "SHOW STATUS LIKE \"Uptime\"";
pub const GET_BEST_NODE_QUERY: &str = r#"
SELECT id, node
FROM nodes
WHERE service = ?
AND available > 0
AND capacity > current_load
AND downed = 0
AND backoff = 0
ORDER BY LOG(current_load) / LOG(capacity)
LIMIT 1"#;
pub const GET_BEST_NODE_RELEASE_CAPACITY_QUERY: &str = r#"
UPDATE nodes
SET available = LEAST(capacity * ?, capacity - current_load)
WHERE service = ?
AND available <= 0
AND capacity > current_load
AND downed = 0"#;
// FIXME: MySQL specific
pub const GET_BEST_NODE_SPANNER_QUERY: &str = r#"
SELECT id, node
FROM nodes
WHERE id = ?
LIMIT 1"#;
pub const ADD_USER_TO_NODE_SYNC_QUERY: &str = r#"
UPDATE nodes
SET current_load = current_load + 1,
available = GREATEST(available - 1, 0)
WHERE service = ?
AND node = ?"#;
pub const ADD_USER_TO_NODE_SYNC_SPANNER_QUERY: &str = r#"
UPDATE nodes
SET current_load = current_load + 1
WHERE service = ?
AND node = ?"#;
pub const GET_USERS_SYNC_QUERY: &str = r#"
SELECT uid, nodes.node, generation, keys_changed_at, client_state, created_at, replaced_at
FROM users
LEFT OUTER JOIN nodes ON users.nodeid = nodes.id
WHERE email = ?
AND users.service = ?
ORDER BY created_at DESC, uid DESC
LIMIT 20"#;
pub const GET_SERVICE_ID_SYNC_QUERY: &str = r#"
SELECT id
FROM services
WHERE service = ?"#;
pub const SET_USER_CREATED_AT_SYNC_QUERY: &str = r#"
UPDATE users
SET created_at = ?
WHERE uid = ?"#;
pub const SET_USER_REPLACED_AT_SYNC_QUERY: &str = r#"
UPDATE users
SET replaced_at = ?
WHERE uid = ?"#;
pub const GET_USER_SYNC_QUERY: &str = r#"
SELECT service, email, generation, client_state, replaced_at, nodeid, keys_changed_at
FROM users
WHERE uid = ?"#;
pub const POST_NODE_SYNC_QUERY: &str = r#"
INSERT INTO nodes (service, node, available, current_load, capacity, downed, backoff)
VALUES (?, ?, ?, ?, ?, ?, ?)"#;
pub const GET_NODE_SYNC_QUERY: &str = r#"
SELECT *
FROM nodes
WHERE id = ?"#;
pub const UNASSIGNED_NODE_SYNC_QUERY: &str = r#"
UPDATE users
SET replaced_at = ?
WHERE nodeid = ?"#;
pub const REMOVE_NODE_SYNC_QUERY: &str = "DELETE FROM nodes WHERE id = ?";
pub const POST_SERVICE_INSERT_SERVICE_QUERY: &str = r#"
INSERT INTO services (service, pattern)
VALUES (?, ?)"#;
