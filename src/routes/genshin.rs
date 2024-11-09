use axum::{
    Router,
    routing::get,
    response::Json,
    extract::{Path, State, ConnectInfo, Query},
    http::StatusCode,
};
use crate::types::{GameCode, CodesResponse, NewsItem, GameCodeResponse, NewsItemResponse};
use crate::routes::AppState;
use mongodb::bson;
use futures_util::TryStreamExt;
use tracing::{error, debug};
use std::net::SocketAddr;
use crate::utils::lang::parse_language_code;
use serde::Deserialize;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/codes", get(codes))
        .route("/news/:category", get(news))
}

#[axum::debug_handler]
async fn codes(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<CodesResponse>, StatusCode> {
    let (db, rate_limiter) = state;
    debug!("Handling request for /genshin/codes from {}", addr.ip());

    let rate_limit = rate_limiter
        .check_rate_limit_with_ip(
            "genshin:codes",
            addr.ip(),
            db.redis.get_rate_limit_config(),
        )
        .await
        .map_err(|e| {
            error!("Rate limit check failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if !rate_limit.is_allowed() {
        debug!("Rate limit exceeded for /genshin/codes");
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    let cached_data = db.get_cached_data("genshin_codes".to_string(), "codes".to_string()).await;
    if let Ok(Some(data)) = cached_data {
        if let Ok(codes) = serde_json::from_str::<CodesResponse>(&data) {
            debug!("Returning cached codes data");
            return Ok(Json(codes));
        }
    }

    let collection = db.mongo.collection::<GameCode>("genshin_codes");
    
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
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
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
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    let has_codes = !active.is_empty() || !inactive.is_empty();
    let response = CodesResponse { active, inactive };

    if has_codes {
        if let Ok(json) = serde_json::to_string(&response) {
            if let Err(e) = db.redis.set_cached("genshin_codes", &json, 300).await {
                error!("Failed to cache codes: {}", e);
            }
        }
    }

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct NewsQuery {
    lang: Option<String>,
}

#[axum::debug_handler]
async fn news(
    Path(category): Path<String>,
    Query(query): Query<NewsQuery>,
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<Vec<NewsItemResponse>>, StatusCode> {
    let (db, rate_limiter) = state;
    debug!("Handling request for /genshin/news/{} from {}", category, addr.ip());
    
    let rate_limit = rate_limiter
        .check_rate_limit_with_ip(
            "genshin:news",
            addr.ip(),
            db.redis.get_rate_limit_config(),
        )
        .await
        .map_err(|e| {
            error!("Rate limit check failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if !rate_limit.is_allowed() {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    let collection = db.mongo.collection::<NewsItem>("genshin_news");
    
    let lang = query.lang
        .as_deref()
        .unwrap_or("en");
    let normalized_lang = parse_language_code(lang);
    
    let filter = bson::doc! { 
        "type": &category,
        "lang": normalized_lang
    };
    debug!("Querying with filter: {:?}", filter);
    
    let cursor = match collection.find(filter.clone()).await {
        Ok(cursor) => cursor,
        Err(e) => {
            error!("Failed to query news with filter {:?}: {}", filter, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
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
    } else {
        debug!("Successfully found {} news items", news.len());
    }

    Ok(Json(news))
} 