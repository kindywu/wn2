use axum::{extract::{Path, State}, Json};
use crate::error::{AppError, AppResult, api_response};
use crate::middleware::auth::Operator;
use crate::dto::*;
use crate::state::AppState;

pub async fn review_example(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Operator(op): Operator,
) -> AppResult<Json<serde_json::Value>> {
    let mut tx = state.db.begin().await?;
    sqlx::query(
        "UPDATE examples SET reviewed = true, reviewed_at = NOW(), updated_at = NOW() WHERE id = $1"
    )
    .bind(id).execute(&mut *tx).await?;

    sqlx::query(
        "INSERT INTO change_log (table_name, row_id, action, operator) VALUES ('examples', $1, 'review', $2)"
    )
    .bind(id).bind(&op).execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(Json(api_response(serde_json::json!({"id": id, "reviewed": true}))))
}

pub async fn update_example(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Operator(op): Operator,
    Json(body): Json<ExampleUpdate>,
) -> AppResult<Json<serde_json::Value>> {
    let example: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT text, translation FROM examples WHERE id = $1"
    )
    .bind(id).fetch_optional(&state.db).await?
    .map(|r: (String, Option<String>)| r);

    let (old_text, old_trans) = example
        .ok_or_else(|| AppError::NotFound(format!("example {id} not found")))?;

    let mut tx = state.db.begin().await?;

    if let Some(ref new_text) = body.text {
        sqlx::query(
            "UPDATE examples SET text = $1, modified = true, modified_at = NOW(), reviewed = true, reviewed_at = NOW(), updated_at = NOW() WHERE id = $2"
        )
        .bind(new_text).bind(id).execute(&mut *tx).await?;

        sqlx::query(
            "INSERT INTO change_log (table_name, row_id, field, old_value, new_value, action, operator) VALUES ('examples', $1, 'text', $2, $3, 'update', $4)"
        )
        .bind(id).bind(&old_text).bind(new_text).bind(&op).execute(&mut *tx).await?;
    }

    if let Some(ref new_trans) = body.translation {
        sqlx::query(
            "UPDATE examples SET translation = $1, trans_modified = true, trans_modified_at = NOW(), trans_reviewed = true, trans_reviewed_at = NOW(), updated_at = NOW() WHERE id = $2"
        )
        .bind(new_trans).bind(id).execute(&mut *tx).await?;

        sqlx::query(
            "INSERT INTO change_log (table_name, row_id, field, old_value, new_value, action, operator) VALUES ('examples', $1, 'translation', $2, $3, 'update', $4)"
        )
        .bind(id).bind(&old_trans).bind(new_trans).bind(&op).execute(&mut *tx).await?;
    }

    tx.commit().await?;
    Ok(Json(api_response(serde_json::json!({"id": id, "updated": true}))))
}

pub async fn create_example(
    State(state): State<AppState>,
    Path(word_id): Path<i64>,
    Operator(op): Operator,
    Json(body): Json<ExampleCreate>,
) -> AppResult<(axum::http::StatusCode, Json<serde_json::Value>)> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM examples WHERE word_id = $1 AND text = $2)"
    )
    .bind(word_id).bind(&body.text).fetch_one(&state.db).await?;

    if exists {
        return Err(AppError::Conflict("example already exists".into()));
    }

    let new_id: i64 = sqlx::query_scalar(
        "INSERT INTO examples (word_id, text, translation, source, reviewed, modified, trans_source, trans_reviewed) VALUES ($1, $2, $3, 'human', true, false, CASE WHEN $3 IS NOT NULL THEN 'human'::data_source ELSE NULL END, $3 IS NOT NULL) RETURNING id"
    )
    .bind(word_id).bind(&body.text).bind(&body.translation).fetch_one(&state.db).await?;

    sqlx::query(
        "INSERT INTO change_log (table_name, row_id, field, new_value, action, operator) VALUES ('examples', $1, 'text', $2, 'create', $3)"
    )
    .bind(new_id).bind(&body.text).bind(&op).execute(&state.db).await?;

    Ok((axum::http::StatusCode::CREATED, Json(api_response(serde_json::json!({"id": new_id})))))
}
