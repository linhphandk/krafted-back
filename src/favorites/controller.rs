use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::listing::models::{ListingResponse, PaginatedResponse};
use crate::shared::errors::AppResult;
use crate::shared::types::AuthenticatedUser;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct PageQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct FavoriteResponse {
    pub id: String,
    pub listing: ListingResponse,
    pub created_at: String,
}

async fn listing_with_details(
    state: &AppState,
    listing: crate::listing::models::Listing,
) -> ListingResponse {
    let category_name = get_category_name(state, listing.category_id).await;
    let seller_name = get_seller_name(state, listing.seller_id).await;
    ListingResponse::from_listing(&listing, category_name, seller_name)
}

async fn get_category_name(state: &AppState, category_id: Uuid) -> Option<String> {
    state
        .category_service
        .list_categories()
        .await
        .ok()
        .and_then(|cats| cats.into_iter().find(|c| c.id == category_id))
        .map(|c| c.name)
}

async fn get_seller_name(state: &AppState, seller_id: Uuid) -> Option<String> {
    state
        .user_service
        .find_by_id(seller_id)
        .await
        .ok()
        .flatten()
        .map(|u| u.name)
}

#[utoipa::path(
    post,
    path = "/api/listings/{id}/favorite",
    responses(
        (status = 201, description = "Added to favorites", body = FavoriteResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Listing not found"),
    ),
    tag = "favorites",
)]
#[instrument(skip(state), fields(listing_id = %id))]
pub async fn add_favorite(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<(StatusCode, Json<FavoriteResponse>)> {
    info!(user_id = %user.id, listing_id = %id, "add_favorite");
    let fav = state.favorite_service.add(user.id, id).await?;
    let listing = state.listing_service.get_listing(id).await?;
    let listing_resp = listing_with_details(&state, listing).await;
    Ok((
        StatusCode::CREATED,
        Json(FavoriteResponse {
            id: fav.id.to_string(),
            listing: listing_resp,
            created_at: fav.created_at.to_string(),
        }),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/listings/{id}/favorite",
    responses(
        (status = 204, description = "Removed from favorites"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Favorite not found"),
    ),
    tag = "favorites",
)]
#[instrument(skip(state), fields(listing_id = %id))]
pub async fn remove_favorite(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    info!(user_id = %user.id, listing_id = %id, "remove_favorite");
    state.favorite_service.remove(user.id, id).await?;
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
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState>,
    Query(query): Query<PageQuery>,
) -> AppResult<Json<PaginatedResponse<FavoriteResponse>>> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);

    info!(user_id = %user.id, page = %page, per_page = %per_page, "list_favorites");

    let result = state.favorite_service.list(user.id, page, per_page).await?;

    let mut items = Vec::with_capacity(result.items.len());
    for (fav, listing) in result.items {
        let listing_resp = listing_with_details(&state, listing).await;
        items.push(FavoriteResponse {
            id: fav.id.to_string(),
            listing: listing_resp,
            created_at: fav.created_at.to_string(),
        });
    }

    let total_pages = (result.total as f64 / result.per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        items,
        total: result.total,
        page: result.page,
        per_page: result.per_page,
        total_pages,
    }))
}
