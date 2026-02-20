use global::Global;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

mod config;
mod database;
mod games;
mod global;
mod http;
mod scraper;
mod util;
mod validator;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
            if let Err(e) = r {
                tracing::error!("http server error: {:#}", e);
            }
        }
        r = validator::run(global.clone()) => {
            if let Err(e) = r {
                tracing::error!("validator error: {:#}", e);
            }
        }
        r = scraper::run(global.clone()) => {
            if let Err(e) = r {
                tracing::error!("scraper error: {:#}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("shutting down");
        }
    }

    Ok(())
}
