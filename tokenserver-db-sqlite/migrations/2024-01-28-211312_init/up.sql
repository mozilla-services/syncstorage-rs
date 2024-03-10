CREATE TABLE IF NOT EXISTS `services` (
  `id` int PRIMARY KEY,
  `service` varchar(30) DEFAULT NULL UNIQUE,
  `pattern` varchar(128) DEFAULT NULL
);

CREATE TABLE IF NOT EXISTS `nodes` (
  `id` bigint PRIMARY KEY,
  `service` int NOT NULL,
  `node` varchar(64) NOT NULL,
  `available` int NOT NULL,
  `current_load` int NOT NULL,
  `capacity` int NOT NULL,
  `downed` int NOT NULL,
  `backoff` int NOT NULL
);

CREATE UNIQUE INDEX `unique_idx` ON `nodes` (`service`, `node`);

CREATE TABLE IF NOT EXISTS `users` (
  `uid` PRIMARY KEY,
  `service` int NOT NULL,
  `email` varchar(255) NOT NULL,
  `generation` bigint NOT NULL,
  `client_state` varchar(32) NOT NULL,
  `created_at` bigint NOT NULL,
  `replaced_at` bigint DEFAULT NULL,
  `nodeid` bigint NOT NULL,
  `keys_changed_at` bigint DEFAULT NULL
);

CREATE INDEX `lookup_idx` ON `users` (`email`, `service`, `created_at`);
CREATE INDEX `replaced_at_idx` ON `users` (`service`, `replaced_at`);
CREATE INDEX `node_idx` ON `users` (`nodeid`);
