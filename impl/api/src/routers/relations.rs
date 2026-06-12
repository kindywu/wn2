use axum::{extract::{Path, State}, Json};
use crate::error::{AppError, AppResult, api_response};
use crate::middleware::auth::Operator;
use crate::dto::*;
use crate::state::AppState;

pub async fn review_relation(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Operator(op): Operator,
) -> AppResult<Json<serde_json::Value>> {
    let mut tx = state.db.begin().await?;
    sqlx::query(
        "UPDATE word_relations SET reviewed = true, reviewed_at = NOW(), updated_at = NOW() WHERE id = $1"
    )
    .bind(id).execute(&mut *tx).await?;

    sqlx::query(
        "INSERT INTO change_log (table_name, row_id, action, operator) VALUES ('word_relations', $1, 'review', $2)"
    )
    .bind(id).bind(&op).execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(Json(api_response(serde_json::json!({"id": id, "reviewed": true}))))
}

pub async fn update_relation(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Operator(op): Operator,
    Json(body): Json<WordRelationUpdate>,
) -> AppResult<Json<serde_json::Value>> {
    let old: Option<(String, String)> = sqlx::query_as(
        "SELECT related_word::text, relation_type FROM word_relations WHERE id = $1"
    )
    .bind(id).fetch_optional(&state.db).await?
    .map(|r: (String, String)| r);

    let (old_related, old_type) = old
        .ok_or_else(|| AppError::NotFound(format!("relation {id} not found")))?;

    let mut tx = state.db.begin().await?;

    let new_related = body.related_word.unwrap_or(old_related.clone());
    let new_type = body.relation_type.unwrap_or(old_type.clone());

    sqlx::query(
        "UPDATE word_relations SET related_word = $1, relation_type = $2, modified = true, modified_at = NOW(), reviewed = true, reviewed_at = NOW(), updated_at = NOW() WHERE id = $3"
    )
    .bind(&new_related).bind(&new_type).bind(id).execute(&mut *tx).await?;

    sqlx::query(
        "INSERT INTO change_log (table_name, row_id, field, old_value, new_value, action, operator) VALUES ('word_relations', $1, 'relation', $2, $3, 'update', $4)"
    )
    .bind(id).bind(&format!("{old_related} / {old_type}")).bind(&format!("{new_related} / {new_type}")).bind(&op).execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(Json(api_response(serde_json::json!({"id": id, "updated": true}))))
}

pub async fn create_relation(
    State(state): State<AppState>,
    Path(word_id): Path<i64>,
    Operator(op): Operator,
    Json(body): Json<WordRelationCreate>,
) -> AppResult<(axum::http::StatusCode, Json<serde_json::Value>)> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM word_relations WHERE word_id = $1 AND related_word = $2 AND relation_type = $3)"
    )
    .bind(word_id).bind(&body.related_word).bind(&body.relation_type).fetch_one(&state.db).await?;

    if exists {
        return Err(AppError::Conflict("relation already exists".into()));
    }

    let new_id: i64 = sqlx::query_scalar(
        "INSERT INTO word_relations (word_id, related_word, relation_type, source, reviewed, modified) VALUES ($1, $2, $3, 'human', true, false) RETURNING id"
    )
    .bind(word_id).bind(&body.related_word).bind(&body.relation_type).fetch_one(&state.db).await?;

    sqlx::query(
        "INSERT INTO change_log (table_name, row_id, field, new_value, action, operator) VALUES ('word_relations', $1, 'relation', $2, 'create', $3)"
    )
    .bind(new_id).bind(&format!("{} -> {}", body.related_word, body.relation_type)).bind(&op).execute(&state.db).await?;

    Ok((axum::http::StatusCode::CREATED, Json(api_response(serde_json::json!({"id": new_id})))))
}
