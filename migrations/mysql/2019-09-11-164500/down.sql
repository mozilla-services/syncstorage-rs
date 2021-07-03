-- At this point, it's sanest to just drop the tables rather than revert them
-- there are a number of non-backwards compatible changes performed and data
-- corruption is HIGHLY likely.
-- Best just try and install the python version (probably in a docker), and
-- let the client try and reconnect and restore.
DROP TABLE IF EXISTS `bso`;
DROP TABLE IF EXISTS `collections`;
DROP TABLE IF EXISTS `user_collections`;
DROP TABLE IF EXISTS `batches`;
