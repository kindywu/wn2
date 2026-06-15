use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};

#[allow(dead_code)]
pub async fn auth_middleware(req: Request, next: Next) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    // Token is validated in extractor or at route level; middleware just ensures header presence
    let Some(_token) = auth_header else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    Ok(next.run(req).await)
}

/// Extractor that provides the operator ID from header
pub struct Operator(pub String);

#[axum::async_trait]
impl<S: Send + Sync> axum::extract::FromRequestParts<S> for Operator {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let op = parts
            .headers
            .get("X-Operator-ID")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("system")
            .to_string();
        Ok(Operator(op))
    }
}
