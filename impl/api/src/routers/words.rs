use axum::{extract::{Path, Query, State}, http::StatusCode, Json, Extension};
use sqlx::PgPool;

use crate::error::{AppError, AppResult, api_response, api_list_response};
use crate::middleware::auth::Operator;
use crate::dto::*;
use crate::state::AppState;

pub async fn search_words(
    State(state): State<AppState>,
    Query(params): Query<WordSearchQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);
    let mode = params.mode.as_deref().unwrap_or("exact");

    let (rows, total): (Vec<WordOut>, i64) = if mode == "prefix" {
        let pattern = format!("{}%", params.q);
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM words WHERE word LIKE $1"
        )
        .bind(&pattern)
        .fetch_one(&state.db).await?;

        let rows = sqlx::query_as::<_, WordOut>(
            "SELECT id, word, pos::text, phonetic, collins, oxford, bnc, frq, tags, curated, curated_at, created_at, updated_at FROM words WHERE word LIKE $1 ORDER BY word LIMIT $2 OFFSET $3"
        )
        .bind(&pattern).bind(limit).bind(offset)
        .fetch_all(&state.db).await?;

        (rows, total)
    } else {
        // exact match (CITEXT handles case-insensitive)
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM words WHERE word = $1"
        )
        .bind(&params.q)
        .fetch_one(&state.db).await?;

        let rows = sqlx::query_as::<_, WordOut>(
            "SELECT id, word, pos::text, phonetic, collins, oxford, bnc, frq, tags, curated, curated_at, created_at, updated_at FROM words WHERE word = $1 ORDER BY word, pos LIMIT $2 OFFSET $3"
        )
        .bind(&params.q).bind(limit).bind(offset)
        .fetch_all(&state.db).await?;

        (rows, total)
    };

    Ok(Json(api_list_response(rows, total, limit, offset)))
}

pub async fn get_word(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let word = sqlx::query_as::<_, WordOut>(
        "SELECT id, word, pos::text, phonetic, collins, oxford, bnc, frq, tags, curated, curated_at, created_at, updated_at FROM words WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.db).await?
    .ok_or_else(|| AppError::NotFound(format!("word {id} not found")))?;

    Ok(Json(api_response(word)))
}

pub async fn curate_word(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Operator(op): Operator,
) -> AppResult<Json<serde_json::Value>> {
    let mut tx = state.db.begin().await?;

    let old_curated: bool = sqlx::query_scalar("SELECT curated FROM words WHERE id = $1")
        .bind(id)
        .fetch_optional(&mut *tx).await?
        .ok_or_else(|| AppError::NotFound(format!("word {id} not found")))?;

    sqlx::query("UPDATE words SET curated = true WHERE id = $1")
        .bind(id)
        .execute(&mut *tx).await?;

    sqlx::query(
        "INSERT INTO change_log (table_name, row_id, field, old_value, new_value, action, operator) VALUES ('words', $1, 'curated', $2, 'true', 'review', $3)"
    )
    .bind(id).bind(old_curated.to_string()).bind(&op)
    .execute(&mut *tx).await?;

    tx.commit().await?;

    Ok(Json(api_response(serde_json::json!({"id": id, "curated": true}))))
}

pub async fn get_word_definitions(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let rows = sqlx::query_as::<_, DefinitionOut>(
        "SELECT id, word_id, text, source::text, reviewed, modified, created_at, updated_at FROM definitions WHERE word_id = $1 ORDER BY data_quality_priority(source, reviewed, modified)"
    )
    .bind(id)
    .fetch_all(&state.db).await?;

    Ok(Json(api_response(rows)))
}

pub async fn get_word_translations(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let rows = sqlx::query_as::<_, TranslationOut>(
        "SELECT id, word_id, text, language, source::text, reviewed, modified, created_at, updated_at FROM translations WHERE word_id = $1 ORDER BY data_quality_priority(source, reviewed, modified)"
    )
    .bind(id)
    .fetch_all(&state.db).await?;

    Ok(Json(api_response(rows)))
}

pub async fn get_word_examples(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let rows = sqlx::query_as::<_, ExampleOut>(
        "SELECT id, word_id, text, translation, source::text, reviewed, modified, trans_source::text, trans_reviewed, trans_modified, created_at, updated_at FROM examples WHERE word_id = $1 ORDER BY data_quality_priority(source, reviewed, modified)"
    )
    .bind(id)
    .fetch_all(&state.db).await?;

    Ok(Json(api_response(rows)))
}

pub async fn get_word_relations(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> AppResult<Json<serde_json::Value>> {
    if let Some(rel_type) = params.get("type") {
        let rows = sqlx::query_as::<_, WordRelationOut>(
            "SELECT id, word_id, related_word::text, relation_type, source::text, reviewed, modified, created_at, updated_at FROM word_relations WHERE word_id = $1 AND relation_type = $2 ORDER BY data_quality_priority(source, reviewed, modified)"
        )
        .bind(id).bind(rel_type)
        .fetch_all(&state.db).await?;
        return Ok(Json(api_response(rows)));
    }

    let rows = sqlx::query_as::<_, WordRelationOut>(
        "SELECT id, word_id, related_word::text, relation_type, source::text, reviewed, modified, created_at, updated_at FROM word_relations WHERE word_id = $1 ORDER BY data_quality_priority(source, reviewed, modified)"
    )
    .bind(id)
    .fetch_all(&state.db).await?;

    Ok(Json(api_response(rows)))
}

pub async fn get_word_forms(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let rows = sqlx::query_as::<_, WordFormOut>(
        "SELECT id, word_id, form, form_type::text, source::text, reviewed, modified, created_at, updated_at FROM word_forms WHERE word_id = $1"
    )
    .bind(id)
    .fetch_all(&state.db).await?;

    Ok(Json(api_response(rows)))
}
