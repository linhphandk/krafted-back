use utoipa::OpenApi;

use crate::auth::controller::{
    LoginRequest, LoginResponse, LogoutRequest, RefreshRequest, RefreshResponse, RegisterRequest,
    UserResponse,
};
use crate::listing::models::{
    Category, CreateListingRequest, ImageResponse, ListingResponse, ListingSort, PaginatedResponse,
    ReorderImagesRequest, UpdateListingRequest,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::auth::controller::register,
        crate::auth::controller::login,
        crate::auth::controller::logout,
        crate::auth::controller::refresh,
        crate::auth::controller::me,
        crate::listing::controller::list_listings,
        crate::listing::controller::get_listing,
        crate::listing::controller::create_listing,
        crate::listing::controller::update_listing,
        crate::listing::controller::delete_listing,
        crate::listing::controller::publish_listing,
        crate::listing::controller::pause_listing,
        crate::listing::controller::seller_listings,
        crate::listing::controller::list_categories,
        crate::listing::controller::upload_images,
        crate::listing::controller::list_images,
        crate::listing::controller::reorder_images,
        crate::listing::controller::delete_image,
    ),
    components(
        schemas(
            RegisterRequest,
            LoginRequest,
            LogoutRequest,
            RefreshRequest,
            LoginResponse,
            RefreshResponse,
            UserResponse,
            CreateListingRequest,
            UpdateListingRequest,
            ListingResponse,
            Category,
            ListingSort,
            PaginatedResponse<ListingResponse>,
            ImageResponse,
            ReorderImagesRequest,
            crate::shared::errors::ErrorResponse,
        ),
    ),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "listings", description = "Listing and category endpoints"),
    ),
)]
pub struct ApiDoc;
