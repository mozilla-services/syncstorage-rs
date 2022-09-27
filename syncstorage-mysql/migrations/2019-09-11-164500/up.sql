-- tests to see if columns need adjustment
-- this is because the `init` may create the tables
-- with column names already correct

CREATE PROCEDURE UPDATE_165600()
BEGIN
    IF EXISTS( SELECT column_name
        FROM INFORMATION_SCHEMA.COLUMNS
        WHERE table_schema=database() AND table_name='bso' AND column_name="user_id")
    THEN
        BEGIN
            alter table `bso` change column `user_id` `userid` int(11) not null;
            alter table `bso` change column `collection_id` `collection` int(11) not null;
            alter table `bso` change column `expiry` `ttl` bigint(20) not null;
        END;
    END IF;

    IF EXISTS( SELECT column_name
        FROM INFORMATION_SCHEMA.COLUMNS
        WHERE table_schema=database() AND table_name='batches' AND column_name="user_id")
    THEN
        BEGIN
            alter table `batches` change column `user_id` `userid` int(11) not null;
            alter table `batches` change column `collection_id` `collection` int(11) not null;
        END;
    END IF;

    IF EXISTS( SELECT column_name
        FROM INFORMATION_SCHEMA.COLUMNS
        WHERE table_schema=database() AND table_name='user_collections' AND column_name="user_id")
    THEN
        BEGIN
            alter table `user_collections` change column `user_id` `userid` int(11) not null;
            alter table `user_collections` change column `collection_id` `collection` int(11) not null;
            alter table `user_collections` change column `modified` `last_modified` bigint(20) not null;
        END;
    END IF;

    -- must be last in case of error
    -- the following column is not used, but preserved for legacy and stand alone systems.
    IF NOT EXISTS( SELECT column_name
        FROM INFORMATION_SCHEMA.COLUMNS
        where table_schema=database() AND table_name='bso' AND column_name="payload_size")
    THEN
        BEGIN
            alter table `bso` add column `payload_size` int(11) default 0;
        END;

    END IF;
END;

CALL UPDATE_165600();

DROP PROCEDURE UPDATE_165600;
