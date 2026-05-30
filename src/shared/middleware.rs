use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

pub async fn auth_middleware(request: Request, next: Next) -> Response {
    next.run(request).await
}

pub async fn rbac_middleware(request: Request, next: Next) -> Response {
    next.run(request).await
}
