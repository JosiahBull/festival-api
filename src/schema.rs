table! {
    cache (id) {
        id -> Int4,
        crt -> Timestamptz,
        nme -> Text,
        word -> Text,
        lang -> Text,
    }
}

table! {
    reqs (id) {
        id -> Int4,
        usr_id -> Int4,
        crt -> Timestamptz,
        word -> Text,
        lang -> Text,
        speed -> Float4,
    }
}

table! {
    users (id) {
        id -> Int4,
        usr -> Text,
        pwd -> Text,
        lckdwn -> Timestamptz,
        crt -> Timestamptz,
        last_accessed -> Timestamptz,
    }
}

joinable!(reqs -> users (usr_id));

allow_tables_to_appear_in_same_query!(
    cache,
    reqs,
    users,
);
