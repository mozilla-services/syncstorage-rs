diesel::table! {
    batch_bsos (fxa_uid, fxa_kid, collection_id, batch_id, batch_bso_id) {
        fxa_uid -> Uuid,
        fxa_kid -> Text,
        collection_id -> Int8,
        batch_id -> Text,
        batch_bso_id -> Text,
        sortindex -> Nullable<Int8>,
        payload -> Nullable<Bytea>,
        ttl -> Nullable<Int8>,
    }
}

diesel::table! {
    batches (fxa_uid, fxa_kid, collection_id, batch_id) {
        fxa_uid -> Uuid,
        fxa_kid -> Text,
        collection_id -> Int8,
        batch_id -> Text,
        expiry -> Timestamp,
    }
}

diesel::table! {
    bsos (fxa_uid, fxa_kid, collection_id, bso_id) {
        fxa_uid -> Uuid,
        fxa_kid -> Text,
        collection_id -> Int8,
        bso_id -> Text,
        sortindex -> Nullable<Int8>,
        payload -> Bytea,
        modified -> Timestamp,
        expiry -> Timestamp,
    }
}

diesel::table! {
    collections (collection_id) {
        collection_id -> Int8,
        #[max_length = 32]
        name -> Varchar,
    }
}

diesel::table! {
    user_collections (fxa_uid, fxa_kid, collection_id) {
        fxa_uid -> Uuid,
        fxa_kid -> Text,
        collection_id -> Int8,
        modified -> Timestamp,
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
