ALTER TABLE `nodes` DROP INDEX `unique_idx`;
ALTER TABLE `users` DROP INDEX `lookup_idx`;
ALTER TABLE `users` DROP INDEX `replaced_at_idx`;
ALTER TABLE `users` DROP INDEX `node_idx`;
