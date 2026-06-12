use dict_models::enums::{DataSource, ImportMode};
use rusqlite::Connection;
use sqlx::PgPool;
use tracing::info;

use crate::common::{self, normalize_pos, parse_exchange, parse_detail, parse_tags, parse_oxford};

struct StardictRow {
    word: String,
    phonetic: Option<String>,
    definition: Option<String>,
    translation: Option<String>,
    pos: Option<String>,
    collins: Option<i32>,
    oxford: Option<i32>,
    tag: Option<String>,
    bnc: Option<i32>,
    frq: Option<i32>,
    exchange: Option<String>,
    detail: Option<String>,
    audio: Option<String>,
}

pub async fn run(
    pool: &PgPool, stardict_path: &str, mode: ImportMode, batch_size: usize,
    limit: Option<usize>,
) -> anyhow::Result<()> {
    info!("Starting stardict import: {} (mode={mode:?})", stardict_path);

    let import_log_id: i64 = sqlx::query_scalar(
        "INSERT INTO import_log (source, mode, status) VALUES ('stardict', $1, 'running') RETURNING id"
    )
    .bind(&mode)
    .fetch_one(pool)
    .await?;

    let result = do_import(pool, stardict_path, batch_size, limit).await;

    match &result {
        Ok((new_w, _, new_d, new_t, new_e, new_f)) => {
            sqlx::query(
                "UPDATE import_log SET status='completed', finished_at=NOW(), new_words=$1, new_definitions=$2, new_translations=$3, new_examples=$4, new_forms=$5 WHERE id=$6"
            )
            .bind(*new_w).bind(*new_d).bind(*new_t).bind(*new_e).bind(*new_f)
            .bind(import_log_id)
            .execute(pool).await?;
            info!("Stardict import completed: {} words, {} defs, {} trans, {} examples, {} forms",
                  new_w, new_d, new_t, new_e, new_f);
        }
        Err(e) => {
            sqlx::query("UPDATE import_log SET status='failed', finished_at=NOW(), error=$1 WHERE id=$2")
                .bind(format!("{e:?}"))
                .bind(import_log_id)
                .execute(pool).await?;
            return Err(anyhow::anyhow!("Stardict import failed: {e}"));
        }
    }

    result.map(|_| ())
}

async fn do_import(
    pool: &PgPool, stardict_path: &str, batch_size: usize, limit: Option<usize>,
) -> anyhow::Result<(i64, i64, i64, i64, i64, i64)> {
    let path = stardict_path.to_string();
    let mut rows: Vec<StardictRow> = tokio::task::spawn_blocking(move || {
        let conn = Connection::open(&path)?;
        let mut stmt = conn.prepare(
            "SELECT word, phonetic, definition, translation, pos, collins, oxford, tag, bnc, frq, exchange, detail, audio FROM stardict"
        )?;
        let rows: Result<Vec<StardictRow>, _> = stmt.query_map([], |row| {
            Ok(StardictRow {
                word: row.get::<_, String>(0)?.trim().to_lowercase(),
                phonetic: row.get::<_, Option<String>>(1)?,
                definition: row.get::<_, Option<String>>(2)?,
                translation: row.get::<_, Option<String>>(3)?,
                pos: row.get::<_, Option<String>>(4)?,
                collins: row.get::<_, Option<i32>>(5)?,
                oxford: row.get::<_, Option<i32>>(6)?,
                tag: row.get::<_, Option<String>>(7)?,
                bnc: row.get::<_, Option<i32>>(8)?,
                frq: row.get::<_, Option<i32>>(9)?,
                exchange: row.get::<_, Option<String>>(10)?,
                detail: row.get::<_, Option<String>>(11)?,
                audio: row.get::<_, Option<String>>(12)?,
            })
        })?.collect();
        rows
    }).await??;

    if let Some(n) = limit {
        rows.truncate(n);
    }
    info!("Loaded {} rows from stardict.db", rows.len());

    // Pre-load words that have POS entries (from WN) for fast sibling lookups
    let words_with_pos: std::collections::HashSet<String> = {
        let w: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT word::text FROM words WHERE pos IS NOT NULL"
        )
        .fetch_all(pool)
        .await?;
        w.into_iter().collect()
    };
    info!("Pre-loaded {} words with POS entries", words_with_pos.len());

    let mut total_rows: i64 = 0;
    let mut total_new_words: i64 = 0;
    let mut total_updated_words: i64 = 0;
    let mut total_new_defs: i64 = 0;
    let mut total_new_trans: i64 = 0;
    let total_new_examples: i64 = 0;
    let mut total_new_forms: i64 = 0;

    for chunk in rows.chunks(batch_size) {
        let mut tx = pool.begin().await?;

        for row in chunk {
            if row.word.is_empty() { continue; }

            let pos = row.pos.as_deref().and_then(normalize_pos);
            let pos_source = pos.as_ref().map(|_| DataSource::Stardict as DataSource);
            let exchange_json = row.exchange.as_deref().and_then(parse_exchange);
            let detail_json = row.detail.as_deref().and_then(parse_detail);
            let tags = parse_tags(row.tag.as_deref().unwrap_or(""));

            let (word_ids, _is_new) = if let Some(ref p) = pos {
                // Entry has POS — upsert directly
                let id: i64 = sqlx::query_scalar(
                    "INSERT INTO words (word, pos, pos_source, phonetic, phonetic_source, audio, collins, oxford, bnc, frq, tags, exchange, detail, sources)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, ARRAY['stardict'])
                     ON CONFLICT ON CONSTRAINT uq_word_pos DO UPDATE
                     SET sources = CASE WHEN NOT ('stardict' = ANY(words.sources)) THEN array_append(words.sources, 'stardict') ELSE words.sources END,
                         pos = COALESCE(words.pos, EXCLUDED.pos),
                         pos_source = COALESCE(words.pos_source, EXCLUDED.pos_source),
                         phonetic = COALESCE(words.phonetic, EXCLUDED.phonetic),
                         phonetic_source = COALESCE(words.phonetic_source, EXCLUDED.phonetic_source),
                         audio = COALESCE(words.audio, EXCLUDED.audio),
                         collins = GREATEST(words.collins, EXCLUDED.collins),
                         oxford = words.oxford OR EXCLUDED.oxford,
                         bnc = COALESCE(words.bnc, EXCLUDED.bnc),
                         frq = COALESCE(words.frq, EXCLUDED.frq),
                         tags = words.tags || EXCLUDED.tags,
                         exchange = COALESCE(words.exchange, EXCLUDED.exchange),
                         detail = COALESCE(words.detail, EXCLUDED.detail)
                     WHERE words.curated = false
                     RETURNING id"
                )
                .bind(&row.word).bind(p).bind(&pos_source)
                .bind(&row.phonetic).bind(Some(DataSource::Stardict as DataSource))
                .bind(&row.audio)
                .bind(row.collins.unwrap_or(0) as i16).bind(parse_oxford(row.oxford.unwrap_or(0)))
                .bind(row.bnc).bind(row.frq)
                .bind(&tags).bind(&exchange_json).bind(&detail_json)
                .fetch_one(&mut *tx).await?;
                (vec![id], false)
            } else {
                // No POS — check pre-built set (fast, no DB query for majority)
                if words_with_pos.contains(&row.word) {
                    // Has POS siblings from WN — merge metadata + defs/translations into them
                    let siblings: Vec<i64> = sqlx::query_scalar(
                        "SELECT id FROM words WHERE word = $1 AND pos IS NOT NULL"
                    )
                    .bind(&row.word)
                    .fetch_all(&mut *tx)
                    .await?;

                    for &sid in &siblings {
                        sqlx::query(
                            "UPDATE words SET sources = array_append(sources, 'stardict'), phonetic = COALESCE(words.phonetic, $2), phonetic_source = COALESCE(words.phonetic_source, $3), collins = GREATEST(words.collins, $4), oxford = words.oxford OR $5, bnc = COALESCE(words.bnc, $6), frq = COALESCE(words.frq, $7), tags = words.tags || $8, exchange = COALESCE(words.exchange, $9), detail = COALESCE(words.detail, $10) WHERE id = $1 AND curated = false"
                        )
                        .bind(sid)
                        .bind(&row.phonetic).bind(Some(DataSource::Stardict as DataSource))
                        .bind(row.collins.unwrap_or(0) as i16).bind(parse_oxford(row.oxford.unwrap_or(0)))
                        .bind(row.bnc).bind(row.frq)
                        .bind(&tags).bind(&exchange_json).bind(&detail_json)
                        .execute(&mut *tx).await?;
                    }
                    total_updated_words += 1;
                    (siblings, false)
                } else {
                    // No POS entry for this word — create NULL-pos row as fallback
                    let maybe_id: Option<i64> = sqlx::query_scalar(
                        "INSERT INTO words (word, pos, phonetic, phonetic_source, audio, collins, oxford, bnc, frq, tags, exchange, detail, sources)
                         VALUES ($1, NULL, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, ARRAY['stardict'])
                         ON CONFLICT ON CONSTRAINT uq_word_pos DO NOTHING
                         RETURNING id"
                    )
                    .bind(&row.word)
                    .bind(&row.phonetic).bind(Some(DataSource::Stardict as DataSource))
                    .bind(&row.audio)
                    .bind(row.collins.unwrap_or(0) as i16).bind(parse_oxford(row.oxford.unwrap_or(0)))
                    .bind(row.bnc).bind(row.frq)
                    .bind(&tags).bind(&exchange_json).bind(&detail_json)
                    .fetch_optional(&mut *tx).await?;

                    if let Some(id) = maybe_id {
                        total_new_words += 1;
                        (vec![id], true)
                    } else {
                        let id: i64 = sqlx::query_scalar(
                            "SELECT id FROM words WHERE word = $1 AND pos IS NULL"
                        )
                        .bind(&row.word)
                        .fetch_one(&mut *tx).await?;
                        (vec![id], false)
                    }
                }
            };

            // Insert definitions, translations, forms into ALL target word_ids
            total_rows += 1;

            for &wid in &word_ids {
                if let Some(ref def) = row.definition {
                    if !def.trim().is_empty() {
                        let def_text = def.trim();
                        let def_hash = format!("{:x}", md5::compute(def_text));
                        let r = sqlx::query(
                            "INSERT INTO definitions (word_id, text, text_hash, source) VALUES ($1, $2, $3, 'stardict') ON CONFLICT (word_id, text_hash, source) DO NOTHING"
                        )
                        .bind(wid).bind(def_text).bind(&def_hash)
                        .execute(&mut *tx).await?;
                        if r.rows_affected() > 0 { total_new_defs += 1; }
                    }
                }

                if let Some(ref tr) = row.translation {
                    if !tr.trim().is_empty() {
                        let tr_text = tr.trim();
                        let tr_hash = format!("{:x}", md5::compute(tr_text));
                        let r = sqlx::query(
                            "INSERT INTO translations (word_id, text, text_hash, language, source) VALUES ($1, $2, $3, 'zh', 'stardict') ON CONFLICT (word_id, text_hash, language, source) DO NOTHING"
                        )
                        .bind(wid).bind(tr_text).bind(&tr_hash)
                        .execute(&mut *tx).await?;
                        if r.rows_affected() > 0 { total_new_trans += 1; }
                    }
                }

                if let Some(ref exchange_str) = row.exchange {
                    if let Some(exchange_map) = parse_exchange(exchange_str) {
                        if let Some(obj) = exchange_map.as_object() {
                            for (key, val) in obj {
                                let form_type = common::EXCHANGE_MAP.iter()
                                    .find(|(k, _)| *k == key.as_str())
                                    .map(|(_, v)| *v);
                                if let Some(ft) = form_type {
                                    if let Some(val_str) = val.as_str() {
                                        for form_text in val_str.split('/') {
                                            let form_text = form_text.trim();
                                            if !form_text.is_empty() {
                                                let r = sqlx::query(
                                                    "INSERT INTO word_forms (word_id, form, form_type, source) VALUES ($1, $2, $3::form_type, 'stardict') ON CONFLICT (word_id, form, form_type) DO NOTHING"
                                                )
                                                .bind(wid).bind(form_text).bind(ft)
                                                .execute(&mut *tx).await?;
                                                if r.rows_affected() > 0 { total_new_forms += 1; }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        tx.commit().await?;
        info!("Progress: {total_rows}/{} rows", rows.len());
    }

    Ok((total_new_words, total_updated_words, total_new_defs, total_new_trans, total_new_examples, total_new_forms))
}
