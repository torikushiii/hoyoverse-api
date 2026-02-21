use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::Response;
use axum::routing::get;
use axum::{Json, Router};
use hyper::StatusCode;

use crate::games::Game;
use crate::global::Global;

pub mod calendar;
pub mod codes;

pub(super) fn json_response(bytes: Bytes) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Body::from(bytes))
        .unwrap()
}

pub fn routes(global: &Arc<Global>) -> Router<Arc<Global>> {
    Router::new()
        .route("/", get(root))
        .merge(codes::routes(&global.config.api.rate_limit))
        .merge(calendar::routes())
}

#[derive(serde::Serialize)]
struct RootResponse {
    message: &'static str,
    version: &'static str,
    uptime: u64,
    endpoints: Vec<String>,
}

#[tracing::instrument(skip(global))]
async fn root(State(global): State<Arc<Global>>) -> Json<RootResponse> {
    let games = [
        Game::Genshin,
        Game::Starrail,
        Game::Zenless,
        Game::Honkai,
        Game::Themis,
    ];

    let endpoints: Vec<String> = games
        .iter()
        .map(|g| format!("/mihoyo/{}/codes", g.slug()))
        .collect();

    Json(RootResponse {
        message: "HoYoverse Redemption Code API",
        version: env!("CARGO_PKG_VERSION"),
        uptime: global.started_at.elapsed().as_secs(),
        endpoints,
    })
}
