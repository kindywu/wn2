use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "data_source", rename_all = "snake_case")]
pub enum DataSource {
    Stardict,
    Wn,
    Llm,
    Human,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "pos_type", rename_all = "snake_case")]
pub enum PosType {
    #[sqlx(rename = "n")]
    N,
    #[sqlx(rename = "v")]
    V,
    #[sqlx(rename = "a")]
    A,
    #[sqlx(rename = "r")]
    R,
    #[sqlx(rename = "s")]
    S,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "form_type", rename_all = "snake_case")]
pub enum FormType {
    Plural,
    Past,
    #[sqlx(rename = "past_participle")]
    PastParticiple,
    #[sqlx(rename = "present_participle")]
    PresentParticiple,
    #[sqlx(rename = "third_person")]
    ThirdPerson,
    Comparative,
    Superlative,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "change_action", rename_all = "snake_case")]
pub enum ChangeAction {
    Create,
    Update,
    Delete,
    Review,
    Unreview,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "change_status", rename_all = "snake_case")]
pub enum ChangeStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "severity_level", rename_all = "snake_case")]
pub enum SeverityLevel {
    Critical,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "import_mode", rename_all = "snake_case")]
pub enum ImportMode {
    Full,
    Incremental,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "import_status", rename_all = "snake_case")]
pub enum ImportStatus {
    Running,
    Completed,
    Failed,
}
