diesel::table! {
    nodes (id) {
        id -> Int8,
        service -> Int4,
        #[max_length = 64]
        node -> Varchar,
        available -> Int4,
        current_load -> Int4,
        capacity -> Int4,
        downed -> Int4,
        backoff -> Int4,
    }
}

diesel::table! {
    services (id) {
        id -> Int4,
        #[max_length = 30]
        service -> Nullable<Varchar>,
        #[max_length = 128]
        pattern -> Nullable<Varchar>,
    }
}

diesel::table! {
    users (uid) {
        uid -> Int8,
        service -> Int4,
        #[max_length = 255]
        email -> Varchar,
        generation -> Int8,
        #[max_length = 32]
        client_state -> Varchar,
        created_at -> Int8,
        replaced_at -> Nullable<Int8>,
        nodeid -> Int8,
        keys_changed_at -> Nullable<Int8>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(nodes, services, users,);
