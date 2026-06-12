use sqlx::PgPool;
use tracing::{info, warn};

use crate::llm_client::{LlmClient, LlmTaskType};

pub async fn run(pool: &PgPool, llm: &(impl LlmClient + ?Sized)) -> anyhow::Result<()> {
    info!("Starting fill_gaps (LLM gap filling)");

    // Get all words
    let words: Vec<(i64, String, Option<String>)> = sqlx::query_as(
        "SELECT id, word, pos::text FROM words"
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r: (i64, String, Option<String>)| r)
    .collect();

    let mut trans_filled = 0u64;
    let mut defs_filled = 0u64;
    let mut examples_filled = 0u64;
    let mut ex_trans_filled = 0u64;

    for (word_id, word, pos) in &words {
        let pos_str = pos.as_deref().unwrap_or("");

        // 1. Fill missing translations
        let has_trans: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM translations WHERE word_id = $1 AND source = 'llm')"
        )
        .bind(word_id)
        .fetch_one(pool)
        .await?;

        if !has_trans {
            let prompt = format!(
                "Translate the English word '{}' ({}) to Chinese. Output comma-separated translations if multiple senses exist.",
                word, pos_str
            );
            match llm.generate(&prompt, LlmTaskType::Generate).await {
                Ok(result) => {
                    let texts: Vec<&str> = result.split(&[',', '，', '\n'][..])
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect();
                    for text in texts {
                        let t = text.trim_matches(&['"', '\'', ' '][..]);
                        if !t.is_empty() {
                            let _ = sqlx::query(
                                "INSERT INTO translations (word_id, text, language, source) VALUES ($1, $2, 'zh', 'llm') ON CONFLICT DO NOTHING"
                            )
                            .bind(word_id).bind(t)
                            .execute(pool).await;
                        }
                    }
                    trans_filled += 1;
                }
                Err(e) => { warn!("LLM trans error for {}: {}", word, e); }
            }
        }

        // 2. Fill missing definitions
        let has_def: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM definitions WHERE word_id = $1)"
        )
        .bind(word_id)
        .fetch_one(pool)
        .await?;

        if !has_def {
            let prompt = format!("Define '{}' ({}) in one concise English sentence.", word, pos_str);
            match llm.generate(&prompt, LlmTaskType::Generate).await {
                Ok(result) => {
                    let _ = sqlx::query(
                        "INSERT INTO definitions (word_id, text, source) VALUES ($1, $2, 'llm') ON CONFLICT DO NOTHING"
                    )
                    .bind(word_id).bind(result.trim())
                    .execute(pool).await;
                    defs_filled += 1;
                }
                Err(e) => { warn!("LLM def error for {}: {}", word, e); }
            }
        }

        // 3. Fill missing examples
        let has_example: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM examples WHERE word_id = $1)"
        )
        .bind(word_id)
        .fetch_one(pool)
        .await?;

        if !has_example {
            let prompt = format!(
                "Write 2-3 example sentences using the word '{}' ({}). Output one sentence per line, no numbering.",
                word, pos_str
            );
            match llm.generate(&prompt, LlmTaskType::Generate).await {
                Ok(result) => {
                    for line in result.lines() {
                        let text = line.trim().trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ').trim();
                        if !text.is_empty() {
                            let _ = sqlx::query(
                                "INSERT INTO examples (word_id, text, source) VALUES ($1, $2, 'llm') ON CONFLICT DO NOTHING"
                            )
                            .bind(word_id).bind(text)
                            .execute(pool).await;
                        }
                    }
                    examples_filled += 1;
                }
                Err(e) => { warn!("LLM example error for {}: {}", word, e); }
            }
        }

        // 4. Fill missing example translations
        let untranslated: Vec<(i64, String)> = sqlx::query_as(
            "SELECT id, text FROM examples WHERE word_id = $1 AND translation IS NULL AND source = 'llm'"
        )
        .bind(word_id)
        .fetch_all(pool)
        .await?;

        for (ex_id, ex_text) in &untranslated {
            let prompt = format!("Translate this English sentence to Chinese: '{}'", ex_text);
            match llm.generate(&prompt, LlmTaskType::Generate).await {
                Ok(result) => {
                    let _ = sqlx::query(
                        "UPDATE examples SET translation = $1, trans_source = 'llm', trans_reviewed = false, updated_at = NOW() WHERE id = $2"
                    )
                    .bind(result.trim()).bind(ex_id)
                    .execute(pool).await;
                    ex_trans_filled += 1;
                }
                Err(e) => { warn!("LLM ex trans error: {}", e); }
            }
        }

        if trans_filled % 100 == 0 && trans_filled > 0 {
            info!("Progress: {} trans, {} defs, {} examples, {} ex-trans filled",
                  trans_filled, defs_filled, examples_filled, ex_trans_filled);
        }
    }

    info!("fill_gaps complete: {} trans, {} defs, {} examples, {} ex-trans",
          trans_filled, defs_filled, examples_filled, ex_trans_filled);
    Ok(())
}
