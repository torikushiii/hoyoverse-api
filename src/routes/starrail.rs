use axum::{
    Router,
    routing::get,
    extract::State,
    http::HeaderMap,
    response::Json,
};
use crate::{
    types::{CodesResponse, NewsItemResponse, CalendarResponse},
    resolvers::starrail::StarRailResolver,
    error::ApiError,
    config::Settings,
};
use super::{AppState, utils::{self, NewsQuery}};
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
    utils::handle_codes("starrail", state, headers).await
}

#[axum::debug_handler]
async fn news(
    category: axum::extract::Path<String>,
    query: axum::extract::Query<NewsQuery>,
    state: State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<NewsItemResponse>>, ApiError> {
    utils::handle_news("starrail", category, query, state, headers).await
}

#[axum::debug_handler]
async fn calendar(
    State((db, rate_limiter)): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CalendarResponse>, ApiError> {
    debug!("Handling request for /starrail/calendar");

    let rate_limit = rate_limiter
        .check_rate_limit_with_headers(
            "starrail:calendar",
            &headers,
            db.redis.get_rate_limit_config(),
        )
        .await
        .map_err(|e| {
            error!("Rate limit check failed: {}", e);
            ApiError::internal_server_error("Failed to check rate limit")
        })?;

    if !rate_limit.is_allowed() {
        debug!("Rate limit exceeded for /starrail/calendar");
        return Err(ApiError::rate_limit_exceeded("Too many requests"));
    }

    let cache_key = "starrail_calendar";
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

    match StarRailResolver::fetch_calendar(&config).await {
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