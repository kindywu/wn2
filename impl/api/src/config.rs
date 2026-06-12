use config::{Config, Environment};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
    pub database_url: String,
    #[serde(default = "default_host")]
    pub api_host: String,
    #[serde(default = "default_port")]
    pub api_port: u16,
    #[serde(default = "default_token")]
    pub api_bearer_token: String,
}

fn default_host() -> String { "127.0.0.1".to_string() }
fn default_port() -> u16 { 8000 }
fn default_token() -> String { "changeme_in_production".to_string() }

impl ApiConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let _ = dotenvy::dotenv();
        let s = Config::builder()
            .add_source(Environment::default().separator("__"))
            .build()?;
        Ok(s.try_deserialize()?)
    }
}
