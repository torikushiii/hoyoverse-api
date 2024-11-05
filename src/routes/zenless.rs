use axum::{
    Router,
    routing::get,
    response::Json,
    extract::{Path, State, ConnectInfo},
    http::StatusCode,
};
use serde::{Serialize, Deserialize};
use crate::routes::AppState;
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
    let rate_limit = rate_limiter
        .check_rate_limit_with_ip(
            "zenless:codes",
            addr.ip(),
            db.redis.get_rate_limit_config(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !rate_limit.is_allowed() {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(Json(vec![GameCode {
        code: "Example Zenless Code".to_string(),
        description: "Example code".to_string(),
        expiry: Some(chrono::Utc::now() + chrono::Duration::days(7)),
    }]))
}

async fn news(
    Path(category): Path<String>,
    State((_, _)): State<AppState>,
) -> Json<Vec<String>> {
    Json(vec![format!("Zenless {} news", category)])
} 