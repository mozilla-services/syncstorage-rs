ALTER TABLE `nodes` ADD UNIQUE KEY `unique_idx` (`service`, `node`);
ALTER TABLE `users` ADD INDEX `lookup_idx` (`email`, `service`, `created_at`);
ALTER TABLE `users` ADD INDEX `replaced_at_idx` (`service`, `replaced_at`);
ALTER TABLE `users` ADD INDEX `node_idx` (`nodeid`);
