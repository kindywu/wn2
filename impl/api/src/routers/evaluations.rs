use axum::{extract::{Path, State, Query}, Json};
use crate::error::{AppError, AppResult, api_response, api_list_response};
use crate::middleware::auth::Operator;
use crate::dto::*;
use crate::state::AppState;

pub async fn list_evaluations(
    State(state): State<AppState>,
    Query(params): Query<EvaluationQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);
    let reviewed = params.reviewed.unwrap_or(false);

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM evaluations WHERE reviewed = $1"
    )
    .bind(reviewed).fetch_one(&state.db).await?;

    let mut query = String::from(
        "SELECT id, target_table, target_id, word_id, dimension, score, comment, suggestion, severity::text, reviewed, created_at FROM evaluations WHERE reviewed = $1"
    );
    if params.severity.is_some() {
        query.push_str(" AND severity = $4::severity_level");
    }
    query.push_str(" ORDER BY CASE severity WHEN 'critical' THEN 0 WHEN 'warning' THEN 1 ELSE 2 END, score ASC LIMIT $2 OFFSET $3");

    let rows = if let Some(ref sev) = params.severity {
        sqlx::query_as::<_, EvaluationOut>(&query)
            .bind(reviewed).bind(limit).bind(offset).bind(sev)
            .fetch_all(&state.db).await?
    } else {
        sqlx::query_as::<_, EvaluationOut>(&query)
            .bind(reviewed).bind(limit).bind(offset)
            .fetch_all(&state.db).await?
    };

    Ok(Json(api_list_response(rows, total, limit, offset)))
}

pub async fn agree_evaluation(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Operator(op): Operator,
) -> AppResult<Json<serde_json::Value>> {
    let eval: Option<(String, i64, Option<String>)> = sqlx::query_as(
        "SELECT target_table, target_id, suggestion FROM evaluations WHERE id = $1 AND reviewed = false"
    )
    .bind(id).fetch_optional(&state.db).await?
    .map(|r: (String, i64, Option<String>)| r);

    let (target_table, target_id, suggestion) = eval
        .ok_or_else(|| AppError::NotFound(format!("evaluation {id} not found or already reviewed")))?;

    let mut tx = state.db.begin().await?;

    if let Some(ref suggestion_text) = suggestion {
        match target_table.as_str() {
            "definitions" => {
                sqlx::query("UPDATE definitions SET text = $1, modified = true, modified_at = NOW(), reviewed = true, reviewed_at = NOW(), updated_at = NOW() WHERE id = $2")
                    .bind(suggestion_text).bind(target_id).execute(&mut *tx).await?;
            }
            "translations" => {
                sqlx::query("UPDATE translations SET text = $1, modified = true, modified_at = NOW(), reviewed = true, reviewed_at = NOW(), updated_at = NOW() WHERE id = $2")
                    .bind(suggestion_text).bind(target_id).execute(&mut *tx).await?;
            }
            "examples" => {
                sqlx::query("UPDATE examples SET text = $1, modified = true, modified_at = NOW(), reviewed = true, reviewed_at = NOW(), updated_at = NOW() WHERE id = $2")
                    .bind(suggestion_text).bind(target_id).execute(&mut *tx).await?;
            }
            _ => return Err(AppError::BadRequest(format!("cannot auto-apply to {target_table}"))),
        }

        sqlx::query(
            "INSERT INTO change_log (table_name, row_id, field, new_value, action, operator) VALUES ($1, $2, 'text', $3, 'update', $4)"
        )
        .bind(&target_table).bind(target_id).bind(suggestion_text).bind(&op)
        .execute(&mut *tx).await?;
    }

    sqlx::query("UPDATE evaluations SET reviewed = true, reviewed_at = NOW() WHERE id = $1")
        .bind(id).execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(Json(api_response(serde_json::json!({"id": id, "reviewed": true}))))
}

pub async fn dismiss_evaluation(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let r = sqlx::query(
        "UPDATE evaluations SET reviewed = true, reviewed_at = NOW() WHERE id = $1 AND reviewed = false"
    )
    .bind(id).execute(&state.db).await?;

    if r.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("evaluation {id} not found or already reviewed")));
    }

    Ok(Json(api_response(serde_json::json!({"id": id, "reviewed": true}))))
}
