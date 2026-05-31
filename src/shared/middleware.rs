use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::shared::types::AuthenticatedUser;
use crate::state::AppState;

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok());

    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => {
            return (StatusCode::UNAUTHORIZED, "Missing or invalid Authorization header")
                .into_response();
        }
    };

    let (user, role) = match state.auth_service.get_current_user(token.to_string()).await {
        Ok(u) => u,
        Err(_) => {
            return (StatusCode::UNAUTHORIZED, "Invalid or expired token").into_response();
        }
    };

    request.extensions_mut().insert(AuthenticatedUser {
        id: user.id,
        email: user.email,
        name: user.name,
        role,
    });

    next.run(request).await
}
