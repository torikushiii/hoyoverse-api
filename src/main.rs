use anyhow::Context as _;
use hoyoverse_api::global::Global;
use hoyoverse_api::{config, http, scraper, validator};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    metrics_prometheus::install();

    let config = config::Config::load()?;

    tracing_subscriber::fmt()
        .with_file(true)
        .with_line_number(true)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .parse_lossy(&config.level),
        )
        .init();

    tracing::info!("starting hoyoverse api");

    let global = Global::init(config).await?;

    tracing::info!("all services initialized");

    tokio::select! {
        r = http::run(global.clone()) => {
            r.context("http server error")?;
            anyhow::bail!("http server stopped unexpectedly");
        }
        r = validator::run(global.clone()) => {
            r.context("validator error")?;
            anyhow::bail!("validator stopped unexpectedly");
        }
        r = scraper::run(global.clone()) => {
            r.context("scraper error")?;
            anyhow::bail!("scraper stopped unexpectedly");
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("shutting down");
        }
    }

    Ok(())
}
