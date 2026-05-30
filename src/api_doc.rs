use std::sync::LazyLock;
use utoipa::openapi::OpenApi as OpenApiSpec;
use utoipa::OpenApi;

use crate::user::controller::{CreateUserRequest, UserResponse};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::user::controller::create_user,
    ),
    components(
        schemas(CreateUserRequest, UserResponse, crate::shared::errors::ErrorResponse),
    ),
    tags(
        (name = "users", description = "User management endpoints"),
    ),
)]
pub struct ApiDoc;

pub static OPENAPI_SPEC: LazyLock<OpenApiSpec> = LazyLock::new(ApiDoc::openapi);
