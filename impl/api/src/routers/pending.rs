use axum::{extract::{Path, State, Query}, Json};
use crate::error::{AppError, AppResult, api_response, api_list_response};
use crate::middleware::auth::Operator;
use crate::dto::*;
use crate::state::AppState;

pub async fn list_pending(
    State(state): State<AppState>,
    Query(params): Query<PendingChangeQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);
    let status = params.status.as_deref().unwrap_or("pending");

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pending_changes WHERE status = $1::change_status"
    )
    .bind(status).fetch_one(&state.db).await?;

    let rows = sqlx::query_as::<_, PendingChangeOut>(
        "SELECT id, table_name, row_id, word_id, field, old_value, new_value, source, status::text, created_at FROM pending_changes WHERE status = $1::change_status ORDER BY source, created_at LIMIT $2 OFFSET $3"
    )
    .bind(status).bind(limit).bind(offset)
    .fetch_all(&state.db).await?;

    Ok(Json(api_list_response(rows, total, limit, offset)))
}

pub async fn approve_pending(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Operator(op): Operator,
    Json(body): Json<PendingChangeApprove>,
) -> AppResult<Json<serde_json::Value>> {
    let pending: Option<(String, Option<i64>, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT table_name, row_id, field, old_value, new_value FROM pending_changes WHERE id = $1 AND status = 'pending'"
    )
    .bind(id).fetch_optional(&state.db).await?
    .map(|r: (String, Option<i64>, Option<String>, Option<String>, Option<String>)| r);

    let (table_name, row_id, field, old_value, new_value) = pending
        .ok_or_else(|| AppError::NotFound(format!("pending change {id} not found or not pending")))?;

    let Some(target_id) = row_id else {
        return Err(AppError::BadRequest("pending change has no target row_id".into()));
    };

    let applied_value = body.value.unwrap_or_else(|| new_value.unwrap_or_default());

    let mut tx = state.db.begin().await?;

    match table_name.as_str() {
        "definitions" => {
            sqlx::query("UPDATE definitions SET text = $1, modified = false, reviewed = true, reviewed_at = NOW(), updated_at = NOW() WHERE id = $2")
                .bind(&applied_value).bind(target_id).execute(&mut *tx).await?;
        }
        "translations" => {
            sqlx::query("UPDATE translations SET text = $1, modified = false, reviewed = true, reviewed_at = NOW(), updated_at = NOW() WHERE id = $2")
                .bind(&applied_value).bind(target_id).execute(&mut *tx).await?;
        }
        "examples" => {
            sqlx::query("UPDATE examples SET text = $1, modified = false, reviewed = true, reviewed_at = NOW(), updated_at = NOW() WHERE id = $2")
                .bind(&applied_value).bind(target_id).execute(&mut *tx).await?;
        }
        "word_relations" => {
            sqlx::query("UPDATE word_relations SET reviewed = true, reviewed_at = NOW(), updated_at = NOW() WHERE id = $1")
                .bind(target_id).execute(&mut *tx).await?;
        }
        "word_forms" => {
            sqlx::query("UPDATE word_forms SET form = $1, modified = false, reviewed = true, reviewed_at = NOW(), updated_at = NOW() WHERE id = $2")
                .bind(&applied_value).bind(target_id).execute(&mut *tx).await?;
        }
        "words" => {
            if let Some(ref f) = field {
                match f.as_str() {
                    "collins" => {
                        let val: i16 = applied_value.parse().unwrap_or(0);
                        sqlx::query("UPDATE words SET collins = $1 WHERE id = $2")
                            .bind(val).bind(target_id).execute(&mut *tx).await?;
                    }
                    _ => {
                        // Generic text update for other fields
                        sqlx::query(&format!("UPDATE words SET {} = $1 WHERE id = $2", f))
                            .bind(&applied_value).bind(target_id).execute(&mut *tx).await?;
                    }
                }
            }
        }
        _ => return Err(AppError::BadRequest(format!("unknown table: {table_name}"))),
    }

    // Insert change_log
    sqlx::query(
        "INSERT INTO change_log (table_name, row_id, field, old_value, new_value, action, operator) VALUES ($1, $2, $3, $4, $5, 'update', $6)"
    )
    .bind(&table_name).bind(target_id).bind(&field).bind(&old_value).bind(&applied_value).bind(&op)
    .execute(&mut *tx).await?;

    // Update pending_changes status
    sqlx::query(
        "UPDATE pending_changes SET status = 'approved', resolved_at = NOW() WHERE id = $1"
    )
    .bind(id).execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(Json(api_response(serde_json::json!({"id": id, "status": "approved"}))))
}

pub async fn reject_pending(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let r = sqlx::query(
        "UPDATE pending_changes SET status = 'rejected', resolved_at = NOW() WHERE id = $1 AND status = 'pending'"
    )
    .bind(id).execute(&state.db).await?;

    if r.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("pending change {id} not found or not pending")));
    }

    Ok(Json(api_response(serde_json::json!({"id": id, "status": "rejected"}))))
}
