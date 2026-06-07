// @generated automatically by Diesel CLI.

diesel::table! {
    categories (id) {
        id -> Uuid,
        #[max_length = 100]
        name -> Varchar,
        #[max_length = 100]
        slug -> Varchar,
        #[max_length = 20]
        kind -> Varchar,
        created_at -> Timestamp,
    }
}

diesel::table! {
    listings (id) {
        id -> Uuid,
        seller_id -> Uuid,
        #[max_length = 255]
        title -> Varchar,
        description -> Text,
        price_cents -> Int4,
        category_id -> Uuid,
        #[max_length = 20]
        status -> Varchar,
        #[max_length = 20]
        condition -> Varchar,
        quantity -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    favorites (id) {
        id -> Uuid,
        user_id -> Uuid,
        listing_id -> Uuid,
        created_at -> Timestamp,
    }
}

diesel::table! {
    listing_images (id) {
        id -> Uuid,
        listing_id -> Uuid,
        #[max_length = 1024]
        url -> Varchar,
        #[max_length = 1024]
        thumbnail_url -> Varchar,
        #[max_length = 512]
        s3_key -> Varchar,
        position -> Int4,
        created_at -> Timestamp,
    }
}

diesel::table! {
    password_resets (id) {
        id -> Uuid,
        user_id -> Uuid,
        #[max_length = 64]
        token_hash -> Varchar,
        expires_at -> Timestamp,
        used_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    permissions (id) {
        id -> Uuid,
        #[max_length = 100]
        name -> Varchar,
        #[max_length = 255]
        description -> Nullable<Varchar>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    role_permissions (role_id, permission_id) {
        role_id -> Uuid,
        permission_id -> Uuid,
        created_at -> Timestamp,
    }
}

diesel::table! {
    roles (id) {
        id -> Uuid,
        #[max_length = 50]
        name -> Varchar,
        #[max_length = 255]
        description -> Nullable<Varchar>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    sessions (id) {
        id -> Uuid,
        user_id -> Uuid,
        #[max_length = 255]
        refresh_token -> Varchar,
        expires_at -> Timestamp,
        created_at -> Timestamp,
    }
}

diesel::table! {
    user_roles (user_id, role_id) {
        user_id -> Uuid,
        role_id -> Uuid,
        created_at -> Timestamp,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 255]
        name -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        #[max_length = 255]
        password_hash -> Varchar,
    }
}

diesel::joinable!(password_resets -> users (user_id));
diesel::joinable!(favorites -> users (user_id));
diesel::joinable!(favorites -> listings (listing_id));
diesel::joinable!(listing_images -> listings (listing_id));
diesel::joinable!(listings -> categories (category_id));
diesel::joinable!(listings -> users (seller_id));
diesel::joinable!(role_permissions -> permissions (permission_id));
diesel::joinable!(role_permissions -> roles (role_id));
diesel::joinable!(sessions -> users (user_id));
diesel::joinable!(user_roles -> roles (role_id));
diesel::joinable!(user_roles -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    categories,
    favorites,
    listing_images,
    listings,
    password_resets,
    permissions,
    role_permissions,
    roles,
    sessions,
    user_roles,
    users,
);
