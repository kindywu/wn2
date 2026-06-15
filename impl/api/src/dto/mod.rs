use serde::{Deserialize, Serialize};

// --- Words ---

#[derive(Debug, Deserialize)]
pub struct WordSearchQuery {
    pub q: String,
    #[allow(dead_code)]
    pub pos: Option<String>,
    pub mode: Option<String>,  // "exact" (default) or "prefix"
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct WordOut {
    pub id: i64,
    pub word: String,
    pub pos: Option<String>,
    pub phonetic: Option<String>,
    pub collins: i16,
    pub oxford: bool,
    pub bnc: Option<i32>,
    pub frq: Option<i32>,
    pub tags: Vec<String>,
    pub curated: bool,
    pub curated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// --- Definitions ---

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DefinitionOut {
    pub id: i64,
    pub word_id: i64,
    pub text: String,
    pub source: String,
    pub reviewed: bool,
    pub modified: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct DefinitionCreate {
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct DefinitionUpdate {
    pub text: String,
}

// --- Translations ---

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TranslationOut {
    pub id: i64,
    pub word_id: i64,
    pub text: String,
    pub language: String,
    pub source: String,
    pub reviewed: bool,
    pub modified: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct TranslationCreate {
    pub text: String,
    pub language: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TranslationUpdate {
    pub text: String,
}

// --- Examples ---

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ExampleOut {
    pub id: i64,
    pub word_id: i64,
    pub text: String,
    pub translation: Option<String>,
    pub source: String,
    pub reviewed: bool,
    pub modified: bool,
    pub trans_source: Option<String>,
    pub trans_reviewed: bool,
    pub trans_modified: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ExampleCreate {
    pub text: String,
    pub translation: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExampleUpdate {
    pub text: Option<String>,
    pub translation: Option<String>,
}

// --- Word Relations ---

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct WordRelationOut {
    pub id: i64,
    pub word_id: i64,
    pub related_word: String,
    pub relation_type: String,
    pub source: String,
    pub reviewed: bool,
    pub modified: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct WordRelationCreate {
    pub related_word: String,
    pub relation_type: String,
}

#[derive(Debug, Deserialize)]
pub struct WordRelationUpdate {
    pub related_word: Option<String>,
    pub relation_type: Option<String>,
}

// --- Word Forms ---

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct WordFormOut {
    pub id: i64,
    pub word_id: i64,
    pub form: String,
    pub form_type: String,
    pub source: String,
    pub reviewed: bool,
    pub modified: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// --- Pending Changes ---

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PendingChangeOut {
    pub id: i64,
    pub table_name: String,
    pub row_id: Option<i64>,
    pub word_id: i64,
    pub field: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub source: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct PendingChangeQuery {
    pub status: Option<String>,
    #[allow(dead_code)]
    pub source: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PendingChangeApprove {
    pub value: Option<String>,
}

// --- Evaluations ---

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct EvaluationOut {
    pub id: i64,
    pub target_table: String,
    pub target_id: i64,
    pub word_id: i64,
    pub dimension: String,
    pub score: i16,
    pub comment: Option<String>,
    pub suggestion: Option<String>,
    pub severity: String,
    pub reviewed: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct EvaluationQuery {
    pub reviewed: Option<bool>,
    pub severity: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// --- Logs ---

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ImportLogOut {
    pub id: i64,
    pub source: String,
    pub source_version: Option<String>,
    pub mode: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    pub new_words: i32,
    pub updated_words: i32,
    pub conflicts: i32,
    pub status: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ChangeLogOut {
    pub id: i64,
    pub table_name: String,
    pub row_id: i64,
    pub field: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub action: String,
    pub operator: String,
    pub changed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ChangeLogQuery {
    pub operator: Option<String>,
    #[allow(dead_code)]
    pub from: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
