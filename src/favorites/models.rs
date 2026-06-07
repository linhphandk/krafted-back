use chrono::NaiveDateTime;
use diesel::prelude::*;
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

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FavoriteResponse {
    pub id: String,
    pub listing_id: String,
    pub created_at: String,
}

impl FavoriteResponse {
    pub fn from_favorite(fav: &Favorite) -> Self {
        Self {
            id: fav.id.to_string(),
            listing_id: fav.listing_id.to_string(),
            created_at: fav.created_at.to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct FavoritesQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}
