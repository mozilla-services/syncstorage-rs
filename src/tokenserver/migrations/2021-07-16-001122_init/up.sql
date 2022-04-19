CREATE TABLE IF NOT EXISTS `services` (
  `id` int NOT NULL AUTO_INCREMENT,
  `service` varchar(30) DEFAULT NULL,
  `pattern` varchar(128) DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `service` (`service`)
);

CREATE TABLE IF NOT EXISTS `nodes` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `service` int NOT NULL,
  `node` varchar(64) NOT NULL,
  `available` int NOT NULL DEFAULT '0',
  `current_load` int NOT NULL DEFAULT '0',
  `capacity` int NOT NULL DEFAULT '0',
  `downed` int NOT NULL DEFAULT '0',
  `backoff` int NOT NULL DEFAULT '0',
  PRIMARY KEY (`id`),
  KEY `service` (`service`),
  CONSTRAINT `nodes_ibfk_1` FOREIGN KEY (`service`) REFERENCES `services` (`id`)
);

CREATE TABLE IF NOT EXISTS `users` (
  `uid` bigint NOT NULL AUTO_INCREMENT,
  `service` int NOT NULL,
  `email` varchar(255) NOT NULL,
  `generation` bigint NOT NULL,
  `client_state` varchar(32) NOT NULL,
  `created_at` bigint NOT NULL,
  `replaced_at` bigint DEFAULT NULL,
  `nodeid` bigint NOT NULL,
  `keys_changed_at` bigint DEFAULT NULL,
  PRIMARY KEY (`uid`),
  KEY `nodeid` (`nodeid`),
  CONSTRAINT `users_ibfk_1` FOREIGN KEY (`nodeid`) REFERENCES `nodes` (`id`)
) ENGINE=InnoDB AUTO_INCREMENT=4 DEFAULT CHARSET=utf8
