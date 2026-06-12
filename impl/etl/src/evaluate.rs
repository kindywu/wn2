use dict_models::enums::SeverityLevel;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::llm_client::{LlmClient, LlmTaskType, generate_json};

#[derive(Deserialize)]
struct EvalResult {
    score: i32,
    comment: String,
    suggestion: Option<String>,
}

pub async fn run(pool: &PgPool, llm: &(impl LlmClient + ?Sized)) -> anyhow::Result<()> {
    info!("Starting evaluate (LLM quality evaluation)");

    // Evaluate translations
    info!("Evaluating translations");
    let translations: Vec<(i64, i64, String, String)> = sqlx::query_as(
        "SELECT t.id, t.word_id, t.text, w.word FROM translations t JOIN words w ON w.id = t.word_id WHERE t.source IN ('stardict', 'wn', 'llm') LIMIT 1000"
    )
    .fetch_all(pool)
    .await?;

    for (trans_id, word_id, trans_text, word) in &translations {
        // Check if already evaluated
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM evaluations WHERE target_table = 'translations' AND target_id = $1 AND dimension = 'translation_accuracy' AND reviewed = false)"
        )
        .bind(trans_id)
        .fetch_one(pool)
        .await?;

        if exists { continue; }

        let prompt = format!(
            "Evaluate the Chinese translation accuracy for the English word '{}'.\nTranslation: '{}'\nScore 1-5 (1=wrong, 5=perfect). If score < 4, provide a better suggestion.",
            word, trans_text
        );

        match generate_json::<_, EvalResult>(llm, &prompt, LlmTaskType::Evaluate).await {
            Ok(result) => {
                if result.score < 4 {
                    let severity = if result.score <= 2 { SeverityLevel::Critical } else { SeverityLevel::Warning };
                    let _ = sqlx::query(
                        "INSERT INTO evaluations (target_table, target_id, word_id, dimension, score, comment, suggestion, severity) VALUES ('translations', $1, $2, 'translation_accuracy', $3, $4, $5, $6)"
                    )
                    .bind(trans_id).bind(word_id).bind(result.score as i16).bind(&result.comment).bind(&result.suggestion).bind(severity)
                    .execute(pool).await;
                }
            }
            Err(e) => { warn!("LLM eval error for trans {}: {}", trans_id, e); }
        }
    }

    // Evaluate definitions
    info!("Evaluating definitions");
    let definitions: Vec<(i64, i64, String, String)> = sqlx::query_as(
        "SELECT d.id, d.word_id, d.text, w.word FROM definitions d JOIN words w ON w.id = d.word_id WHERE d.source IN ('stardict', 'wn') LIMIT 1000"
    )
    .fetch_all(pool)
    .await?;

    for (def_id, word_id, def_text, word) in &definitions {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM evaluations WHERE target_table = 'definitions' AND target_id = $1 AND dimension = 'definition_completeness' AND reviewed = false)"
        )
        .bind(def_id)
        .fetch_one(pool)
        .await?;

        if exists { continue; }

        let prompt = format!(
            "Evaluate this English definition for '{}':\n'{}'\nScore 1-5 on completeness and clarity (1=too vague/brief, 5=clear and complete). If score < 4, suggest improvement.",
            word, def_text
        );

        match generate_json::<_, EvalResult>(llm, &prompt, LlmTaskType::Evaluate).await {
            Ok(result) => {
                if result.score < 4 {
                    let severity = if result.score <= 2 { SeverityLevel::Critical } else { SeverityLevel::Warning };
                    let _ = sqlx::query(
                        "INSERT INTO evaluations (target_table, target_id, word_id, dimension, score, comment, suggestion, severity) VALUES ('definitions', $1, $2, 'definition_completeness', $3, $4, $5, $6)"
                    )
                    .bind(def_id).bind(word_id).bind(result.score as i16).bind(&result.comment).bind(&result.suggestion).bind(severity)
                    .execute(pool).await;
                }
            }
            Err(e) => { warn!("LLM eval error for def {}: {}", def_id, e); }
        }
    }

    // Evaluate examples
    info!("Evaluating examples");
    let examples: Vec<(i64, i64, String, String)> = sqlx::query_as(
        "SELECT e.id, e.word_id, e.text, w.word FROM examples e JOIN words w ON w.id = e.word_id WHERE e.source IN ('stardict', 'wn') LIMIT 1000"
    )
    .fetch_all(pool)
    .await?;

    for (ex_id, word_id, ex_text, word) in &examples {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM evaluations WHERE target_table = 'examples' AND target_id = $1 AND dimension = 'example_naturalness' AND reviewed = false)"
        )
        .bind(ex_id)
        .fetch_one(pool)
        .await?;

        if exists { continue; }

        let prompt = format!(
            "Evaluate how natural this example sentence is for the word '{}':\n'{}'\nScore 1-5 (1=awkward/unnatural, 5=perfectly natural). If score < 4, suggest a better example.",
            word, ex_text
        );

        match generate_json::<_, EvalResult>(llm, &prompt, LlmTaskType::Evaluate).await {
            Ok(result) => {
                if result.score < 4 {
                    let severity = if result.score <= 2 { SeverityLevel::Critical } else { SeverityLevel::Warning };
                    let _ = sqlx::query(
                        "INSERT INTO evaluations (target_table, target_id, word_id, dimension, score, comment, suggestion, severity) VALUES ('examples', $1, $2, 'example_naturalness', $3, $4, $5, $6)"
                    )
                    .bind(ex_id).bind(word_id).bind(result.score as i16).bind(&result.comment).bind(&result.suggestion).bind(severity)
                    .execute(pool).await;
                }
            }
            Err(e) => { warn!("LLM eval error for ex {}: {}", ex_id, e); }
        }
    }

    info!("evaluate complete");
    Ok(())
}
