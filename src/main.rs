use clap::Parser;
use openai_tts_bridge::{run_server, Config};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if available
    dotenvy::dotenv().ok();

    let config = Config::parse();

    // Initialize structured logging
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    run_server(config).await?;

    Ok(())
}
