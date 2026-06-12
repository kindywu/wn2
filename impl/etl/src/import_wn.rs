use std::collections::HashMap;
use dict_models::enums::{DataSource, ImportMode, PosType};
use rusqlite::Connection;
use sqlx::PgPool;
use tracing::info;

pub async fn run(pool: &PgPool, wn_path: &str, mode: ImportMode, batch_size: usize, limit: Option<usize>) -> anyhow::Result<()> {
    info!("Starting wn import: {} (mode={mode:?})", wn_path);

    let import_log_id: i64 = sqlx::query_scalar(
        "INSERT INTO import_log (source, mode, status) VALUES ('wn', $1, 'running') RETURNING id"
    )
    .bind(&mode)
    .fetch_one(pool)
    .await?;

    let result = do_import(pool, wn_path, batch_size, limit).await;

    match &result {
        Ok((nw, uw, nd, nt, ne, nr, nf)) => {
            sqlx::query(
                "UPDATE import_log SET status='completed', finished_at=NOW(), new_words=$1, updated_words=$2, new_definitions=$3, new_translations=$4, new_examples=$5, new_relations=$6, new_forms=$7 WHERE id=$8"
            )
            .bind(*nw).bind(*uw).bind(*nd).bind(*nt).bind(*ne).bind(*nr).bind(*nf)
            .bind(import_log_id)
            .execute(pool).await?;
            info!("WN import completed: {} new words, {} updated, {} defs, {} trans, {} examples, {} relations, {} forms",
                  nw, uw, nd, nt, ne, nr, nf);
        }
        Err(e) => {
            sqlx::query("UPDATE import_log SET status='failed', finished_at=NOW(), error=$1 WHERE id=$2")
                .bind(format!("{e:?}"))
                .bind(import_log_id)
                .execute(pool).await?;
            return Err(anyhow::anyhow!("WN import failed: {e}"));
        }
    }

    result.map(|_| ())
}

async fn do_import(pool: &PgPool, wn_path: &str, batch_size: usize, limit: Option<usize>) -> anyhow::Result<(i64, i64, i64, i64, i64, i64, i64)> {
    let path = wn_path.to_string();

    // Load data from wn.db in a blocking thread
    let mut wn_data: WnData = tokio::task::spawn_blocking(move || {
        load_wn_data(&path)
    }).await??;

    info!("Loaded wn data: {} entries, {} synsets, {} defs, {} examples",
          wn_data.entries.len(), wn_data.synsets.len(), wn_data.definitions.len(),
          wn_data.examples.len());

    if let Some(n) = limit {
        info!("Limiting to {n} WN entries");
        wn_data.english_entries.truncate(n);
    }

    let mut new_words: i64 = 0;
    let mut updated_words: i64 = 0;
    let mut new_defs: i64 = 0;
    let mut new_trans: i64 = 0;
    let mut new_examples: i64 = 0;
    let mut new_relations: i64 = 0;
    let mut new_forms: i64 = 0;

    // Phase 1: Import English lemmas → words
    info!("Phase 1: Importing English lemmas");
    for chunk in wn_data.english_entries.chunks(batch_size) {
        let mut tx = pool.begin().await?;
        for entry in chunk {
            if entry.pos != "n" && entry.pos != "v" && entry.pos != "a" && entry.pos != "r" && entry.pos != "s" {
                continue;
            }
            let pos: PosType = match entry.pos.as_str() {
                "n" => PosType::N, "v" => PosType::V, "a" => PosType::A,
                "r" => PosType::R, "s" => PosType::S, _ => continue,
            };
            let lemma = entry.lemma.to_lowercase();

            let pronounce = wn_data.pronunciations.get(&entry.entry_rowid);

            let result = sqlx::query_scalar::<_, i64>(
                "INSERT INTO words (word, pos, pos_source, phonetic, phonetic_source, sources)
                 VALUES ($1, $2, 'wn', $3, $4, ARRAY['wn'])
                 ON CONFLICT ON CONSTRAINT uq_word_pos DO UPDATE
                 SET sources = CASE WHEN NOT ('wn' = ANY(words.sources)) THEN array_append(words.sources, 'wn') ELSE words.sources END,
                     pos = COALESCE(words.pos, EXCLUDED.pos),
                     pos_source = COALESCE(words.pos_source, 'wn'),
                     phonetic = COALESCE(words.phonetic, EXCLUDED.phonetic),
                     phonetic_source = COALESCE(words.phonetic_source, EXCLUDED.phonetic_source)
                 WHERE words.curated = false
                 RETURNING id"
            )
            .bind(lemma)
            .bind(&pos)
            .bind(&pronounce)
            .bind(pronounce.as_ref().map(|_| DataSource::Wn as DataSource))
            .fetch_optional(&mut *tx)
            .await?;

            match result {
                Some(_id) => { new_words += 1; }
                None => { updated_words += 1; }
            }
        }
        tx.commit().await?;
    }

    // Build (word, pos) → word_id lookup map for fast resolve
    info!("Building word_id lookup map");
    let word_id_map: HashMap<(String, PosType), i64> = {
        let rows: Vec<(String, PosType, i64)> = sqlx::query_as(
            "SELECT word::text, pos, id FROM words WHERE 'wn' = ANY(sources)"
        )
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|(w, p, id)| ((w, p), id)).collect()
    };
    info!("Built word_id map with {} entries", word_id_map.len());

    // Phase 2: Import definitions and examples from English synsets
    info!("Phase 2: Importing definitions and examples");
    for chunk in wn_data.english_synsets.chunks(batch_size) {
        let mut tx = pool.begin().await?;
        for synset_id in chunk {
            let mut word_ids = Vec::new();
            if let Some(entry_ids) = wn_data.synset_entries.get(synset_id) {
                for eid in entry_ids {
                    if let Some(entry) = wn_data.entries.get(eid) {
                        let pos: PosType = match entry.pos.as_str() {
                            "n" => PosType::N, "v" => PosType::V, "a" => PosType::A,
                            "r" => PosType::R, "s" => PosType::S, _ => continue,
                        };
                        if let Some(&wid) = word_id_map.get(&(entry.lemma.to_lowercase(), pos)) {
                            word_ids.push(wid);
                        }
                    }
                }
            }

            // Insert definitions
            if let Some(defs) = wn_data.definitions.get(synset_id) {
                for def_text in defs {
                    let def_hash = format!("{:x}", md5::compute(def_text.as_bytes()));
                    for &word_id in &word_ids {
                        let r = sqlx::query(
                            "INSERT INTO definitions (word_id, text, text_hash, source) VALUES ($1, $2, $3, 'wn') ON CONFLICT (word_id, text_hash, source) DO NOTHING"
                        )
                        .bind(word_id).bind(def_text).bind(&def_hash)
                        .execute(&mut *tx).await?;
                        if r.rows_affected() > 0 { new_defs += 1; }
                    }
                }
            }

            // Insert examples
            if let Some(examples) = wn_data.examples.get(synset_id) {
                for ex_text in examples {
                    let ex_hash = format!("{:x}", md5::compute(ex_text.as_bytes()));
                    for &word_id in &word_ids {
                        let r = sqlx::query(
                            "INSERT INTO examples (word_id, text, text_hash, source) VALUES ($1, $2, $3, 'wn') ON CONFLICT (word_id, text_hash) DO NOTHING"
                        )
                        .bind(word_id).bind(ex_text).bind(&ex_hash)
                        .execute(&mut *tx).await?;
                        if r.rows_affected() > 0 { new_examples += 1; }
                    }
                }
            }
        }
        tx.commit().await?;
    }

    // Phase 3: Chinese translations via ILI mapping
    info!("Phase 3: Importing Chinese translations via ILI");
    let mut ili_to_en_entries: HashMap<i64, Vec<i64>> = HashMap::new();
    for synset_id in &wn_data.english_synsets {
        if let Some(ili_id) = wn_data.synset_ilis.get(synset_id) {
            if let Some(entry_ids) = wn_data.synset_entries.get(synset_id) {
                for entry_rowid in entry_ids {
                    if let Some(en_entry) = wn_data.entries.get(entry_rowid) {
                        let pos: PosType = match en_entry.pos.as_str() {
                            "n" => PosType::N, "v" => PosType::V, "a" => PosType::A,
                            "r" => PosType::R, "s" => PosType::S, _ => continue,
                        };
                        if let Some(&wid) = word_id_map.get(&(en_entry.lemma.to_lowercase(), pos)) {
                            ili_to_en_entries.entry(*ili_id).or_default().push(wid);
                        }
                    }
                }
            }
        }
    }

    // Find Chinese lemmas via ILI
    {
        let mut tx = pool.begin().await?;
        for cmn_entry in &wn_data.cmn_entries {
            if let Some(ili_id) = wn_data.cmn_synset_ilis.get(&cmn_entry.synset_rowid) {
                if let Some(en_word_ids) = ili_to_en_entries.get(ili_id) {
                    for &en_word_id in en_word_ids {
                        let tr_hash = format!("{:x}", md5::compute(cmn_entry.lemma.as_bytes()));
                        let r = sqlx::query(
                            "INSERT INTO translations (word_id, text, text_hash, language, source) VALUES ($1, $2, $3, 'zh', 'wn') ON CONFLICT (word_id, text_hash, language, source) DO NOTHING"
                        )
                        .bind(en_word_id).bind(&cmn_entry.lemma).bind(&tr_hash)
                        .execute(&mut *tx).await?;
                        if r.rows_affected() > 0 { new_trans += 1; }
                    }
                }
            }
        }
        tx.commit().await?;
    }

    // Phase 4: Semantic relations
    info!("Phase 4: Importing semantic relations");
    {
        let mut tx = pool.begin().await?;
        for rel in &wn_data.relations {
            let mut src_word_ids = Vec::new();
            let mut tgt_word_ids = Vec::new();

            if let Some(src_entries) = wn_data.synset_entries.get(&rel.source_synset) {
                for eid in src_entries {
                    if let Some(entry) = wn_data.entries.get(eid) {
                        let pos: PosType = match entry.pos.as_str() {
                            "n" => PosType::N, "v" => PosType::V, "a" => PosType::A,
                            "r" => PosType::R, "s" => PosType::S, _ => continue,
                        };
                        if let Some(&wid) = word_id_map.get(&(entry.lemma.to_lowercase(), pos)) {
                            src_word_ids.push((wid, entry));
                        }
                    }
                }
            }

            if let Some(tgt_entries) = wn_data.synset_entries.get(&rel.target_synset) {
                for eid in tgt_entries {
                    if let Some(entry) = wn_data.entries.get(eid) {
                        let pos: PosType = match entry.pos.as_str() {
                            "n" => PosType::N, "v" => PosType::V, "a" => PosType::A,
                            "r" => PosType::R, "s" => PosType::S, _ => continue,
                        };
                        if let Some(&wid) = word_id_map.get(&(entry.lemma.to_lowercase(), pos)) {
                            tgt_word_ids.push((wid, entry));
                        }
                    }
                }
            }

            for &(swid, _) in &src_word_ids {
                for &(twid, ref tgt_entry) in &tgt_word_ids {
                    let r = sqlx::query(
                        "INSERT INTO word_relations (word_id, related_word, relation_type, source) VALUES ($1, $2, $3, 'wn') ON CONFLICT (word_id, related_word, relation_type) DO NOTHING"
                    )
                    .bind(swid).bind(&tgt_entry.lemma).bind(&rel.rel_type)
                    .execute(&mut *tx).await?;
                    if r.rows_affected() > 0 { new_relations += 1; }

                    // Bidirectional for symmetric relations
                    if rel.rel_type == "synonym" || rel.rel_type == "antonym" || rel.rel_type == "similar" {
                        for &(_swid2, ref src_entry2) in &src_word_ids {
                            let _ = sqlx::query(
                                "INSERT INTO word_relations (word_id, related_word, relation_type, source) VALUES ($1, $2, $3, 'wn') ON CONFLICT (word_id, related_word, relation_type) DO NOTHING"
                            )
                            .bind(twid).bind(&src_entry2.lemma).bind(&rel.rel_type)
                            .execute(&mut *tx).await?;
                        }
                    }
                }
            }
        }
        tx.commit().await?;
    }

    // Phase 5: Word forms from rank > 0
    info!("Phase 5: Importing word forms");
    {
        let mut tx = pool.begin().await?;
        for (entry_rowid, form_text, rank) in &wn_data.forms {
            if *rank == 0 { continue; }
            if let Some(entry) = wn_data.entries.get(entry_rowid) {
                let pos: PosType = match entry.pos.as_str() {
                    "n" => PosType::N, "v" => PosType::V, "a" => PosType::A,
                    "r" => PosType::R, "s" => PosType::S, _ => continue,
                };
                if let Some(&wid) = word_id_map.get(&(entry.lemma.to_lowercase(), pos)) {
                    if let Some(ft) = infer_form_type(form_text, &entry.pos) {
                        let r = sqlx::query(
                            "INSERT INTO word_forms (word_id, form, form_type, source) VALUES ($1, $2, $3::form_type, 'wn') ON CONFLICT (word_id, form, form_type) DO NOTHING"
                        )
                        .bind(wid).bind(form_text).bind(ft)
                        .execute(&mut *tx).await?;
                        if r.rows_affected() > 0 { new_forms += 1; }
                    }
                }
            }
        }
        tx.commit().await?;
    }

    Ok((new_words, updated_words, new_defs, new_trans, new_examples, new_relations, new_forms))
}

// Remove old resolve_word_ids — no longer needed
// (function removed below)

// --- Data types for loading wn.db ---

#[derive(Debug, Clone)]
struct WnEntry {
    entry_rowid: i64,
    lemma: String,
    pos: String,
}

#[derive(Debug, Clone)]
struct CmnEntry {
    lemma: String,
    synset_rowid: i64,
}

#[derive(Debug, Clone)]
struct SynsetRelation {
    source_synset: i64,
    target_synset: i64,
    rel_type: String,
}

struct WnData {
    entries: HashMap<i64, WnEntry>,
    english_entries: Vec<WnEntry>,
    cmn_entries: Vec<CmnEntry>,
    synsets: Vec<i64>,
    english_synsets: Vec<i64>,
    definitions: HashMap<i64, Vec<String>>,
    examples: HashMap<i64, Vec<String>>,
    synset_entries: HashMap<i64, Vec<i64>>,  // synset_rowid → entry_rowids
    synset_ilis: HashMap<i64, i64>,
    cmn_synset_ilis: HashMap<i64, i64>,
    relations: Vec<SynsetRelation>,
    pronunciations: HashMap<i64, String>,
    forms: Vec<(i64, String, i64)>,  // (entry_rowid, form_text, rank)
}

fn load_wn_data(path: &str) -> anyhow::Result<WnData> {
    let conn = Connection::open(path)?;

    // Load relation_types
    let mut rel_types: HashMap<i64, String> = HashMap::new();
    {
        let mut stmt = conn.prepare("SELECT rowid, type FROM relation_types")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (id, t) = row?;
            rel_types.insert(id, t);
        }
    }

    // Load lexicons: en=1, cmn=2
    let mut en_lexicon_rowid = 0i64;
    let mut cmn_lexicon_rowid = 0i64;
    {
        let mut stmt = conn.prepare("SELECT rowid, language FROM lexicons")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (id, lang) = row?;
            if lang == "en" { en_lexicon_rowid = id; }
            else if lang.starts_with("cmn") { cmn_lexicon_rowid = id; }
        }
    }

    // Load entries (English only for words import)
    let mut entries: HashMap<i64, WnEntry> = HashMap::new();
    let mut english_entries: Vec<WnEntry> = Vec::new();
    let mut entry_lemma: HashMap<i64, String> = HashMap::new();
    let mut entry_pos: HashMap<i64, String> = HashMap::new();
    {
        let mut stmt = conn.prepare("SELECT rowid, pos, lexicon_rowid FROM entries")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
        })?;
        for row in rows {
            let (rowid, pos, lr) = row?;
            entry_pos.insert(rowid, pos.clone());
            if lr == en_lexicon_rowid {
                english_entries.push(WnEntry { entry_rowid: rowid, lemma: String::new(), pos: pos.clone() });
            }
        }
    }

    // Load forms (rank=0 for lemma)
    {
        let mut stmt = conn.prepare("SELECT rowid, entry_rowid, form, rank, lexicon_rowid FROM forms")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?, row.get::<_, i64>(3)?, row.get::<_, i64>(4)?))
        })?;
        let mut forms: Vec<(i64, String, i64)> = Vec::new();
        for row in rows {
            let (_rowid, entry_rowid, form, rank, lr) = row?;
            if lr == en_lexicon_rowid && rank == 0 {
                entry_lemma.insert(entry_rowid, form.clone());
            }
            if lr == en_lexicon_rowid {
                forms.push((entry_rowid, form, rank));
            }
        }
        // Update english_entries with lemmas
        for e in &mut english_entries {
            if let Some(lemma) = entry_lemma.get(&e.entry_rowid) {
                e.lemma = lemma.clone();
            }
        }
        // Build entries map
        for e in &english_entries {
            entries.insert(e.entry_rowid, e.clone());
        }
    }

    // Load pronunciations
    let mut pronunciations: HashMap<i64, String> = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT f.entry_rowid, p.value FROM pronunciations p JOIN forms f ON f.rowid = p.form_rowid WHERE p.variety = 'GB' OR p.variety = 'US' ORDER BY CASE p.variety WHEN 'GB' THEN 0 ELSE 1 END"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (entry_rowid, value) = row?;
            pronunciations.entry(entry_rowid).or_insert(value);
        }
    }

    // Load Chinese entries (omw-cmn)
    let mut cmn_entries: Vec<CmnEntry> = Vec::new();
    let mut cmn_forms: Vec<(i64, String)> = Vec::new();  // (entry_rowid, form)
    {
        let mut stmt = conn.prepare("SELECT rowid, entry_rowid, form, rank FROM forms WHERE lexicon_rowid = ? AND rank = 0")?;
        let rows = stmt.query_map([cmn_lexicon_rowid], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?, row.get::<_, i64>(3)?))
        })?;
        for row in rows {
            let (_rowid, entry_rowid, form, _rank) = row?;
            cmn_forms.push((entry_rowid, form));
        }
    }

    // Load senses
    let mut synset_entries: HashMap<i64, Vec<i64>> = HashMap::new();
    let mut entry_synset: HashMap<i64, i64> = HashMap::new();
    {
        let mut stmt = conn.prepare("SELECT rowid, entry_rowid, synset_rowid, lexicon_rowid FROM senses")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?))
        })?;
        for row in rows {
            let (_rowid, entry_rowid, synset_rowid, lr) = row?;
            synset_entries.entry(synset_rowid).or_default().push(entry_rowid);
            if lr == en_lexicon_rowid {
                entry_synset.insert(entry_rowid, synset_rowid);
            }
            if lr == cmn_lexicon_rowid {
                // Find lemma from cmn_forms
                if let Some((_, lemma)) = cmn_forms.iter().find(|(eid, _)| *eid == entry_rowid) {
                    cmn_entries.push(CmnEntry { lemma: lemma.clone(), synset_rowid });
                }
            }
        }
    }

    // Load synsets → ILI mapping
    let mut synset_ilis: HashMap<i64, i64> = HashMap::new();
    let mut cmn_synset_ilis: HashMap<i64, i64> = HashMap::new();
    let mut english_synsets: Vec<i64> = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT rowid, ili_rowid, lexicon_rowid FROM synsets")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?, row.get::<_, i64>(2)?))
        })?;
        for row in rows {
            let (rowid, ili_rowid, lr) = row?;
            if lr == en_lexicon_rowid {
                if let Some(ili) = ili_rowid { synset_ilis.insert(rowid, ili); }
                english_synsets.push(rowid);
            }
            if lr == cmn_lexicon_rowid {
                if let Some(ili) = ili_rowid { cmn_synset_ilis.insert(rowid, ili); }
            }
        }
    }

    // Load definitions (only English synsets)
    let mut definitions: HashMap<i64, Vec<String>> = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT synset_rowid, definition FROM definitions WHERE lexicon_rowid = ?"
        )?;
        let rows = stmt.query_map([en_lexicon_rowid], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (synset_id, def) = row?;
            definitions.entry(synset_id).or_default().push(def);
        }
    }

    // Load synset_examples (only English)
    let mut examples: HashMap<i64, Vec<String>> = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT synset_rowid, example FROM synset_examples WHERE lexicon_rowid = ?"
        )?;
        let rows = stmt.query_map([en_lexicon_rowid], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (synset_id, ex) = row?;
            examples.entry(synset_id).or_default().push(ex);
        }
    }

    // Load synset_relations
    let mut relations: Vec<SynsetRelation> = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT source_rowid, target_rowid, type_rowid FROM synset_relations WHERE lexicon_rowid = ?"
        )?;
        let rows = stmt.query_map([en_lexicon_rowid], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))
        })?;
        for row in rows {
            let (src, tgt, type_id) = row?;
            if let Some(rel_type) = rel_types.get(&type_id) {
                relations.push(SynsetRelation {
                    source_synset: src,
                    target_synset: tgt,
                    rel_type: rel_type.clone(),
                });
            }
        }
    }

    // Load forms (all ranks)
    let mut all_forms: Vec<(i64, String, i64)> = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT entry_rowid, form, rank FROM forms WHERE lexicon_rowid = ?"
        )?;
        let rows = stmt.query_map([en_lexicon_rowid], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
        })?;
        for row in rows {
            let (entry_rowid, form, rank) = row?;
            all_forms.push((entry_rowid, form, rank));
        }
    }

    let synsets: Vec<i64> = english_synsets.clone();

    Ok(WnData {
        entries,
        english_entries,
        cmn_entries,
        synsets,
        english_synsets,
        definitions,
        examples,
        synset_entries,
        synset_ilis,
        cmn_synset_ilis,
        relations,
        pronunciations,
        forms: all_forms,
    })
}

fn infer_form_type(form: &str, pos: &str) -> Option<String> {
    match pos {
        "n" => {
            if form.ends_with('s') || form.ends_with("es") || form.ends_with("ies") {
                Some("plural".into())
            } else {
                None  // Could be a variant spelling
            }
        }
        "v" => {
            if form.ends_with("ing") {
                Some("present_participle".into())
            } else if form.ends_with("ed") {
                Some("past".into())
            } else if form.ends_with("en") || form.ends_with("ed") {
                Some("past_participle".into())
            } else if form.ends_with('s') {
                Some("third_person".into())
            } else {
                None
            }
        }
        "a" | "r" | "s" => {
            if form.ends_with("er") {
                Some("comparative".into())
            } else if form.ends_with("est") {
                Some("superlative".into())
            } else {
                None
            }
        }
        _ => None,
    }
}
