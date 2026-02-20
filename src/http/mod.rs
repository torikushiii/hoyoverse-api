use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use axum::extract::Request;
use axum::response::Response;
use axum::Router;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer, MaxAge};
use tower_http::trace::{DefaultOnFailure, TraceLayer};
use tracing::Span;

use crate::global::Global;

pub mod error;
pub mod routes;

fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::any())
        .allow_methods(AllowMethods::list([hyper::Method::GET]))
        .allow_headers(AllowHeaders::any())
        .max_age(MaxAge::exact(Duration::from_secs(7200)))
}

fn app(global: Arc<Global>) -> Router {
    Router::new()
        .nest("/mihoyo", routes::routes())
        .with_state(global)
        .fallback(not_found)
        .layer(
            ServiceBuilder::new()
                .layer(CompressionLayer::new())
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(|req: &Request| {
                            tracing::info_span!(
                                "request",
                                "request.method" = %req.method(),
                                "request.uri" = %req.uri(),
                                "response.status_code" = tracing::field::Empty,
                            )
                        })
                        .on_failure(DefaultOnFailure::new().level(tracing::Level::DEBUG))
                        .on_response(|res: &Response, _, span: &Span| {
                            span.record("response.status_code", res.status().as_u16());
                        }),
                )
                .layer(cors_layer()),
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

    axum::serve(listener, app(global))
        .await
        .context("http server error")?;

    Ok(())
}
