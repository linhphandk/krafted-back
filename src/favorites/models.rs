use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sql_types;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = crate::schema::favorites)]
pub struct Favorite {
    pub id: Uuid,
    pub user_id: Uuid,
    pub listing_id: Uuid,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::favorites)]
pub struct NewFavorite {
    pub user_id: Uuid,
    pub listing_id: Uuid,
}

#[derive(Debug, Clone, QueryableByName)]
pub struct FavoriteWithListing {
    #[diesel(sql_type = sql_types::Uuid)]
    pub id: Uuid,
    #[diesel(sql_type = sql_types::Uuid)]
    pub user_id: Uuid,
    #[diesel(sql_type = sql_types::Uuid)]
    pub listing_id: Uuid,
    #[diesel(sql_type = sql_types::Timestamp)]
    pub created_at: NaiveDateTime,
    #[diesel(sql_type = sql_types::Varchar)]
    pub title: String,
    #[diesel(sql_type = sql_types::Int4)]
    pub price_cents: i32,
    #[diesel(sql_type = sql_types::Varchar)]
    pub status: String,
    #[diesel(sql_type = sql_types::Nullable<sql_types::Varchar>)]
    pub thumbnail_url: Option<String>,
    #[diesel(sql_type = sql_types::Uuid)]
    pub seller_id: Uuid,
    #[diesel(sql_type = sql_types::Varchar)]
    pub seller_name: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FavoriteResponse {
    pub id: String,
    pub listing_id: String,
    pub title: String,
    pub price_cents: i32,
    pub status: String,
    pub image_url: Option<String>,
    pub seller_id: String,
    pub seller_name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct FavoritesQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}
