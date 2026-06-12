use clap::{Parser, Subcommand};
use tracing_subscriber::{fmt, EnvFilter};

mod config;
mod db;
mod common;
mod llm_client;
mod import_stardict;
mod import_wn;
mod fill_gaps;
mod evaluate;
mod quality_report;
mod generate_examples;

use crate::llm_client::DeepSeekClient;
use crate::config::EtlConfig;

#[derive(Parser)]
#[command(name = "dict-etl")]
#[command(about = "dict ETL pipeline")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Import from stardict.db (run after WN import)
    Stardict {
        #[arg(long, default_value = "full")]
        mode: String,
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Import from wn.db (run first)
    Wn {
        #[arg(long, default_value = "full")]
        mode: String,
        #[arg(long)]
        limit: Option<usize>,
    },
    /// LLM gap filling
    FillGaps,
    /// LLM quality evaluation
    Evaluate,
    /// Data quality report (no LLM)
    QualityReport,
    /// Generate examples for tagged words
    GenExamples {
        #[arg(long)]
        limit: Option<usize>,
    },
}

fn make_llm(cfg: &EtlConfig) -> DeepSeekClient {
    DeepSeekClient::new(
        cfg.llm_base_url.clone(),
        cfg.llm_api_key.clone(),
        cfg.llm_model_generate.clone(),
        cfg.llm_model_evaluate.clone(),
        cfg.llm_timeout_seconds,
        cfg.llm_rpm,
        cfg.llm_max_retries,
    )
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cli = Cli::parse();
    let cfg = config::EtlConfig::from_env()?;
    let pool = db::connect(&cfg.database_url).await?;

    match cli.command {
        Commands::Stardict { mode, limit } => {
            let mode = match mode.as_str() {
                "full" => dict_models::enums::ImportMode::Full,
                "incremental" => dict_models::enums::ImportMode::Incremental,
                _ => anyhow::bail!("Unknown mode: {mode}"),
            };
            import_stardict::run(&pool, &cfg.etl_sqlite_stardict_path, mode, cfg.etl_batch_size, limit).await?;
        }
        Commands::Wn { mode, limit } => {
            let mode = match mode.as_str() {
                "full" => dict_models::enums::ImportMode::Full,
                "incremental" => dict_models::enums::ImportMode::Incremental,
                _ => anyhow::bail!("Unknown mode: {mode}"),
            };
            import_wn::run(&pool, &cfg.etl_sqlite_wn_path, mode, cfg.etl_batch_size, limit).await?;
        }
        Commands::FillGaps => {
            let llm = make_llm(&cfg);
            fill_gaps::run(&pool, &llm).await?;
        }
        Commands::Evaluate => {
            let llm = make_llm(&cfg);
            evaluate::run(&pool, &llm).await?;
        }
        Commands::QualityReport => {
            quality_report::run(&pool).await?;
        }
        Commands::GenExamples { limit } => {
            let llm = make_llm(&cfg);
            generate_examples::run(&pool, &llm, limit).await?;
        }
    }

    Ok(())
}
