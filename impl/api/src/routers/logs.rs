use axum::{extract::{Path, State, Query}, Json};
use crate::error::{AppResult, api_response, api_list_response};
use crate::dto::*;
use crate::state::AppState;

pub async fn list_import_logs(
    State(state): State<AppState>,
) -> AppResult<Json<serde_json::Value>> {
    let rows = sqlx::query_as::<_, ImportLogOut>(
        "SELECT id, source, source_version, mode::text, started_at, finished_at, new_words, updated_words, conflicts, status::text FROM import_log ORDER BY started_at DESC LIMIT 50"
    )
    .fetch_all(&state.db).await?;
    let count = rows.len() as i64;

    Ok(Json(api_list_response(rows, count, 50, 0)))
}

pub async fn get_import_log(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let row = sqlx::query_as::<_, ImportLogOut>(
        "SELECT id, source, source_version, mode::text, started_at, finished_at, new_words, updated_words, conflicts, status::text FROM import_log WHERE id = $1"
    )
    .bind(id).fetch_optional(&state.db).await?;

    match row {
        Some(log) => Ok(Json(api_response(log))),
        None => Err(crate::error::AppError::NotFound(format!("import log {id} not found"))),
    }
}

pub async fn list_change_logs(
    State(state): State<AppState>,
    Query(params): Query<ChangeLogQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0);

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM change_log")
        .fetch_one(&state.db).await?;

    let rows = if let Some(ref op) = params.operator {
        sqlx::query_as::<_, ChangeLogOut>(
            "SELECT id, table_name, row_id, field, old_value, new_value, action::text, operator, changed_at FROM change_log WHERE operator = $1 ORDER BY changed_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(op).bind(limit).bind(offset)
        .fetch_all(&state.db).await?
    } else {
        sqlx::query_as::<_, ChangeLogOut>(
            "SELECT id, table_name, row_id, field, old_value, new_value, action::text, operator, changed_at FROM change_log ORDER BY changed_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(limit).bind(offset)
        .fetch_all(&state.db).await?
    };

    Ok(Json(api_list_response(rows, total, limit, offset)))
}
