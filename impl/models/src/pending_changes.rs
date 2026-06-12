use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PendingChange {
    pub id: i64,
    pub table_name: String,
    pub row_id: Option<i64>,
    pub word_id: i64,
    pub field: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub source: String,
    pub import_log_id: Option<i64>,
    pub status: super::enums::ChangeStatus,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
