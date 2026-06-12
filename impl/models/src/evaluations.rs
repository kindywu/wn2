use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Evaluation {
    pub id: i64,
    pub target_table: String,
    pub target_id: i64,
    pub word_id: i64,
    pub dimension: String,
    pub score: i16,
    pub comment: Option<String>,
    pub suggestion: Option<String>,
    pub severity: super::enums::SeverityLevel,
    pub reviewed: bool,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
