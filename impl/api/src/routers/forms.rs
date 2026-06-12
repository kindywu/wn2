use axum::{extract::{Path, State}, Json};
use crate::error::{AppError, AppResult, api_response};
use crate::middleware::auth::Operator;
use crate::state::AppState;

pub async fn review_form(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Operator(op): Operator,
) -> AppResult<Json<serde_json::Value>> {
    let mut tx = state.db.begin().await?;
    sqlx::query(
        "UPDATE word_forms SET reviewed = true, reviewed_at = NOW(), updated_at = NOW() WHERE id = $1"
    )
    .bind(id).execute(&mut *tx).await?;

    sqlx::query(
        "INSERT INTO change_log (table_name, row_id, action, operator) VALUES ('word_forms', $1, 'review', $2)"
    )
    .bind(id).bind(&op).execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(Json(api_response(serde_json::json!({"id": id, "reviewed": true}))))
}

pub async fn update_form(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Operator(op): Operator,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let old_form: Option<String> = sqlx::query_scalar("SELECT form FROM word_forms WHERE id = $1")
        .bind(id).fetch_optional(&state.db).await?
        .ok_or_else(|| AppError::NotFound(format!("form {id} not found")))?;

    let new_form = body.get("form").and_then(|v| v.as_str()).unwrap_or(old_form.as_deref().unwrap_or("")).to_string();

    let mut tx = state.db.begin().await?;
    sqlx::query(
        "UPDATE word_forms SET form = $1, modified = true, modified_at = NOW(), reviewed = true, reviewed_at = NOW(), updated_at = NOW() WHERE id = $2"
    )
    .bind(&new_form).bind(id).execute(&mut *tx).await?;

    sqlx::query(
        "INSERT INTO change_log (table_name, row_id, field, old_value, new_value, action, operator) VALUES ('word_forms', $1, 'form', $2, $3, 'update', $4)"
    )
    .bind(id).bind(&old_form).bind(&new_form).bind(&op).execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(Json(api_response(serde_json::json!({"id": id, "updated": true}))))
}
