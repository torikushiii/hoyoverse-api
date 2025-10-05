use super::{
    utils::{self, NewsQuery},
    AppState,
};
use crate::error::ApiError;
use crate::types::{CodesResponse, NewsItemResponse};
use axum::{extract::State, http::HeaderMap, response::Json, routing::get, Router};

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
    utils::handle_codes("zenless", state, headers).await
}

#[axum::debug_handler]
async fn news(
    category: axum::extract::Path<String>,
    query: axum::extract::Query<NewsQuery>,
    state: State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<NewsItemResponse>>, ApiError> {
    utils::handle_news("zenless", category, query, state, headers).await
}
