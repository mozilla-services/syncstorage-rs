table! {
    batches (id) {
        id -> BigInt,
        collection_id -> BigInt,
        last_modified -> BigInt,
        bsos -> Text,
    }
}

table! {
    bso (collection_id, id) {
        collection_id -> BigInt,
        id -> Text,
        sortindex -> Nullable<BigInt>,
        payload -> Text,
        payload_size -> BigInt,
        last_modified -> BigInt,
        expiry -> BigInt,
    }
}

table! {
    collections (id) {
        id -> BigInt,
        name -> Text,
        last_modified -> BigInt,
    }
}

table! {
    keyvalues (key) {
        key -> Text,
        value -> Text,
    }
}

allow_tables_to_appear_in_same_query!(batches, bso, collections, keyvalues,);
