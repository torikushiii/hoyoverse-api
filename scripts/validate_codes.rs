use std::sync::Arc;
use hoyoverse_api::{
    config::Settings,
    db::DatabaseConnections,
    services::code_validator::CodeValidationService,
};
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    fmt::format::FmtSpan,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_span_events(FmtSpan::CLOSE)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .compact();

    let filter = tracing_subscriber::EnvFilter::new(
        std::env::var("RUST_LOG")
            .unwrap_or_else(|_| "debug".to_string())
    )
    .add_directive("html5ever=warn".parse().unwrap())
    .add_directive("selectors=warn".parse().unwrap())
    .add_directive("scraper=warn".parse().unwrap())
    .add_directive("reqwest=warn".parse().unwrap())
    .add_directive("h2=warn".parse().unwrap())
    .add_directive("hyper=warn".parse().unwrap())
    .add_directive("rustls=warn".parse().unwrap());

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();

    let config = Settings::new().expect("Failed to load configuration");

    let db = DatabaseConnections::new(&config).await?;
    let db = Arc::new(db);
    let config = Arc::new(config);

    let validator = CodeValidationService::new(db, config);
    validator.validate_all_codes().await;

    Ok(())
}