use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ImportLog {
    pub id: i64,
    pub source: String,
    pub source_version: Option<String>,
    pub mode: super::enums::ImportMode,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i32>,
    pub new_words: i32,
    pub updated_words: i32,
    pub new_definitions: i32,
    pub updated_definitions: i32,
    pub new_translations: i32,
    pub updated_translations: i32,
    pub new_examples: i32,
    pub updated_examples: i32,
    pub new_relations: i32,
    pub updated_relations: i32,
    pub new_forms: i32,
    pub updated_forms: i32,
    pub conflicts: i32,
    pub status: super::enums::ImportStatus,
    pub error: Option<String>,
}
