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

pub async fn run(
    pool: &PgPool, llm: &(impl LlmClient + ?Sized), limit: Option<usize>,
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

    let batch_size = 10;
    let mut total_inserted = 0u64;

    for chunk in rows.chunks(batch_size) {
        let prompt = build_batch_prompt(chunk);

        let result: Result<Vec<ExampleResult>, _> =
            crate::llm_client::generate_json(llm, &prompt, LlmTaskType::Generate).await;

        match result {
            Ok(results) => {
                for r in &results {
                    if r.idx < chunk.len() {
                        let word_id = chunk[r.idx].id;
                        for ex in &r.examples {
                            let ex = ex.trim();
                            if ex.is_empty() { continue; }
                            let ex_hash = format!("{:x}", md5::compute(ex));
                            match sqlx::query(
                                "INSERT INTO examples (word_id, text, text_hash, source) VALUES ($1, $2, $3, 'llm') ON CONFLICT (word_id, text_hash) DO NOTHING"
                            )
                            .bind(word_id).bind(ex).bind(&ex_hash)
                            .execute(pool).await
                            {
                                Ok(r) => { total_inserted += r.rows_affected(); }
                                Err(e) => { warn!("Insert error: {e}"); }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!("LLM batch error ({} words): {}", chunk.len(), e);
            }
        }

        info!(
            "Progress: {}/{} words, {} examples inserted",
            (chunk.as_ptr() as usize - rows.as_ptr() as usize) / std::mem::size_of::<WordEntry>() + chunk.len(),
            rows.len(),
            total_inserted
        );
    }

    info!("Done. Total examples inserted: {total_inserted}");
    Ok(())
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
            i, w.word, w.pos,
            truncate(&w.definition, 150),
            truncate(&w.translation, 80),
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
