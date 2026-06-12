use serde::Deserialize;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::llm_client::{LlmClient, LlmTaskType};

#[derive(Deserialize)]
struct ExampleResult {
    idx: usize,
    examples: Vec<String>,
}

struct WordEntry {
    id: i64,
    word: String,
    pos: String,
    definition: String,
    translation: String,
}

const BATCH_SIZE: usize = 50;
const FALLBACK_SIZE: usize = 10;

pub async fn run(
    pool: &PgPool,
    llm: &(impl LlmClient + ?Sized),
    limit: Option<usize>,
) -> anyhow::Result<()> {
    info!("Generating examples for tagged words...");

    let rows: Vec<WordEntry> = sqlx::query_as!(
        WordEntry,
        r#"SELECT w.id, w.word::text as "word!", w.pos::text as "pos!",
           (SELECT d.text FROM definitions d WHERE d.word_id = w.id ORDER BY d.id LIMIT 1) as "definition!",
           (SELECT t.text FROM translations t WHERE t.word_id = w.id ORDER BY t.id LIMIT 1) as "translation!"
         FROM words w
         WHERE w.tags <> '{}'
           AND w.pos IS NOT NULL
           AND NOT EXISTS (SELECT 1 FROM examples e WHERE e.word_id = w.id)
         ORDER BY random()
         LIMIT $1"#,
        limit.unwrap_or(6725) as i64
    )
    .fetch_all(pool)
    .await?;

    info!("Loaded {} words needing examples", rows.len());

    let mut total_inserted = 0u64;
    let mut processed = 0usize;

    let mut i = 0;
    while i < rows.len() {
        let end = (i + BATCH_SIZE).min(rows.len());
        let chunk = &rows[i..end];

        match try_batch(pool, llm, chunk).await {
            Ok(n) => {
                total_inserted += n;
                processed += chunk.len();
                i = end;
                info!(
                    "Progress: {processed}/{} words, {total_inserted} examples inserted (batch {})",
                    rows.len(),
                    i / BATCH_SIZE
                );
            }
            Err(e) => {
                warn!(
                    "Batch {}-{} failed ({} words): {}. Retrying in sub-batches...",
                    i,
                    end,
                    chunk.len(),
                    e
                );
                // Fallback: process in smaller sub-batches
                for sub in chunk.chunks(FALLBACK_SIZE) {
                    match try_batch(pool, llm, sub).await {
                        Ok(n) => {
                            total_inserted += n;
                            processed += sub.len();
                            info!("  Sub-batch OK: {processed}/{} words", rows.len());
                        }
                        Err(e2) => {
                            warn!("  Sub-batch also failed ({} words): {}", sub.len(), e2);
                            processed += sub.len();
                        }
                    }
                }
                i = end;
            }
        }
    }

    info!("Done. Total examples inserted: {total_inserted}");
    Ok(())
}

async fn try_batch(
    pool: &PgPool,
    llm: &(impl LlmClient + ?Sized),
    words: &[WordEntry],
) -> anyhow::Result<u64> {
    let prompt = build_batch_prompt(words);
    let results: Vec<ExampleResult> =
        crate::llm_client::generate_json(llm, &prompt, LlmTaskType::Generate).await?;

    let mut inserted = 0u64;
    for r in &results {
        if r.idx >= words.len() {
            continue;
        }
        let word_id = words[r.idx].id;
        for ex in &r.examples {
            let ex = ex.trim();
            if ex.is_empty() {
                continue;
            }
            let ex_hash = format!("{:x}", md5::compute(ex));
            match sqlx::query(
                "INSERT INTO examples (word_id, text, text_hash, source) VALUES ($1, $2, $3, 'llm') ON CONFLICT (word_id, text_hash) DO NOTHING"
            )
            .bind(word_id).bind(ex).bind(&ex_hash)
            .execute(pool).await
            {
                Ok(r) => { inserted += r.rows_affected(); }
                Err(e) => { warn!("Insert error: {e}"); }
            }
        }
    }
    Ok(inserted)
}

fn build_batch_prompt(words: &[WordEntry]) -> String {
    let mut body = String::from(
        "Generate 2-3 natural English example sentences for each of these words.\n\n\
         Return ONLY a JSON array:\n\
         [{\"idx\":0, \"examples\":[\"sentence1\",\"sentence2\"]}, ...]\n\n\
         Words:\n",
    );

    for (i, w) in words.iter().enumerate() {
        body.push_str(&format!(
            "{}. {} ({}) = \"{}\" | {}\n",
            i,
            w.word,
            w.pos,
            truncate(&w.definition, 120),
            truncate(&w.translation, 60),
        ));
    }

    body
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let end = s.char_indices().nth(max).map(|(i, _)| i).unwrap_or(s.len());
        format!("{}...", &s[..end])
    }
}
