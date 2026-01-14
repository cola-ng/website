diesel::table! {
    desktop_auth_codes (id) {
        id -> Uuid,
        user_id -> Uuid,
        code_hash -> Text,
        redirect_uri -> Text,
        state -> Text,
        expires_at -> Timestamptz,
        used_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    learning_records (id) {
        id -> Uuid,
        user_id -> Uuid,
        record_type -> Text,
        content -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        email -> Text,
        password_hash -> Text,
        name -> Nullable<Text>,
        phone -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(desktop_auth_codes -> users (user_id));
diesel::joinable!(learning_records -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(desktop_auth_codes, learning_records, users,);
