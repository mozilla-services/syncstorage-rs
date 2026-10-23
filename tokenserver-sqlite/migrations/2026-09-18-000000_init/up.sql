CREATE TABLE IF NOT EXISTS services (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  service TEXT UNIQUE,
  pattern TEXT
);
-- The standard Sync service entry
INSERT INTO services (service, pattern) VALUES ('sync-1.5', '{node}/1.5/{uid}');

CREATE TABLE IF NOT EXISTS nodes (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  service INTEGER NOT NULL,
  node TEXT NOT NULL,
  available INTEGER NOT NULL,
  current_load INTEGER NOT NULL,
  capacity INTEGER NOT NULL,
  downed INTEGER NOT NULL,
  backoff INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS unique_idx ON nodes (service, node);

CREATE TABLE IF NOT EXISTS users (
  uid INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  service INTEGER NOT NULL,
  email TEXT NOT NULL,
  generation BIGINT NOT NULL,
  client_state TEXT NOT NULL,
  created_at BIGINT NOT NULL,
  replaced_at BIGINT,
  nodeid BIGINT NOT NULL,
  keys_changed_at BIGINT
);
CREATE INDEX IF NOT EXISTS lookup_idx ON users (email, service, created_at);
CREATE INDEX IF NOT EXISTS replaced_at_idx ON users (service, replaced_at);
CREATE INDEX IF NOT EXISTS node_idx ON users (nodeid);
