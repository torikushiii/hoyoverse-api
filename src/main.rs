use std::net::SocketAddr;
use std::sync::Arc;
use axum::Router;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    fmt::format::FmtSpan
};
use hyper::http::HeaderValue;

mod routes;
mod config;
mod db;
mod ratelimit;

use config::Settings;
use db::DatabaseConnections;
use ratelimit::RateLimiter;

use hoyoverse_api::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Settings::new().expect("Failed to load configuration");

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_span_events(FmtSpan::CLOSE)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .with_ansi(config.logging.format == "pretty")
        .compact();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| config.logging.level.clone()),
        ))
        .with(fmt_layer)
        .init();

    let db = DatabaseConnections::new(&config).await?;
    let db = Arc::new(db);

    let rate_limiter = Arc::new(RateLimiter::new(Arc::new(db.redis.clone())).await?);

    let cors = if config.server.cors_origins.contains(&"*".to_string()) {
        CorsLayer::permissive()
    } else {
        CorsLayer::new().allow_origin(
            config.server.cors_origins.iter()
                .map(|origin| {
                    HeaderValue::from_str(origin)
                        .expect("Invalid CORS origin")
                })
                .collect::<Vec<_>>()
        )
    };

    let app = Router::new()
        .nest("/mihoyo", routes::mihoyo_routes())
        .layer(cors)
        .with_state((db, rate_limiter));

    let addr = SocketAddr::new(
        config.server.host.parse().expect("Invalid host address"),
        config.server.port,
    );
    
    tracing::info!("Server listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    
    let app = app.into_make_service_with_connect_info::<SocketAddr>();
    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
}
