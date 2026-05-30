use utoipa::OpenApi;

use crate::auth::controller::{RegisterRequest, UserResponse as AuthUserResponse};
use crate::user::controller::{CreateUserRequest, UserResponse};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::user::controller::create_user,
        crate::auth::controller::register,
    ),
    components(
        schemas(CreateUserRequest, UserResponse, AuthUserResponse, RegisterRequest, crate::shared::errors::ErrorResponse),
    ),
    tags(
        (name = "users", description = "User management endpoints"),
        (name = "auth", description = "Authentication endpoints"),
    ),
)]
pub struct ApiDoc;
