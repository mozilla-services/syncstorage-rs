diesel::table! {
    batch_bsos (user_id, collection_id, batch_id, batch_bso_id) {
        user_id -> Int8,
        collection_id -> Int4,
        batch_id -> Uuid,
        batch_bso_id -> Text,
        sortindex -> Nullable<Int4>,
        payload -> Nullable<Text>,
        ttl -> Nullable<Int8>,
    }
}

diesel::table! {
    batches (user_id, collection_id, batch_id) {
        user_id -> Int8,
        collection_id -> Int4,
        batch_id -> Uuid,
        expiry -> Timestamptz,
    }
}

diesel::table! {
    bsos (user_id, collection_id, bso_id) {
        user_id -> Int8,
        collection_id -> Int4,
        bso_id -> Text,
        sortindex -> Nullable<Int4>,
        payload -> Text,
        modified -> Timestamptz,
        expiry -> Timestamptz,
    }
}

diesel::table! {
    collections (collection_id) {
        collection_id -> Int4,
        #[max_length = 32]
        name -> Varchar,
    }
}

diesel::table! {
    user_collections (user_id, collection_id) {
        user_id -> Int8,
        collection_id -> Int4,
        modified -> Timestamptz,
        count -> Nullable<Int8>,
        total_bytes -> Nullable<Int8>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    batch_bsos,
    batches,
    bsos,
    collections,
    user_collections,
);
