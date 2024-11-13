use axum::{
    Router,
    routing::get,
    response::Json,
};
use serde::Serialize;
use std::sync::Arc;
use crate::db::DatabaseConnections;
use crate::ratelimit::RateLimiter;
use crate::utils::datetime::get_uptime;

mod genshin;
mod starrail;
mod zenless;
mod themis;

pub type AppState = (Arc<DatabaseConnections>, Arc<RateLimiter>);

pub fn mihoyo_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(root_handler))
        .nest("/genshin", genshin::routes())
        .nest("/starrail", starrail::routes())
        .nest("/zenless", zenless::routes())
        .nest("/themis", themis::routes())
}

#[derive(Serialize)]
struct ApiInfo {
    message: &'static str,
    version: &'static str,
    uptime: i64,
    endpoints: Vec<String>,
}

async fn root_handler() -> Json<ApiInfo> {
    Json(ApiInfo {
        message: "Welcome to the HoYoverse API!",
        version: env!("CARGO_PKG_VERSION"),
        uptime: get_uptime(),
        endpoints: vec![
            "/mihoyo/genshin/codes".to_string(),
            "/mihoyo/genshin/news/{category}".to_string(),
            "/mihoyo/starrail/codes".to_string(),
            "/mihoyo/starrail/news/{category}".to_string(),
            "/mihoyo/zenless/codes".to_string(),
            "/mihoyo/zenless/news/{category}".to_string(),
            "/mihoyo/themis/codes".to_string(),
            "/mihoyo/themis/news/{category}".to_string(),
        ],
    })
}