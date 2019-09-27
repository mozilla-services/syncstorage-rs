-- /*!40100 DEFAULT CHARACTER SET latin1 */

-- DROP TABLE IF EXISTS `bso`;
-- XXX: bsov1, etc
CREATE TABLE `bso` (
    `user_id` INT                           NOT NULL,
    `collection_id` INT                     NOT NULL,
    `id` VARCHAR(64)                        NOT NULL,

    `sortindex` INT,

    `payload` MEDIUMTEXT                    NOT NULL,

    -- last modified time in milliseconds since epoch
    `modified` BIGINT                       NOT NULL,
    -- expiration in milliseconds since epoch
    `expiry` BIGINT DEFAULT '3153600000000' NOT NULL,

    PRIMARY KEY (`user_id`, `collection_id`, `id`),
    KEY `bso_expiry_idx` (`expiry`),
    KEY `bso_usr_col_mod_idx` (`user_id`, `collection_id`, `modified`)
) ENGINE=InnoDB DEFAULT CHARSET=latin1;


-- DROP TABLE IF EXISTS `collections`;
CREATE TABLE `collections` (
    `id` INT PRIMARY KEY      NOT NULL AUTO_INCREMENT,
    `name` VARCHAR(32) UNIQUE NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=latin1;
INSERT INTO collections (id, name) VALUES
    ( 1, "clients"),
    ( 2, "crypto"),
    ( 3, "forms"),
    ( 4, "history"),
    ( 5, "keys"),
    ( 6, "meta"),
    ( 7, "bookmarks"),
    ( 8, "prefs"),
    ( 9, "tabs"),
    (10, "passwords"),
    (11, "addons"),
    (12, "addresses"),
    (13, "creditcards");


-- DROP TABLE IF EXISTS `user_collections`;
CREATE TABLE `user_collections` (
    `user_id` INT       NOT NULL,
    `collection_id` INT NOT NULL,
    -- last modified time in milliseconds since epoch
    `modified` BIGINT   NOT NULL,
    PRIMARY KEY (`user_id`, `collection_id`)
) ENGINE=InnoDB DEFAULT CHARSET=latin1;


-- XXX: based on the go version (bsos is a concatenated blob of BSO jsons separated by newlines)
-- DROP TABLE IF EXISTS `batches`;
CREATE TABLE `batches` (
    `user_id` INT                           NOT NULL,
    `collection_id` INT                     NOT NULL,
    `id` BIGINT                             NOT NULL,

    `bsos` LONGTEXT                         NOT NULL,

    -- expiration in milliseconds since epoch
    `expiry` BIGINT DEFAULT '3153600000000' NOT NULL,

    PRIMARY KEY (`user_id`, `collection_id`, `id`)
) ENGINE=InnoDB DEFAULT CHARSET=latin1;
