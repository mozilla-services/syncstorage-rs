table! {
    batches (user_id, collection_id, id) {
        #[sql_name="userid"]
        user_id -> BigInt,
        #[sql_name="collection"]
        collection_id -> Integer,
        id -> Bigint,
        bsos -> Longtext,
        expiry -> Bigint,
    }
}

table! {
    bso (user_id, collection_id, id) {
        #[sql_name="userid"]
        user_id -> BigInt,
        #[sql_name="collection"]
        collection_id -> Integer,
        id -> Varchar,
        sortindex -> Nullable<Integer>,
        payload -> Mediumtext,
        // not used, but legacy
        payload_size -> Bigint,
        modified -> Bigint,
        #[sql_name="ttl"]
        expiry -> Bigint,
    }
}

table! {
    collections (id) {
        id -> Integer,
        name -> Varchar,
    }
}

table! {
    user_collections (user_id, collection_id) {
        #[sql_name="userid"]
        user_id -> BigInt,
        #[sql_name="collection"]
        collection_id -> Integer,
        #[sql_name="last_modified"]
        modified -> Bigint,
        #[sql_name="count"]
        count -> Integer,
        #[sql_name="total_bytes"]
        total_bytes -> BigInt,
    }
}

allow_tables_to_appear_in_same_query!(batches, bso, collections, user_collections);
