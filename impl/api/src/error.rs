use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

#[derive(Debug)]
#[allow(dead_code)]
pub enum AppError {
    NotFound(String),
    Conflict(String),
    BadRequest(String),
    Unauthorized,
    Internal(anyhow::Error),
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match &e {
            sqlx::Error::RowNotFound => AppError::NotFound("record not found".into()),
            sqlx::Error::Database(db_err) => {
                if let Some(constraint) = db_err.constraint() {
                    AppError::Conflict(format!("constraint violation: {constraint}"))
                } else {
                    AppError::Internal(e.into())
                }
            }
            _ => AppError::Internal(e.into()),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", "invalid token".into()),
            AppError::Internal(e) => {
                tracing::error!("Internal error: {e:?}");
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL", "internal server error".into())
            }
        };
        let body = json!({ "error": { "code": code, "message": message } });
        (status, Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

/// Helper to build paginated response
pub fn api_response<T: serde::Serialize>(data: T) -> Value {
    json!({ "data": data })
}

pub fn api_list_response<T: serde::Serialize>(data: Vec<T>, total: i64, limit: i64, offset: i64) -> Value {
    json!({
        "data": data,
        "meta": { "total": total, "limit": limit, "offset": offset }
    })
}
