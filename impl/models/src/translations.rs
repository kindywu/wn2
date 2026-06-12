use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Translation {
    pub id: i64,
    pub word_id: i64,
    pub text: String,
    pub language: String,
    pub source: super::enums::DataSource,
    pub reviewed: bool,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub modified: bool,
    pub modified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
