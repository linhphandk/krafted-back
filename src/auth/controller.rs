use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use tracing::{info, instrument};
use utoipa::ToSchema;

use crate::shared::errors::AppResult;
use crate::shared::types::AuthenticatedUser;
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

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    #[schema(example = "user@example.com")]
    pub email: String,
    #[schema(example = "securepassword123")]
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

#[derive(serde::Serialize, ToSchema)]
pub struct RegisterResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub expires_in: u64,
}

#[derive(serde::Serialize, ToSchema)]
pub struct LoginResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub refresh_token: String,
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
        (status = 200, description = "User registered", body = RegisterResponse),
        (status = 400, description = "Bad request", body = crate::shared::errors::ErrorResponse),
    ),
    tag = "auth",
)]
#[instrument(skip(state, req), fields(email = %req.email))]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> AppResult<(StatusCode, Json<RegisterResponse>)> {
    info!("register endpoint called");
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
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "User logged in", body = LoginResponse),
        (status = 400, description = "Bad request", body = crate::shared::errors::ErrorResponse),
    ),
    tag = "auth",
)]
#[instrument(skip(state, req), fields(email = %req.email))]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<(StatusCode, Json<LoginResponse>)> {
    info!("login endpoint called");
    let (user, tokens) = state.auth_service.login(req.email, req.password).await?;
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

#[utoipa::path(
    post,
    path = "/auth/logout",
    request_body = LogoutRequest,
    responses(
        (status = 200, description = "User logged out"),
        (status = 400, description = "Bad request", body = crate::shared::errors::ErrorResponse),
    ),
    tag = "auth",
)]
#[instrument(skip(state, req))]
pub async fn logout(
    State(state): State<AppState>,
    Json(req): Json<LogoutRequest>,
) -> AppResult<StatusCode> {
    info!("logout endpoint called");
    state.auth_service.logout(req.refresh_token).await?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub email: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/auth/me",
    responses(
        (status = 200, description = "Current user", body = UserResponse),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "auth",
)]
pub async fn me(
    Extension(user): Extension<AuthenticatedUser>,
    State(_state): State<AppState>,
) -> AppResult<Json<UserResponse>> {
    info!(user_id = %user.id, "me");
    Ok(Json(UserResponse {
        id: user.id.to_string(),
        email: user.email.clone(),
        name: user.name.clone(),
    }))
}

#[utoipa::path(
    patch,
    path = "/api/auth/me",
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Profile updated", body = UserResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "auth",
)]
pub async fn update_profile(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState>,
    Json(req): Json<UpdateProfileRequest>,
) -> AppResult<Json<UserResponse>> {
    info!(user_id = %user.id, "update_profile");

    let data = crate::user::models::UpdateUser {
        name: req
            .name
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty()),
        email: req
            .email
            .map(|e| e.trim().to_string())
            .filter(|e| !e.is_empty()),
    };

    let updated = state.user_service.update_profile(user.id, data).await?;

    Ok(Json(UserResponse {
        id: updated.id.to_string(),
        email: updated.email,
        name: updated.name,
    }))
}

#[derive(Deserialize, ToSchema)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(serde::Serialize, ToSchema)]
pub struct RefreshResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[utoipa::path(
    post,
    path = "/auth/refresh",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Token refreshed", body = RefreshResponse),
        (status = 400, description = "Bad request", body = crate::shared::errors::ErrorResponse),
    ),
    tag = "auth",
)]
#[instrument(skip(state, req))]
pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> AppResult<(StatusCode, Json<RefreshResponse>)> {
    info!("refresh token endpoint called");
    let (user, tokens) = state.auth_service.refresh_token(req.refresh_token).await?;
    let response = RefreshResponse {
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
