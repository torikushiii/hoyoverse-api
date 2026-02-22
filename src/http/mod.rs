use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use axum::extract::Request;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use axum_prometheus::BaseMetricLayer;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer, MaxAge};
use tower_http::trace::TraceLayer;
use tracing::Span;

use crate::global::Global;

pub mod error;
pub mod routes;

fn classify_user_agent(ua: &str) -> &'static str {
    let ua = ua.to_ascii_lowercase();
    if ua.starts_with("mozilla/") {
        "browser"
    } else if ua.starts_with("curl/") {
        "curl"
    } else if ua.contains("python") {
        "python"
    } else if ua.starts_with("go-http-client/") {
        "go"
    } else if ua.contains("axios") || ua.starts_with("node") {
        "node"
    } else if ua.contains("bot") || ua.contains("spider") || ua.contains("crawler") {
        "bot"
    } else if ua.is_empty() {
        "none"
    } else {
        "other"
    }
}

async fn track_client(
    matched_path: Option<axum::extract::MatchedPath>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let client = req
        .headers()
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(classify_user_agent)
        .unwrap_or("none");

    let endpoint = matched_path
        .as_ref()
        .map(|p| p.as_str().to_owned())
        .unwrap_or_else(|| "unknown".to_owned());

    metrics::counter!(
        "http_requests_by_client_total",
        "endpoint" => endpoint,
        "client" => client,
    )
    .increment(1);

    next.run(req).await
}

fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::any())
        .allow_methods(AllowMethods::list([hyper::Method::GET]))
        .allow_headers(AllowHeaders::any())
        .max_age(MaxAge::exact(Duration::from_secs(7200)))
}

fn app(global: Arc<Global>) -> Router {
    Router::new()
        .route("/metrics", get(metrics_handler))
        .nest("/mihoyo", routes::routes(&global))
        .with_state(global)
        .fallback(not_found)
        .layer(
            ServiceBuilder::new()
                .layer(BaseMetricLayer::new())
                .layer(CompressionLayer::new())
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(|req: &Request| {
                            tracing::info_span!(
                                "request",
                                method = %req.method(),
                                uri = %req.uri(),
                                status = tracing::field::Empty,
                            )
                        })
                        .on_request(|req: &Request, _span: &Span| {
                            tracing::info!(method = %req.method(), uri = %req.uri(), "incoming request");
                        })
                        .on_response(|res: &Response, latency: Duration, span: &Span| {
                            span.record("status", res.status().as_u16());
                            tracing::info!(status = res.status().as_u16(), latency = ?latency, "response");
                        })
                        .on_failure(()),
                )
                .layer(cors_layer()),
        )
}

async fn metrics_handler() -> impl axum::response::IntoResponse {
    let encoder = prometheus::TextEncoder::new();
    let body = encoder
        .encode_to_string(&prometheus::gather())
        .unwrap_or_default();
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}

#[tracing::instrument]
async fn not_found() -> error::ApiError {
    error::ApiError::not_found(error::ApiErrorCode::ROUTE_NOT_FOUND, "route not found")
}

#[tracing::instrument(name = "HTTP", skip_all)]
pub async fn run(global: Arc<Global>) -> anyhow::Result<()> {
    let bind = global.config.api.bind;

    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .context("failed to bind HTTP server")?;

    tracing::info!(%bind, "http server listening");

    axum::serve(
        listener,
        app(global).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .context("http server error")?;

    Ok(())
}
