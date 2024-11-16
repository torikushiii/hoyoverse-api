use axum::{
    Router,
    routing::get,
    extract::State,
    http::HeaderMap,
    response::Json,
};
use crate::types::{CodesResponse, NewsItemResponse};
use super::{AppState, utils::{self, NewsQuery}};
use crate::error::ApiError;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/codes", get(codes))
        .route("/news/:category", get(news))
}

#[axum::debug_handler]
async fn codes(
    state: State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CodesResponse>, ApiError> {
    utils::handle_codes("honkai", state, headers).await
}

#[axum::debug_handler]
async fn news(
    category: axum::extract::Path<String>,
    query: axum::extract::Query<NewsQuery>,
    state: State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<NewsItemResponse>>, ApiError> {
    utils::handle_news("honkai", category, query, state, headers).await
}