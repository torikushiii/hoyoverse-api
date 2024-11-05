use axum::{
    Router,
    routing::get,
    response::Json,
    extract::{Path, State, ConnectInfo},
    http::StatusCode,
};
use serde::{Serialize, Deserialize};
use futures_util::TryStreamExt;
use crate::routes::AppState;
use tracing::{error, debug};
use std::net::SocketAddr;

#[derive(Debug, Serialize, Deserialize)]
struct GameCode {
    code: String,
    description: String,
    expiry: Option<chrono::DateTime<chrono::Utc>>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/codes", get(codes))
        .route("/news/:category", get(news))
}

async fn codes(
    State((db, rate_limiter)): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<Vec<GameCode>>, StatusCode> {
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

    if let Ok(Some(cached_data)) = db.get_cached_data("genshin_codes", "active_codes").await {
        if let Ok(codes) = serde_json::from_str::<Vec<GameCode>>(&cached_data) {
            debug!("Returning cached codes data");
            return Ok(Json(codes));
        }
    }

    let collection = db.mongo.collection::<GameCode>("genshin_codes");
    let cursor = collection.find(bson::doc! {}).await;
    
    let codes = match cursor {
        Ok(cursor) => {
            let codes: Vec<GameCode> = cursor.try_collect().await.unwrap_or_default();
            if !codes.is_empty() {
                if let Ok(json) = serde_json::to_string(&codes) {
                    if let Err(e) = db.redis.set_cached("genshin_codes", &json, 300).await {
                        error!("Failed to cache codes: {}", e);
                    }
                }
            }
            debug!("Retrieved {} codes from MongoDB", codes.len());
            codes
        },
        Err(e) => {
            error!("Failed to query MongoDB: {}", e);
            vec![GameCode {
                code: "DEFAULTCODE".to_string(),
                description: "Default fallback code".to_string(),
                expiry: Some(chrono::Utc::now() + chrono::Duration::days(7)),
            }]
        },
    };
    
    Ok(Json(codes))
}

async fn news(
    Path(category): Path<String>,
    State((_, _)): State<AppState>,
) -> Json<Vec<String>> {
    Json(vec![format!("Genshin {} news", category)])
} 