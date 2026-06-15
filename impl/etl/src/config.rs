use config::{Config, Environment};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct EtlConfig {
    pub database_url: String,
    #[serde(default = "default_batch_size")]
    pub etl_batch_size: usize,
    #[serde(default = "default_stardict_path")]
    pub etl_sqlite_stardict_path: String,
    #[serde(default = "default_wn_path")]
    pub etl_sqlite_wn_path: String,
    #[serde(default = "default_llm_provider")]
    #[allow(dead_code)]
    pub llm_provider: String,
    #[serde(default = "default_llm_base_url")]
    pub llm_base_url: String,
    #[serde(default)]
    pub llm_api_key: String,
    #[serde(default = "default_llm_model_generate")]
    pub llm_model_generate: String,
    #[serde(default = "default_llm_model_evaluate")]
    pub llm_model_evaluate: String,
    #[serde(default = "default_rpm")]
    pub llm_rpm: u32,
    #[serde(default = "default_max_retries")]
    pub llm_max_retries: u32,
    #[serde(default = "default_timeout")]
    pub llm_timeout_seconds: u64,
}

fn default_batch_size() -> usize {
    1000
}
fn default_stardict_path() -> String {
    "../data/stardict.db".to_string()
}
fn default_wn_path() -> String {
    "../data/wn.db".to_string()
}
fn default_llm_provider() -> String {
    "deepseek".to_string()
}
fn default_llm_base_url() -> String {
    "https://api.deepseek.com".to_string()
}
fn default_llm_model_generate() -> String {
    "deepseek-v4-flash".to_string()
}
fn default_llm_model_evaluate() -> String {
    "deepseek-v4-pro".to_string()
}
fn default_rpm() -> u32 {
    60
}
fn default_max_retries() -> u32 {
    3
}
fn default_timeout() -> u64 {
    60
}

impl EtlConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        // Load .env from current directory or parent (for cargo run from workspace root)
        let _ = dotenvy::dotenv();
        let s = Config::builder()
            .add_source(Environment::default().separator("__"))
            .build()?;
        Ok(s.try_deserialize()?)
    }
}
