use axum::{
    Router,
    routing::get,
    response::Json,
};
use serde::Serialize;
use std::sync::Arc;
use crate::db::DatabaseConnections;
use crate::ratelimit::RateLimiter;

mod genshin;
mod starrail;
mod zenless;

pub type AppState = (Arc<DatabaseConnections>, Arc<RateLimiter>);

pub fn mihoyo_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(root_handler))
        .nest("/genshin", genshin::routes())
        .nest("/starrail", starrail::routes())
        .nest("/zenless", zenless::routes())
}

#[derive(Serialize)]
struct ApiInfo {
    uptime: String,
    endpoints: Vec<String>,
}

async fn root_handler() -> Json<ApiInfo> {
    let endpoints = vec![
        "/mihoyo/genshin/codes".to_string(),
        "/mihoyo/genshin/news/notices".to_string(),
        "/mihoyo/genshin/news/events".to_string(),
        "/mihoyo/genshin/news/info".to_string(),
        "/mihoyo/starrail/codes".to_string(),
        "/mihoyo/starrail/news/notices".to_string(),
        "/mihoyo/starrail/news/events".to_string(),
        "/mihoyo/starrail/news/info".to_string(),
        "/mihoyo/zenless/codes".to_string(),
        "/mihoyo/zenless/news/notices".to_string(),
        "/mihoyo/zenless/news/events".to_string(),
        "/mihoyo/zenless/news/info".to_string(),
    ];

    Json(ApiInfo {
        uptime: "0".to_string(),
        endpoints,
    })
}