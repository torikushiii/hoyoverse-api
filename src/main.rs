use std::net::SocketAddr;
use std::sync::Arc;
use axum::Router;
use tower_http::{
    cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer},
    compression::{CompressionLayer, CompressionLevel},
    request_id::{MakeRequestId, RequestId, SetRequestIdLayer, PropagateRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{error, Span};
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    fmt::format::FmtSpan
};
use hyper::{Request, Response, Method};
use hyper::header::{HeaderName, HeaderValue};
use uuid::Uuid;
use std::time::Duration;

use hoyoverse_api::{
    config::Settings,
    db::DatabaseConnections,
    ratelimit::RateLimiter,
    crons::Scheduler,
    services::code_validator::CodeValidationService,
    routes,
};

use hoyoverse_api::utils::datetime::set_start_time;

#[derive(Clone)]
struct TraceRequestId;

impl MakeRequestId for TraceRequestId {
    fn make_request_id<B>(&mut self, _: &Request<B>) -> Option<RequestId> {
        Some(RequestId::new(
            HeaderValue::from_str(&Uuid::new_v4().to_string())
                .expect("UUID should be valid header value")
        ))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    set_start_time();

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

    let filter = tracing_subscriber::EnvFilter::new(
        std::env::var("RUST_LOG")
            .unwrap_or_else(|_| config.logging.level.clone())
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

    let db = DatabaseConnections::new(&config).await?;
    let db = Arc::new(db);
    let config = Arc::new(config);

    let scheduler = Scheduler::new(db.clone(), config.clone());
    if let Err(e) = scheduler.start().await {
        error!("Failed to start scheduler: {}", e);
        std::process::exit(1);
    }

    let validator = CodeValidationService::new(db.clone(), config.clone());
    tokio::spawn(async move {
        if let Err(e) = validator.start().await {
            error!("Failed to start code validation service: {}", e);
            std::process::exit(1);
        }
    });

    let rate_limiter = Arc::new(RateLimiter::new(Arc::new(db.redis.clone())).await?);

    let cors = if config.server.cors_origins.contains(&"*".to_string()) {
        CorsLayer::permissive()
    } else {
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(
                config.server.cors_origins.iter()
                    .map(|origin| {
                        HeaderValue::from_str(origin)
                            .expect("Invalid CORS origin")
                    })
                    .collect::<Vec<_>>()
            ))
            .allow_methods(AllowMethods::list([Method::GET]))
            .allow_headers(AllowHeaders::list([
                HeaderName::from_static("content-type"),
                HeaderName::from_static("x-request-id"),
            ]))
            .max_age(Duration::from_secs(7200))
    };

    let compression_layer = CompressionLayer::new()
        .br(true)
        .gzip(true)
        .deflate(true)
        .zstd(true)
        .quality(CompressionLevel::Default);

    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(|req: &Request<_>| {
            let matched_path = req
                .extensions()
                .get::<axum::extract::MatchedPath>()
                .map(axum::extract::MatchedPath::as_str);

            tracing::info_span!(
                "request",
                "request.method" = %req.method(),
                "request.uri" = %req.uri(),
                "request.matched_path" = %matched_path.unwrap_or("<not found>"),
                "response.status_code" = tracing::field::Empty,
            )
        })
        .on_response(|res: &Response<_>, _latency, span: &Span| {
            span.record("response.status_code", res.status().as_u16());
        });

    let app = Router::new()
        .nest("/mihoyo", routes::mihoyo_routes())
        .layer(cors)
        .layer(compression_layer)
        .layer(trace_layer)
        .layer(SetRequestIdLayer::x_request_id(TraceRequestId))
        .layer(PropagateRequestIdLayer::x_request_id())
        .with_state((db, rate_limiter));

    let addr = SocketAddr::new(
        config.server.host.parse().expect("Invalid host address"),
        config.server.port,
    );

    tracing::info!("Server listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    let app = app.into_make_service();

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
