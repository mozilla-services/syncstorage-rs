ALTER TABLE `nodes` ADD CONSTRAINT `nodes_ibfk_1` FOREIGN KEY (`service`) REFERENCES `services` (`id`);
ALTER TABLE `users` ADD CONSTRAINT `users_ibfk_1` FOREIGN KEY (`nodeid`) REFERENCES `nodes` (`id`);
