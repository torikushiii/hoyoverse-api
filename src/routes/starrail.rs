use axum::{
    Router,
    routing::get,
    response::Json,
    extract::{Path, State, ConnectInfo},
    http::StatusCode,
};
use crate::types::{GameCode, CodesResponse, NewsItem, CodeStatus};
use crate::routes::AppState;
use mongodb::bson;
use futures_util::TryStreamExt;
use tracing::{error, debug};
use std::net::SocketAddr;

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
    debug!("Handling request for /starrail/codes from {}", addr.ip());

    let rate_limit = rate_limiter
        .check_rate_limit_with_ip(
            "starrail:codes",
            addr.ip(),
            db.redis.get_rate_limit_config(),
        )
        .await
        .map_err(|e| {
            error!("Rate limit check failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if !rate_limit.is_allowed() {
        debug!("Rate limit exceeded for /starrail/codes");
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    let cached_data = db.get_cached_data("starrail_codes".to_string(), "codes".to_string()).await;
    if let Ok(Some(data)) = cached_data {
        if let Ok(codes) = serde_json::from_str::<CodesResponse>(&data) {
            debug!("Returning cached codes data");
            return Ok(Json(codes));
        }
    }

    let collection = db.mongo.collection::<GameCode>("starrail_codes");
    
    let mut active = Vec::new();
    let mut inactive = Vec::new();

    if let Ok(mut cursor) = collection.find(bson::doc! {}).await {
        while let Ok(Some(code)) = cursor.try_next().await {
            match code.status {
                CodeStatus::Active => active.push(code),
                CodeStatus::Inactive => inactive.push(code),
            }
        }
    }

    let response = CodesResponse { active, inactive };

    if let Ok(json) = serde_json::to_string(&response) {
        if let Err(e) = db.redis.set_cached("starrail_codes", &json, 300).await {
            error!("Failed to cache codes: {}", e);
        }
    }

    Ok(Json(response))
}

#[axum::debug_handler]
async fn news(
    Path(category): Path<String>,
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<Vec<NewsItem>>, StatusCode> {
    let (db, rate_limiter) = state;
    
    let rate_limit = rate_limiter
        .check_rate_limit_with_ip(
            "starrail:news",
            addr.ip(),
            db.redis.get_rate_limit_config(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !rate_limit.is_allowed() {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    let cache_key = format!("starrail_news_{}", category);
    let cached_data = db.get_cached_data("starrail_news".to_string(), cache_key.clone()).await;
    if let Ok(Some(data)) = cached_data {
        if let Ok(news) = serde_json::from_str::<Vec<NewsItem>>(&data) {
            return Ok(Json(news));
        }
    }

    let collection = db.mongo.collection::<NewsItem>("starrail_news");
    let filter = bson::doc! { "category": &category };
    
    let news = match collection.find(filter).await {
        Ok(cursor) => cursor.try_collect().await.unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    Ok(Json(news))
} 