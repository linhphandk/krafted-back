use axum::extract::{Multipart, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
use tracing::{info, instrument, warn};
use uuid::Uuid;

use crate::listing::models::{
    CategoryKind, CreateListingRequest, ImageResponse, ListingFilters, ListingResponse,
    ListingSort, PaginatedResponse, PaginatedResult, ReorderImagesRequest, UpdateListingRequest,
};
use crate::shared::errors::{AppError, AppResult};
use crate::state::AppState;

const MAX_IMAGE_SIZE: usize = 500 * 1024;

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

async fn get_seller_name(state: &AppState, seller_id: Uuid) -> Option<String> {
    state
        .user_service
        .find_by_id(seller_id)
        .await
        .ok()
        .flatten()
        .map(|u| u.name)
}

fn to_listing_response(
    listing: crate::listing::models::Listing,
    category_name: Option<String>,
    seller_name: Option<String>,
) -> ListingResponse {
    ListingResponse::from_listing(&listing, category_name, seller_name)
}

async fn listing_with_category_and_seller(
    state: &AppState,
    listing: crate::listing::models::Listing,
) -> ListingResponse {
    let category_name = get_category_name(state, listing.category_id).await;
    let seller_name = get_seller_name(state, listing.seller_id).await;
    to_listing_response(listing, category_name, seller_name)
}

#[utoipa::path(
    get,
    path = "/api/listings",
    params(
        ("status" = Option<String>, Query, description = "Filter by status (active, draft, paused)"),
        ("category_id" = Option<String>, Query, description = "Filter by category UUID"),
        ("kind" = Option<String>, Query, description = "Filter by kind (craft, supply)"),
        ("search" = Option<String>, Query, description = "Search in title"),
        ("sort" = Option<String>, Query, description = "Sort order: newest, price_asc, price_desc"),
        ("page" = Option<i64>, Query, description = "Page number"),
        ("per_page" = Option<i64>, Query, description = "Items per page"),
    ),
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
        items.push(listing_with_category_and_seller(&state, listing).await);
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
    let seller_name = get_seller_name(&state, listing.seller_id).await;
    Ok(Json(to_listing_response(
        listing,
        category_name,
        seller_name,
    )))
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
    let seller_name = get_seller_name(&state, listing.seller_id).await;
    Ok((
        StatusCode::CREATED,
        Json(to_listing_response(listing, category_name, seller_name)),
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
    let seller_name = get_seller_name(&state, listing.seller_id).await;
    Ok(Json(to_listing_response(
        listing,
        category_name,
        seller_name,
    )))
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
    let seller_name = get_seller_name(&state, listing.seller_id).await;
    Ok(Json(to_listing_response(
        listing,
        category_name,
        seller_name,
    )))
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
    let seller_name = get_seller_name(&state, listing.seller_id).await;
    Ok(Json(to_listing_response(
        listing,
        category_name,
        seller_name,
    )))
}

#[utoipa::path(
    get,
    path = "/api/listings/mine",
    params(
        ("page" = Option<i64>, Query, description = "Page number"),
        ("per_page" = Option<i64>, Query, description = "Items per page"),
    ),
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
        items.push(listing_with_category_and_seller(&state, listing).await);
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
    params(
        ("kind" = Option<String>, Query, description = "Filter by kind (craft, supply)"),
    ),
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

#[utoipa::path(
    post,
    path = "/api/listings/{id}/images",
    params(
        ("id" = Uuid, Path, description = "Listing UUID"),
    ),
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Images uploaded", body = Vec<ImageResponse>),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Listing not found"),
    ),
    tag = "listings",
)]
pub async fn upload_images(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<Vec<ImageResponse>>)> {
    let seller_id = extract_seller_id(&headers)?;
    info!(seller_id = %seller_id, listing_id = %id, "upload_images");

    let mut files: Vec<(Vec<u8>, String, Option<i32>)> = Vec::new();
    let mut positions_json: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "files" {
            let content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();
            let data = field.bytes().await.map_err(|e| {
                warn!(error = %e, "failed to read multipart file");
                AppError::BadRequest("Failed to read file".to_string())
            })?;

            if data.len() > MAX_IMAGE_SIZE {
                warn!(size = data.len(), "file exceeds 500KB");
                return Err(AppError::BadRequest(
                    "File exceeds maximum size of 500KB".to_string(),
                ));
            }

            files.push((data.to_vec(), content_type, None));
        } else if name == "positions" {
            let text = field.text().await.map_err(|e| {
                warn!(error = %e, "failed to read positions field");
                AppError::BadRequest("Failed to read positions".to_string())
            })?;
            positions_json = Some(text);
        }
    }

    if let Some(json) = positions_json {
        let positions: Vec<Option<i32>> = serde_json::from_str(&json).map_err(|e| {
            warn!(error = %e, "invalid positions JSON");
            AppError::BadRequest("Invalid positions format".to_string())
        })?;
        for (i, pos) in positions.into_iter().enumerate() {
            if let Some(file) = files.get_mut(i) {
                file.2 = pos;
            }
        }
    }

    if files.is_empty() {
        return Err(AppError::BadRequest("No files provided".to_string()));
    }

    let result = state
        .listing_image_service
        .upload_images(id, seller_id, files)
        .await?;

    Ok((StatusCode::CREATED, Json(result)))
}

#[utoipa::path(
    get,
    path = "/api/listings/{id}/images",
    params(
        ("id" = Uuid, Path, description = "Listing UUID"),
    ),
    responses(
        (status = 200, description = "List of images", body = Vec<ImageResponse>),
    ),
    tag = "listings",
)]
pub async fn list_images(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<ImageResponse>>> {
    info!(listing_id = %id, "list_images");
    let images = state.listing_image_service.list_images(id).await?;
    Ok(Json(images))
}

#[utoipa::path(
    put,
    path = "/api/listings/{id}/images/reorder",
    params(
        ("id" = Uuid, Path, description = "Listing UUID"),
    ),
    request_body = ReorderImagesRequest,
    responses(
        (status = 200, description = "Images reordered"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Listing not found"),
    ),
    tag = "listings",
)]
pub async fn reorder_images(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<ReorderImagesRequest>,
) -> AppResult<Json<()>> {
    let seller_id = extract_seller_id(&headers)?;
    info!(seller_id = %seller_id, listing_id = %id, "reorder_images");
    state
        .listing_image_service
        .reorder_images(id, seller_id, req.image_ids)
        .await?;
    Ok(Json(()))
}

#[utoipa::path(
    delete,
    path = "/api/listings/{id}/images/{image_id}",
    params(
        ("id" = Uuid, Path, description = "Listing UUID"),
        ("image_id" = Uuid, Path, description = "Image UUID"),
    ),
    responses(
        (status = 204, description = "Image deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Listing or image not found"),
    ),
    tag = "listings",
)]
pub async fn delete_image(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, image_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    let seller_id = extract_seller_id(&headers)?;
    info!(seller_id = %seller_id, listing_id = %id, image_id = %image_id, "delete_image");
    state
        .listing_image_service
        .delete_image(id, image_id, seller_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
