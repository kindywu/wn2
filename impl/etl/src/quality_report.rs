use sqlx::PgPool;
use tracing::info;

pub async fn run(pool: &PgPool) -> anyhow::Result<()> {
    info!("========== DICT DATA QUALITY REPORT ==========");

    // ── WORDS ──────────────────────────────────────────
    let (total_words,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM words").fetch_one(pool).await?;
    let (words_with_pos,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM words WHERE pos IS NOT NULL").fetch_one(pool).await?;
    let (words_with_phonetic,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM words WHERE phonetic IS NOT NULL").fetch_one(pool).await?;
    let (words_with_audio,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM words WHERE audio IS NOT NULL").fetch_one(pool).await?;
    let (words_curated,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM words WHERE curated").fetch_one(pool).await?;
    let (words_null_pos,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM words WHERE pos IS NULL").fetch_one(pool).await?;

    info!("── WORDS ──");
    info!("  Total:               {total_words:>10}");
    info!("  With POS:            {words_with_pos:>10} ({:.1}%)",
          words_with_pos as f64 / total_words as f64 * 100.0);
    info!("  Without POS (NULL):  {words_null_pos:>10} ({:.1}%)",
          words_null_pos as f64 / total_words as f64 * 100.0);
    info!("  With phonetic:       {words_with_phonetic:>10} ({:.1}%)",
          words_with_phonetic as f64 / total_words as f64 * 100.0);
    info!("  With audio:          {words_with_audio:>10} ({:.1}%)",
          words_with_audio as f64 / total_words as f64 * 100.0);
    info!("  Curated:             {words_curated:>10}");

    // POS distribution
    info!("  POS distribution:");
    let pos_rows: Vec<(Option<String>, i64)> = sqlx::query_as(
        "SELECT pos::text, count(*) FROM words GROUP BY pos ORDER BY count(*) DESC"
    ).fetch_all(pool).await?;
    for (pos, cnt) in &pos_rows {
        info!("    {:>8}: {:>10} ({:.1}%)",
              pos.as_deref().unwrap_or("NULL"), cnt,
              *cnt as f64 / total_words as f64 * 100.0);
    }

    // Source distribution
    info!("  Source distribution:");
    let src_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT unnest(sources) as src, count(*) FROM words GROUP BY src ORDER BY count(*) DESC"
    ).fetch_all(pool).await?;
    for (src, cnt) in &src_rows {
        info!("    {src:>12}: {cnt:>10}");
    }

    // Words with collins/bnc/frq data (stardict metadata)
    let (words_collins,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM words WHERE collins > 0").fetch_one(pool).await?;
    let (words_oxford,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM words WHERE oxford").fetch_one(pool).await?;
    let (words_bnc,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM words WHERE bnc IS NOT NULL").fetch_one(pool).await?;
    let (words_frq,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM words WHERE frq IS NOT NULL").fetch_one(pool).await?;
    let (words_tags,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM words WHERE tags <> '{}'").fetch_one(pool).await?;
    let (words_exchange,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM words WHERE exchange IS NOT NULL").fetch_one(pool).await?;
    info!("  Metadata coverage (stardict fields):");
    info!("    Collins (0-5):      {words_collins:>10}");
    info!("    Oxford:             {words_oxford:>10}");
    info!("    BNC freq:           {words_bnc:>10}");
    info!("    FRQ freq:           {words_frq:>10}");
    info!("    Tags (CET4 etc):    {words_tags:>10}");
    info!("    Exchange (forms):   {words_exchange:>10}");

    // ── DEFINITIONS ────────────────────────────────────
    let (total_defs,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM definitions").fetch_one(pool).await?;
    let (words_with_defs,): (i64,) = sqlx::query_as(
        "SELECT count(DISTINCT word_id) FROM definitions"
    ).fetch_one(pool).await?;
    let (words_no_defs,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM words w WHERE NOT EXISTS (SELECT 1 FROM definitions d WHERE d.word_id = w.id)"
    ).fetch_one(pool).await?;

    info!("── DEFINITIONS ──");
    info!("  Total:               {total_defs:>10}");
    info!("  Words with >=1 def:  {words_with_defs:>10} ({:.1}%)",
          words_with_defs as f64 / total_words as f64 * 100.0);
    info!("  Words with 0 defs:   {words_no_defs:>10} ({:.1}%)",
          words_no_defs as f64 / total_words as f64 * 100.0);
    info!("  Avg defs per word:   {:.2}", total_defs as f64 / total_words as f64);

    info!("  By source:");
    let def_src_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT source::text, count(*) FROM definitions GROUP BY source ORDER BY count(*) DESC"
    ).fetch_all(pool).await?;
    for (src, cnt) in &def_src_rows {
        info!("    {src:>12}: {cnt:>10}");
    }

    let (defs_reviewed,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM definitions WHERE reviewed").fetch_one(pool).await?;
    info!("  Reviewed:            {defs_reviewed:>10}");

    // ── TRANSLATIONS ───────────────────────────────────
    let (total_trans,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM translations").fetch_one(pool).await?;
    let (words_with_trans,): (i64,) = sqlx::query_as(
        "SELECT count(DISTINCT word_id) FROM translations"
    ).fetch_one(pool).await?;
    let (words_no_trans,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM words w WHERE NOT EXISTS (SELECT 1 FROM translations t WHERE t.word_id = w.id)"
    ).fetch_one(pool).await?;

    info!("── TRANSLATIONS ──");
    info!("  Total:               {total_trans:>10}");
    info!("  Words with >=1 trans:{words_with_trans:>10} ({:.1}%)",
          words_with_trans as f64 / total_words as f64 * 100.0);
    info!("  Words with 0 trans:  {words_no_trans:>10} ({:.1}%)",
          words_no_trans as f64 / total_words as f64 * 100.0);
    info!("  Avg trans per word:  {:.2}", total_trans as f64 / total_words as f64);

    info!("  By source:");
    let trans_src_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT source::text, count(*) FROM translations GROUP BY source ORDER BY count(*) DESC"
    ).fetch_all(pool).await?;
    for (src, cnt) in &trans_src_rows {
        info!("    {src:>12}: {cnt:>10}");
    }

    let (trans_reviewed,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM translations WHERE reviewed").fetch_one(pool).await?;
    info!("  Reviewed:            {trans_reviewed:>10}");

    // ── EXAMPLES ───────────────────────────────────────
    let (total_examples,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM examples").fetch_one(pool).await?;
    let (words_with_examples,): (i64,) = sqlx::query_as(
        "SELECT count(DISTINCT word_id) FROM examples"
    ).fetch_one(pool).await?;
    let (words_no_examples,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM words w WHERE NOT EXISTS (SELECT 1 FROM examples e WHERE e.word_id = w.id)"
    ).fetch_one(pool).await?;

    info!("── EXAMPLES ──");
    info!("  Total:               {total_examples:>10}");
    info!("  Words with >=1 ex:   {words_with_examples:>10} ({:.1}%)",
          words_with_examples as f64 / total_words as f64 * 100.0);
    info!("  Words with 0 ex:     {words_no_examples:>10} ({:.1}%)",
          words_no_examples as f64 / total_words as f64 * 100.0);

    info!("  By source:");
    let ex_src_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT source::text, count(*) FROM examples GROUP BY source ORDER BY count(*) DESC"
    ).fetch_all(pool).await?;
    for (src, cnt) in &ex_src_rows {
        info!("    {src:>12}: {cnt:>10}");
    }

    // ── WORD FORMS ─────────────────────────────────────
    let (total_forms,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM word_forms").fetch_one(pool).await?;

    info!("── WORD FORMS ──");
    info!("  Total:               {total_forms:>10}");

    info!("  By type:");
    let form_type_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT form_type::text, count(*) FROM word_forms GROUP BY form_type ORDER BY count(*) DESC"
    ).fetch_all(pool).await?;
    for (ft, cnt) in &form_type_rows {
        info!("    {ft:>22}: {cnt:>10}");
    }

    info!("  By source:");
    let form_src_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT source::text, count(*) FROM word_forms GROUP BY source ORDER BY count(*) DESC"
    ).fetch_all(pool).await?;
    for (src, cnt) in &form_src_rows {
        info!("    {src:>12}: {cnt:>10}");
    }

    // ── WORD RELATIONS ─────────────────────────────────
    let (total_relations,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM word_relations").fetch_one(pool).await?;

    info!("── WORD RELATIONS ──");
    info!("  Total:               {total_relations:>10}");

    let rel_type_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT relation_type, count(*) FROM word_relations GROUP BY relation_type ORDER BY count(*) DESC LIMIT 10"
    ).fetch_all(pool).await?;
    info!("  Top 10 relation types:");
    for (rt, cnt) in &rel_type_rows {
        info!("    {rt:>22}: {cnt:>10}");
    }

    // ── CROSS-TABLE GAPS ───────────────────────────────
    info!("── CROSS-TABLE GAPS ──");

    // Words missing BOTH POS and phonetic
    let (no_pos_no_phonetic,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM words WHERE pos IS NULL AND phonetic IS NULL"
    ).fetch_one(pool).await?;
    info!("  No POS AND no phonetic:        {no_pos_no_phonetic:>10}");

    // Words with POS but no definitions
    let (pos_no_defs,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM words w WHERE w.pos IS NOT NULL AND NOT EXISTS (SELECT 1 FROM definitions d WHERE d.word_id = w.id)"
    ).fetch_one(pool).await?;
    info!("  Has POS but 0 definitions:     {pos_no_defs:>10}");

    // Words with definitions but no translations
    let (defs_no_trans,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM words w WHERE EXISTS (SELECT 1 FROM definitions d WHERE d.word_id = w.id) AND NOT EXISTS (SELECT 1 FROM translations t WHERE t.word_id = w.id)"
    ).fetch_one(pool).await?;
    info!("  Has defs but 0 translations:   {defs_no_trans:>10}");

    // Words with translations but no definitions
    let (trans_no_defs,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM words w WHERE NOT EXISTS (SELECT 1 FROM definitions d WHERE d.word_id = w.id) AND EXISTS (SELECT 1 FROM translations t WHERE t.word_id = w.id)"
    ).fetch_one(pool).await?;
    info!("  Has trans but 0 definitions:   {trans_no_defs:>10}");

    // Words with no definitions AND no translations (completely bare)
    let (bare_words,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM words w WHERE NOT EXISTS (SELECT 1 FROM definitions d WHERE d.word_id = w.id) AND NOT EXISTS (SELECT 1 FROM translations t WHERE t.word_id = w.id)"
    ).fetch_one(pool).await?;
    info!("  No defs AND no translations:   {bare_words:>10} ({:.1}%)",
          bare_words as f64 / total_words as f64 * 100.0);

    // Words with no examples
    info!("  Has 0 examples:                {words_no_examples:>10} ({:.1}%)",
          words_no_examples as f64 / total_words as f64 * 100.0);

    // ── SOURCE OVERLAP ─────────────────────────────────
    info!("── SOURCE OVERLAP ──");
    let (both_sources,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM words WHERE 'stardict' = ANY(sources) AND 'wn' = ANY(sources)"
    ).fetch_one(pool).await?;
    let (stardict_only,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM words WHERE 'stardict' = ANY(sources) AND NOT ('wn' = ANY(sources))"
    ).fetch_one(pool).await?;
    let (wn_only,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM words WHERE 'wn' = ANY(sources) AND NOT ('stardict' = ANY(sources))"
    ).fetch_one(pool).await?;
    info!("  Both stardict + wn:  {both_sources:>10}");
    info!("  Stardict only:       {stardict_only:>10}");
    info!("  WN only:             {wn_only:>10}");

    // ── EVALUATIONS ────────────────────────────────────
    let (total_evals,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM evaluations").fetch_one(pool).await?;
    info!("── EVALUATIONS ──");
    info!("  Total:               {total_evals:>10}");

    if total_evals > 0 {
        let eval_rows: Vec<(String, String, i32, i64)> = sqlx::query_as(
            "SELECT target_table, dimension, score, count(*) FROM evaluations GROUP BY target_table, dimension, score ORDER BY target_table, dimension, score"
        ).fetch_all(pool).await?;
        for (table, dim, score, cnt) in &eval_rows {
            info!("    {table}.{dim} score={score}: {cnt}");
        }
    }

    // ── SUMMARY ────────────────────────────────────────
    info!("========== REPORT COMPLETE ==========");

    Ok(())
}
