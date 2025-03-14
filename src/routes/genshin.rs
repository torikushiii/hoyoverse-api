use axum::{
    Router,
    routing::get,
    extract::State,
    http::HeaderMap,
    response::Json,
};
use crate::types::{CodesResponse, NewsItemResponse, CalendarResponse};
use super::{AppState, utils::{self, NewsQuery}};
use crate::error::ApiError;
use crate::resolvers::genshin::GenshinResolver;
use crate::config::Settings;
use tracing::{debug, error};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/codes", get(codes))
        .route("/news/:category", get(news))
        .route("/calendar", get(calendar))
}

#[axum::debug_handler]
async fn codes(
    state: State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CodesResponse>, ApiError> {
    utils::handle_codes("genshin", state, headers).await
}

#[axum::debug_handler]
async fn news(
    category: axum::extract::Path<String>,
    query: axum::extract::Query<NewsQuery>,
    state: State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<NewsItemResponse>>, ApiError> {
    utils::handle_news("genshin", category, query, state, headers).await
}

#[axum::debug_handler]
async fn calendar(
    State((db, rate_limiter)): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CalendarResponse>, ApiError> {
    let endpoint = "/genshin/calendar";
    utils::log_endpoint_metrics(endpoint, &(db.clone(), rate_limiter.clone())).await;

    debug!("Handling request for /genshin/calendar");

    let rate_limit = rate_limiter
        .check_rate_limit_with_headers(
            "genshin:calendar",
            &headers,
            db.redis.get_rate_limit_config(),
        )
        .await
        .map_err(|e| {
            error!("Rate limit check failed: {}", e);
            ApiError::internal_server_error("Failed to check rate limit")
        })?;

    if !rate_limit.is_allowed() {
        debug!("Rate limit exceeded for /genshin/calendar");
        return Err(ApiError::rate_limit_exceeded("Too many requests"));
    }

    let cache_key = "genshin_calendar";
    let cached_data = db.get_cached_data("calendar".to_string(), cache_key.to_string())
        .await
        .map_err(|e| ApiError::cache_error(format!("Cache error: {}", e)))?;

    if let Some(data) = cached_data {
        if let Ok(calendar) = serde_json::from_str::<CalendarResponse>(&data) {
            debug!("Returning cached calendar data");
            return Ok(Json(calendar));
        }
    }

    let config = Settings::new().map_err(|e| {
        error!("Failed to load config: {}", e);
        ApiError::internal_server_error("Failed to load configuration")
    })?;

    match GenshinResolver::fetch_calendar(&config, &db.mongo).await {
        Ok(calendar) => {
            if let Ok(json) = serde_json::to_string(&calendar) {
                if let Err(e) = db.redis.set_cached(cache_key, &json, 3600).await {
                    error!("Failed to cache calendar data: {}", e);
                }
            }
            Ok(Json(calendar))
        }
        Err(e) => {
            error!("Failed to fetch calendar data: {}", e);
            Err(ApiError::internal_server_error("Failed to fetch calendar data"))
        }
    }
}