use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::shared::errors::AppResult;
use crate::state::AppState;

#[derive(Deserialize, ToSchema)]
pub struct RegisterRequest {
    #[schema(example = "user@example.com")]
    pub email: String,
    #[schema(example = "John Doe")]
    pub name: String,
    #[schema(example = "securepassword123")]
    pub password: String,
}

#[derive(serde::Serialize, ToSchema)]
pub struct LoginResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    #[schema(example = "user@example.com")]
    pub email: String,
    #[schema(example = "securepassword123")]
    pub password: String,
}

#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 400, description = "Bad request", body = crate::shared::errors::ErrorResponse),
    ),
    tag = "auth",
)]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<(StatusCode, Json<LoginResponse>)> {
    let (user, tokens) = state
        .auth_service
        .login(req.email, req.password)
        .await?;
    let response = LoginResponse {
        user: UserResponse {
            id: user.id.to_string(),
            email: user.email,
            name: user.name,
        },
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_in: tokens.expires_in,
    };
    Ok((StatusCode::OK, Json(response)))
}

#[derive(serde::Serialize, ToSchema)]
pub struct RegisterResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub expires_in: u64,
}

#[derive(serde::Serialize, ToSchema)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
}

#[utoipa::path(
    post,
    path = "/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered", body = RegisterResponse),
        (status = 400, description = "Bad request", body = crate::shared::errors::ErrorResponse),
    ),
    tag = "auth",
)]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> AppResult<(StatusCode, Json<RegisterResponse>)> {
    let (user, tokens) = state
        .auth_service
        .register(req.email, req.name, req.password)
        .await?;
    let response = RegisterResponse {
        user: UserResponse {
            id: user.id.to_string(),
            email: user.email,
            name: user.name,
        },
        access_token: tokens.access_token,
        expires_in: tokens.expires_in,
    };
    Ok((StatusCode::CREATED, Json(response)))
}
