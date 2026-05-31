use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
use tracing::{info, instrument, warn};
use uuid::Uuid;

use crate::listing::models::{
    CategoryKind, CreateListingRequest, ListingFilters, ListingResponse, ListingSort,
    PaginatedResponse, PaginatedResult, UpdateListingRequest,
};
use crate::shared::errors::{AppError, AppResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListListingsQuery {
    pub status: Option<String>,
    pub category_id: Option<String>,
    pub kind: Option<String>,
    pub search: Option<String>,
    pub sort: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ListCategoriesQuery {
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PageQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[instrument(skip_all)]
fn extract_seller_id(headers: &HeaderMap) -> AppResult<Uuid> {
    let result = headers
        .get("x-seller-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok());
    if result.is_none() {
        warn!("missing or invalid x-seller-id header");
    }
    result.ok_or_else(|| AppError::Unauthorized("Missing x-seller-id header".to_string()))
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

fn to_listing_response(
    listing: crate::listing::models::Listing,
    category_name: Option<String>,
) -> ListingResponse {
    ListingResponse::from_listing(&listing, category_name)
}

async fn listing_with_category(
    state: &AppState,
    listing: crate::listing::models::Listing,
) -> ListingResponse {
    let category_name = get_category_name(state, listing.category_id).await;
    to_listing_response(listing, category_name)
}

#[utoipa::path(
    get,
    path = "/api/listings",
    responses(
        (status = 200, description = "List of listings", body = PaginatedResponse<ListingResponse>),
    ),
    tag = "listings",
)]
#[instrument(skip(state), fields(search = ?query.search, sort = ?query.sort, page, per_page))]
pub async fn list_listings(
    State(state): State<AppState>,
    Query(query): Query<ListListingsQuery>,
) -> AppResult<Json<PaginatedResponse<ListingResponse>>> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let category_id = query.category_id.and_then(|s| Uuid::parse_str(&s).ok());
    let kind = query
        .kind
        .as_deref()
        .and_then(|s| s.parse::<CategoryKind>().ok());
    let sort = query
        .sort
        .as_deref()
        .map(|s| match s {
            "price_asc" => ListingSort::PriceAsc,
            "price_desc" => ListingSort::PriceDesc,
            _ => ListingSort::Newest,
        })
        .unwrap_or(ListingSort::Newest);

    info!(page, per_page, ?kind, "list_listings");

    let filters = ListingFilters {
        status: query.status,
        category_id,
        kind,
        search: query.search,
        sort,
    };

    let result = state
        .listing_service
        .list_listings(filters, page, per_page)
        .await?;

    let mut items = Vec::with_capacity(result.items.len());
    for listing in result.items {
        items.push(listing_with_category(&state, listing).await);
    }

    Ok(Json(PaginatedResponse::from_paginated_result(
        PaginatedResult {
            items,
            total: result.total,
            page: result.page,
            per_page: result.per_page,
        },
    )))
}

#[utoipa::path(
    get,
    path = "/api/listings/{id}",
    responses(
        (status = 200, description = "Listing details", body = ListingResponse),
        (status = 404, description = "Listing not found"),
    ),
    tag = "listings",
)]
#[instrument(skip(state), fields(listing_id = %id))]
pub async fn get_listing(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ListingResponse>> {
    info!("get_listing");
    let listing = state.listing_service.get_listing(id).await?;
    let category_name = get_category_name(&state, listing.category_id).await;
    Ok(Json(to_listing_response(listing, category_name)))
}

#[utoipa::path(
    post,
    path = "/api/listings",
    request_body = CreateListingRequest,
    responses(
        (status = 201, description = "Listing created", body = ListingResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "listings",
)]
#[instrument(skip(state, headers, req), fields(title = %req.title))]
pub async fn create_listing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateListingRequest>,
) -> AppResult<(StatusCode, Json<ListingResponse>)> {
    let seller_id = extract_seller_id(&headers)?;
    info!(seller_id = %seller_id, "create_listing");
    let listing = state.listing_service.create_listing(seller_id, req).await?;
    let category_name = get_category_name(&state, listing.category_id).await;
    Ok((
        StatusCode::CREATED,
        Json(to_listing_response(listing, category_name)),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/listings/{id}",
    request_body = UpdateListingRequest,
    responses(
        (status = 200, description = "Listing updated", body = ListingResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Listing not found"),
    ),
    tag = "listings",
)]
#[instrument(skip(state, headers, req), fields(listing_id = %id))]
pub async fn update_listing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateListingRequest>,
) -> AppResult<Json<ListingResponse>> {
    let seller_id = extract_seller_id(&headers)?;
    info!(seller_id = %seller_id, "update_listing");
    let listing = state
        .listing_service
        .update_listing(id, seller_id, req)
        .await?;
    let category_name = get_category_name(&state, listing.category_id).await;
    Ok(Json(to_listing_response(listing, category_name)))
}

#[utoipa::path(
    delete,
    path = "/api/listings/{id}",
    responses(
        (status = 204, description = "Listing deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Listing not found"),
    ),
    tag = "listings",
)]
#[instrument(skip(state, headers), fields(listing_id = %id))]
pub async fn delete_listing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let seller_id = extract_seller_id(&headers)?;
    info!(seller_id = %seller_id, "delete_listing");
    state.listing_service.delete_listing(id, seller_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/listings/{id}/publish",
    responses(
        (status = 200, description = "Listing published", body = ListingResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Listing not found"),
    ),
    tag = "listings",
)]
#[instrument(skip(state, headers), fields(listing_id = %id))]
pub async fn publish_listing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ListingResponse>> {
    let seller_id = extract_seller_id(&headers)?;
    info!(seller_id = %seller_id, "publish_listing");
    let listing = state.listing_service.publish_listing(id, seller_id).await?;
    let category_name = get_category_name(&state, listing.category_id).await;
    Ok(Json(to_listing_response(listing, category_name)))
}

#[utoipa::path(
    post,
    path = "/api/listings/{id}/pause",
    responses(
        (status = 200, description = "Listing paused", body = ListingResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Listing not found"),
    ),
    tag = "listings",
)]
#[instrument(skip(state, headers), fields(listing_id = %id))]
pub async fn pause_listing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ListingResponse>> {
    let seller_id = extract_seller_id(&headers)?;
    info!(seller_id = %seller_id, "pause_listing");
    let listing = state.listing_service.pause_listing(id, seller_id).await?;
    let category_name = get_category_name(&state, listing.category_id).await;
    Ok(Json(to_listing_response(listing, category_name)))
}

#[utoipa::path(
    get,
    path = "/api/listings/mine",
    responses(
        (status = 200, description = "Seller's listings", body = PaginatedResponse<ListingResponse>),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "listings",
)]
#[instrument(skip(state, headers), fields(page, per_page))]
pub async fn seller_listings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PageQuery>,
) -> AppResult<Json<PaginatedResponse<ListingResponse>>> {
    let seller_id = extract_seller_id(&headers)?;
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);

    info!(seller_id = %seller_id, page, per_page, "seller_listings");

    let result = state
        .listing_service
        .list_my_listings(seller_id, page, per_page)
        .await?;

    let mut items = Vec::with_capacity(result.items.len());
    for listing in result.items {
        items.push(listing_with_category(&state, listing).await);
    }

    Ok(Json(PaginatedResponse::from_paginated_result(
        PaginatedResult {
            items,
            total: result.total,
            page: result.page,
            per_page: result.per_page,
        },
    )))
}

#[utoipa::path(
    get,
    path = "/api/categories",
    responses(
        (status = 200, description = "List of categories", body = Vec<crate::listing::models::Category>),
    ),
    tag = "listings",
)]
#[instrument(skip(state), fields(kind = ?query.kind))]
pub async fn list_categories(
    State(state): State<AppState>,
    Query(query): Query<ListCategoriesQuery>,
) -> AppResult<Json<Vec<crate::listing::models::Category>>> {
    match query.kind {
        Some(kind) => {
            info!(kind, "list_categories filtered");
            let categories = state
                .category_service
                .list_categories_by_kind(&kind)
                .await?;
            Ok(Json(categories))
        }
        None => {
            info!("list_categories all");
            let categories = state.category_service.list_categories().await?;
            Ok(Json(categories))
        }
    }
}
