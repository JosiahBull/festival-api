table! {
    cache (id) {
        id -> Int4,
        crt -> Nullable<Timestamptz>,
        nme -> Text,
        word -> Text,
        lang -> Text,
    }
}

table! {
    reqs (id) {
        id -> Int4,
        usr_id -> Int4,
        crt -> Nullable<Timestamptz>,
        word -> Text,
        lang -> Text,
        speed -> Float4,
        ip_addr -> Bytea,
    }
}

table! {
    users (id) {
        id -> Int4,
        usr -> Text,
        pwd -> Text,
        lckdwn -> Nullable<Timestamptz>,
        crt -> Nullable<Timestamptz>,
        last_accessed -> Nullable<Timestamptz>,
    }
}

joinable!(reqs -> users (usr_id));

allow_tables_to_appear_in_same_query!(
    cache,
    reqs,
    users,
);
