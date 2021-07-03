// We use Bigint here instead of Integer, even though Sqlite does not have the concept of Bigint,
// to allow diesel to assume that integer is i64. See https://github.com/diesel-rs/diesel/issues/852

table! {
    batch_uploads (batch_id, user_id) {
        #[sql_name="batch"]
        batch_id -> Bigint,
        #[sql_name="userid"]
        user_id -> Bigint,
        #[sql_name="collection"]
        collection_id -> Integer,
    }
}

table! {
    batch_upload_items (batch_id, user_id, id) {
        #[sql_name="batch"]
        batch_id -> Bigint,
        #[sql_name="userid"]
        user_id -> Bigint,
        id -> Text,
        sortindex -> Nullable<Integer>,
        payload -> Nullable<Text>,
        payload_size -> Nullable<Bigint>,
        ttl_offset -> Nullable<Integer>,
    }
}

table! {
    bso (user_id, collection_id, id) {
        #[sql_name="userid"]
        user_id -> BigInt,
        #[sql_name="collection"]
        collection_id -> Integer,
        id -> Text,
        sortindex -> Nullable<Integer>,
        payload -> Text,
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
        name -> Text,
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

allow_tables_to_appear_in_same_query!(
    batch_uploads,
    batch_upload_items,
    bso,
    collections,
    user_collections,
);
