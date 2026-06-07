use axum::extract::{Path, Query};
use axum::{Extension, Json};
use axum::extract::State;
use axum::http::StatusCode;
use tracing::instrument;
use uuid::Uuid;

use crate::favorites::models::{FavoriteResponse, FavoritesQuery};
use crate::listing::models::PaginatedResponse;
use crate::shared::errors::AppResult;
use crate::shared::types::AuthenticatedUser;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/favorites/{listing_id}",
    responses(
        (status = 200, description = "Favorite added", body = FavoriteResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Listing not found"),
    ),
    tag = "favorites",
)]
#[instrument(skip(state), fields(listing_id = %listing_id))]
pub async fn add_favorite(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(listing_id): Path<Uuid>,
) -> AppResult<Json<FavoriteResponse>> {
    let fav = state
        .favorites_service
        .add_favorite(user.id, listing_id)
        .await?;
    Ok(Json(fav))
}

#[utoipa::path(
    delete,
    path = "/api/favorites/{listing_id}",
    responses(
        (status = 204, description = "Favorite removed"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Favorite not found"),
    ),
    tag = "favorites",
)]
#[instrument(skip(state), fields(listing_id = %listing_id))]
pub async fn remove_favorite(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(listing_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    state
        .favorites_service
        .remove_favorite(user.id, listing_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/favorites",
    params(
        ("page" = Option<i64>, Query, description = "Page number"),
        ("per_page" = Option<i64>, Query, description = "Items per page"),
    ),
    responses(
        (status = 200, description = "List of favorites", body = PaginatedResponse<FavoriteResponse>),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "favorites",
)]
#[instrument(skip(state), fields(page, per_page))]
pub async fn list_favorites(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(query): Query<FavoritesQuery>,
) -> AppResult<Json<PaginatedResponse<FavoriteResponse>>> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);

    let result = state
        .favorites_service
        .list_favorites(user.id, page, per_page)
        .await?;

    Ok(Json(PaginatedResponse::from_paginated_result(result)))
}
