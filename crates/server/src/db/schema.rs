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

diesel::table! {
    oauth_identities (id) {
        id -> Uuid,
        provider -> Text,
        provider_user_id -> Text,
        email -> Nullable<Text>,
        user_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    oauth_login_sessions (id) {
        id -> Uuid,
        provider -> Text,
        state -> Text,
        redirect_uri -> Text,
        created_at -> Timestamptz,
        expires_at -> Timestamptz,
    }
}

diesel::table! {
    roles (id) {
        id -> Uuid,
        name -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    user_roles (user_id, role_id) {
        user_id -> Uuid,
        role_id -> Uuid,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    role_permissions (id) {
        id -> Uuid,
        role_id -> Uuid,
        operation -> Text,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(desktop_auth_codes -> users (user_id));
diesel::joinable!(learning_records -> users (user_id));
diesel::joinable!(oauth_identities -> users (user_id));
diesel::joinable!(role_permissions -> roles (role_id));
diesel::joinable!(user_roles -> roles (role_id));
diesel::joinable!(user_roles -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    desktop_auth_codes,
    learning_records,
    oauth_identities,
    oauth_login_sessions,
    role_permissions,
    roles,
    user_roles,
    users,
);
