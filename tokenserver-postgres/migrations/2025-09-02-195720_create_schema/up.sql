-- Create Tables
CREATE TABLE IF NOT EXISTS services (
    id SERIAL PRIMARY KEY,
    service VARCHAR(30) UNIQUE,
    pattern VARCHAR(128)
);

CREATE TABLE IF NOT EXISTS nodes (
    id BIGSERIAL PRIMARY KEY,
    service INTEGER NOT NULL,
    node VARCHAR(64) NOT NULL,
    available INTEGER NOT NULL,
    current_load INTEGER NOT NULL,
    capacity INTEGER NOT NULL,
    downed INTEGER NOT NULL,
    backoff INTEGER NOT NULL,
    UNIQUE (service, node)
);

CREATE TABLE IF NOT EXISTS users (
    uid BIGSERIAL PRIMARY KEY,
    service INTEGER NOT NULL,
    email VARCHAR(255) NOT NULL,
    generation BIGINT NOT NULL,
    client_state VARCHAR(32) NOT NULL,
    created_at BIGINT NOT NULL,
    replaced_at BIGINT,
    nodeid BIGINT NOT NULL,
    keys_changed_at BIGINT
);

-- Create Indexes for `users` table
CREATE INDEX IF NOT EXISTS lookup_idx ON users (email, service, created_at);

CREATE INDEX IF NOT EXISTS replaced_at_idx ON users (service, replaced_at);

CREATE INDEX IF NOT EXISTS node_idx ON users (nodeid);