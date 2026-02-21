use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::{ConnectInfo, Path, State};
use axum::http::Response;
use axum::routing::get;
use axum::Router;
use hyper::StatusCode;
use tower_governor::errors::GovernorError;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::KeyExtractor;
use tower_governor::GovernorLayer;

use crate::config::RateLimitConfig;
use crate::database::redemption_code::{RedemptionCode, RedemptionCodeResponse};
use crate::games::Game;
use crate::global::Global;
use crate::http::error::{ApiError, ApiErrorCode};

#[derive(Clone)]
struct CloudflareIp;

impl KeyExtractor for CloudflareIp {
    type Key = String;

    fn extract<T>(&self, req: &axum::http::Request<T>) -> Result<Self::Key, GovernorError> {
        let headers = req.headers();

        let ip = headers
            .get("CF-Connecting-IP")
            .or_else(|| headers.get("X-Real-IP"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .or_else(|| {
                headers
                    .get("X-Forwarded-For")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.split(',').next())
                    .map(|s| s.trim().to_string())
            })
            .or_else(|| {
                req.extensions()
                    .get::<ConnectInfo<SocketAddr>>()
                    .map(|ci| ci.0.ip().to_string())
            });

        ip.ok_or(GovernorError::UnableToExtractKey)
    }
}

pub fn routes(rate_limit: &RateLimitConfig) -> Router<Arc<Global>> {
    let governor = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(rate_limit.per_second)
            .burst_size(rate_limit.burst_size)
            .key_extractor(CloudflareIp)
            .finish()
            .unwrap(),
    );

    Router::new()
        .route("/:game/codes", get(get_codes))
        .layer(GovernorLayer { config: governor })
}

#[derive(serde::Serialize)]
struct CodesResponse {
    active: Vec<RedemptionCodeResponse>,
    inactive: Vec<RedemptionCodeResponse>,
}

fn json_response(bytes: Bytes) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Body::from(bytes))
        .unwrap()
}

/// GET /mihoyo/:game/codes
///
/// Returns all redemption codes for the given game, split by active/inactive.
#[tracing::instrument(skip(global))]
async fn get_codes(
    State(global): State<Arc<Global>>,
    Path(game_slug): Path<String>,
) -> Result<Response<Body>, ApiError> {
    let game = Game::from_slug(&game_slug)
        .ok_or_else(|| ApiError::not_found(ApiErrorCode::UNKNOWN_GAME, "unknown game"))?;

    let cache_key = format!("/mihoyo/{game_slug}/codes");

    if let Some(bytes) = global.response_cache.get(&cache_key).await {
        return Ok(json_response(bytes));
    }

    let all_codes = RedemptionCode::find_all(&global.db, game)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to query codes");
            ApiError::internal_server_error(ApiErrorCode::DATABASE_ERROR, "failed to query codes")
        })?;

    let (active, inactive): (Vec<_>, Vec<_>) = all_codes.into_iter().partition(|c| c.active);
    let response = CodesResponse {
        active: active.into_iter().map(Into::into).collect(),
        inactive: inactive.into_iter().map(Into::into).collect(),
    };

    let bytes =
        Bytes::from(serde_json::to_vec(&response).expect("CodesResponse is always serializable"));
    global.response_cache.insert(cache_key, bytes.clone()).await;

    Ok(json_response(bytes))
}
