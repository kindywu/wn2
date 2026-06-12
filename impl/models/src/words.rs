use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::enums::PosType;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Word {
    pub id: i64,
    pub word: String,
    pub pos: Option<PosType>,
    pub pos_source: Option<super::enums::DataSource>,
    pub phonetic: Option<String>,
    pub phonetic_source: Option<super::enums::DataSource>,
    pub audio: Option<String>,
    pub collins: i16,
    pub oxford: bool,
    pub bnc: Option<i32>,
    pub frq: Option<i32>,
    pub tags: Vec<String>,
    pub exchange: Option<serde_json::Value>,
    pub detail: Option<serde_json::Value>,
    pub sources: Vec<String>,
    pub curated: bool,
    pub curated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
