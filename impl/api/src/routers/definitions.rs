use axum::{extract::{Path, State}, Json};
use crate::error::{AppError, AppResult, api_response};
use crate::middleware::auth::Operator;
use crate::dto::*;
use crate::state::AppState;

pub async fn review_definition(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Operator(op): Operator,
) -> AppResult<Json<serde_json::Value>> {
    let mut tx = state.db.begin().await?;
    sqlx::query(
        "UPDATE definitions SET reviewed = true, reviewed_at = NOW(), updated_at = NOW() WHERE id = $1"
    )
    .bind(id).execute(&mut *tx).await?;

    sqlx::query(
        "INSERT INTO change_log (table_name, row_id, action, operator) VALUES ('definitions', $1, 'review', $2)"
    )
    .bind(id).bind(&op).execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(Json(api_response(serde_json::json!({"id": id, "reviewed": true}))))
}

pub async fn update_definition(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Operator(op): Operator,
    Json(body): Json<DefinitionUpdate>,
) -> AppResult<Json<serde_json::Value>> {
    let old_text: Option<String> = sqlx::query_scalar("SELECT text FROM definitions WHERE id = $1")
        .bind(id).fetch_optional(&state.db).await?
        .ok_or_else(|| AppError::NotFound(format!("definition {id} not found")))?;

    let mut tx = state.db.begin().await?;
    sqlx::query(
        "UPDATE definitions SET text = $1, modified = true, modified_at = NOW(), reviewed = true, reviewed_at = NOW(), updated_at = NOW() WHERE id = $2"
    )
    .bind(&body.text).bind(id).execute(&mut *tx).await?;

    sqlx::query(
        "INSERT INTO change_log (table_name, row_id, field, old_value, new_value, action, operator) VALUES ('definitions', $1, 'text', $2, $3, 'update', $4)"
    )
    .bind(id).bind(&old_text).bind(&body.text).bind(&op).execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(Json(api_response(serde_json::json!({"id": id, "modified": true}))))
}

pub async fn create_definition(
    State(state): State<AppState>,
    Path(word_id): Path<i64>,
    Operator(op): Operator,
    Json(body): Json<DefinitionCreate>,
) -> AppResult<(axum::http::StatusCode, Json<serde_json::Value>)> {
    // Check for duplicate
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM definitions WHERE word_id = $1 AND text = $2 AND source = 'human')"
    )
    .bind(word_id).bind(&body.text).fetch_one(&state.db).await?;

    if exists {
        return Err(AppError::Conflict("definition already exists".into()));
    }

    let new_id: i64 = sqlx::query_scalar(
        "INSERT INTO definitions (word_id, text, source, reviewed, modified) VALUES ($1, $2, 'human', true, false) RETURNING id"
    )
    .bind(word_id).bind(&body.text).fetch_one(&state.db).await?;

    let mut tx = state.db.begin().await?;
    sqlx::query(
        "INSERT INTO change_log (table_name, row_id, field, new_value, action, operator) VALUES ('definitions', $1, 'text', $2, 'create', $3)"
    )
    .bind(new_id).bind(&body.text).bind(&op).execute(&mut *tx).await?;
    tx.commit().await?;

    Ok((axum::http::StatusCode::CREATED, Json(api_response(serde_json::json!({"id": new_id})))))
}
