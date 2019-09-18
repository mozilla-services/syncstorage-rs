alter table `batches` change column `user_id` `userid` int(11) not null;
alter table `batches` change column `collection_id` `collection` int(11) not null;
alter table `bso` change column `user_id` `userid` int(11) not null;
alter table `bso` change column `collection_id` `collection` int(11) not null;
alter table `bso` change column `expiry` `ttl` bigint(20) not null;
alter table `user_collections` change column `user_id` `userid` int(11) not null;
alter table `user_collections` change column `collection_id` `collection` int(11) not null;
alter table `user_collections` change column `modified` `last_modified` bigint(20) not null;
-- must be last in case of error
-- the following column is not used, but preserved for legacy and stand alone systems.
alter table `bso` add column `payload_size` int(11) default 0;
