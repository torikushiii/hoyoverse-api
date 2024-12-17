use axum::{
    response::Json,
    extract::{State, Query, Path},
    http::HeaderMap,
};
use futures_util::TryStreamExt;
use mongodb::bson;
use serde::Deserialize;
use tracing::{debug, error};

use crate::{
    error::ApiError,
    types::{GameCode, CodesResponse, NewsItem, GameCodeResponse, NewsItemResponse},
    utils::lang::parse_language_code,
};
use super::AppState;

#[derive(Debug, Deserialize)]
pub struct NewsQuery {
    pub lang: Option<String>,
}

pub async fn log_endpoint_metrics(
    endpoint: &str,
    state: &AppState,
) {
    let (db, _) = state;
    if let Err(e) = db.redis.log_endpoint_hit(endpoint).await {
        error!("Failed to log endpoint metrics: {}", e);
    }
}

pub async fn handle_codes(
    game_name: &str,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CodesResponse>, ApiError> {
    let endpoint = format!("/{}/codes", game_name);
    log_endpoint_metrics(&endpoint, &state).await;

    let (db, rate_limiter) = state;
    debug!("Handling request for /{}/codes", game_name);

    let rate_limit = rate_limiter
        .check_rate_limit_with_headers(
            &format!("{}:codes", game_name),
            &headers,
            db.redis.get_rate_limit_config(),
        )
        .await
        .map_err(|e| {
            error!("Rate limit check failed: {}", e);
            ApiError::internal_server_error("Failed to check rate limit")
        })?;

    if !rate_limit.is_allowed() {
        debug!("Rate limit exceeded for /{}/codes", game_name);
        return Err(ApiError::rate_limit_exceeded("Too many requests"));
    }

    let cache_key = format!("{}_codes", game_name);
    let cached_data = db.redis.get_cached(&cache_key)
        .await
        .map_err(|e| ApiError::cache_error(format!("Cache error: {}", e)))?;

    if let Some(data) = cached_data {
        if let Ok(codes) = serde_json::from_str::<CodesResponse>(&data) {
            debug!("Returning cached codes data");
            return Ok(Json(codes));
        }
    }

    let collection = db.mongo.collection::<GameCode>(&format!("{}_codes", game_name));

    let active_filter = bson::doc! { "active": true };
    let mut active = Vec::new();
    match collection.find(active_filter).await {
        Ok(mut cursor) => {
            while let Ok(Some(code)) = cursor.try_next().await {
                active.push(GameCodeResponse::from(code));
            }
        }
        Err(e) => {
            error!("Failed to query active codes: {}", e);
            return Err(ApiError::database_error("Failed to query active codes"));
        }
    }

    let inactive_filter = bson::doc! { "active": false };
    let mut inactive = Vec::new();
    match collection.find(inactive_filter).await {
        Ok(mut cursor) => {
            while let Ok(Some(code)) = cursor.try_next().await {
                inactive.push(GameCodeResponse::from(code));
            }
        }
        Err(e) => {
            error!("Failed to query inactive codes: {}", e);
            return Err(ApiError::database_error("Failed to query inactive codes"));
        }
    }

    let has_codes = !active.is_empty() || !inactive.is_empty();
    let response = CodesResponse { active, inactive };

    if has_codes {
        if let Ok(json) = serde_json::to_string(&response) {
            if let Err(e) = db.redis.set_cached(&cache_key, &json, 300).await {
                error!("Failed to cache codes: {}", e);
            }
        }
    }

    Ok(Json(response))
}

pub async fn handle_news(
    game_name: &str,
    Path(category): Path<String>,
    Query(query): Query<NewsQuery>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<NewsItemResponse>>, ApiError> {
    let endpoint = format!("/{}/news/{}", game_name, category);
    log_endpoint_metrics(&endpoint, &state).await;

    let (db, rate_limiter) = state;
    debug!("Handling request for /{}/news/{}", game_name, category);

    let rate_limit = rate_limiter
        .check_rate_limit_with_headers(
            &format!("{}:news", game_name),
            &headers,
            db.redis.get_rate_limit_config(),
        )
        .await
        .map_err(|e| {
            error!("Rate limit check failed: {}", e);
            ApiError::internal_server_error("Failed to check rate limit")
        })?;

    if !rate_limit.is_allowed() {
        return Err(ApiError::rate_limit_exceeded("Too many requests"));
    }

    let collection = db.mongo.collection::<NewsItem>(&format!("{}_news", game_name));

    let lang = query.lang.as_deref().unwrap_or("en");
    let normalized_lang = parse_language_code(lang);

    let normalized_category = match category.as_str() {
        "event" | "events" => "event",
        "notice" | "notices" => "notice",
        "info" | "information" => "info",
        _ => return Err(ApiError::bad_request("Invalid category"))
    };

    let filter = bson::doc! {
        "type": normalized_category,
        "lang": normalized_lang
    };
    debug!("Querying with filter: {:?}", filter);

    let cursor = match collection.find(filter.clone()).await {
        Ok(cursor) => cursor,
        Err(e) => {
            error!("Failed to query news with filter {:?}: {}", filter, e);
            return Err(ApiError::database_error("Failed to query news"));
        }
    };

    let mut news = Vec::new();
    let mut cursor = cursor;
    while let Ok(Some(doc)) = cursor.try_next().await {
        news.push(NewsItemResponse::from(doc));
    }

    if news.is_empty() {
        debug!(
            "No news items found for category: {} with language: {}. Filter: {:?}",
            category,
            normalized_lang,
            filter
        );
        return Err(ApiError::not_found("No news items found"));
    }

    Ok(Json(news))
}