use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ChangeLog {
    pub id: i64,
    pub table_name: String,
    pub row_id: i64,
    pub field: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub action: super::enums::ChangeAction,
    pub operator: String,
    pub changed_at: DateTime<Utc>,
}
